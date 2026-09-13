use crate::db::models::*;
use crate::db::queries;
use crate::AppState;
use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex as StdMutex};
use tauri::{AppHandle, Emitter, Manager, State};

type Db<'a> = State<'a, AppState>;

/// Global download manager shared via app state.
#[derive(Default)]
pub struct DownloadManager {
    pub tasks: StdMutex<HashMap<String, DownloadHandle>>,
}

pub struct DownloadHandle {
    pub task: DownloadTask,
    pub cancel: Arc<AtomicBool>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DownloadProgressEvent {
    pub rj_code: String,
    pub status: String,
    pub progress: f64,
    pub speed: f64,
    pub bytes_done: u64,
    pub bytes_total: u64,
    pub error: Option<String>,
}

const CHUNK: usize = 256 * 1024;

/// Resolve download directories from settings.
fn resolve_dirs(db: &rusqlite::Connection) -> Result<(PathBuf, PathBuf), String> {
    let base = queries::get_setting(db, "download_dir")
        .map_err(|e| e.to_string())?
        .map(PathBuf::from)
        .unwrap_or_else(|| dirs::download_dir().unwrap_or_else(|| PathBuf::from(".")));
    std::fs::create_dir_all(&base).map_err(|e| e.to_string())?;
    let staging = base.join(".staging");
    std::fs::create_dir_all(&staging).map_err(|e| e.to_string())?;
    Ok((base, staging))
}

/// Start downloading a work's archive from a direct URL.
#[tauri::command]
pub fn download_work(
    app: AppHandle,
    db: Db,
    work_id: i64,
    url: String,
) -> Result<(), String> {
    let conn = db.db.lock().map_err(|e| e.to_string())?;
    let view = queries::get_work(&conn, work_id)
        .map_err(|e| e.to_string())?
        .ok_or("作品不存在")?;
    let rj = view.work.rj_code.clone();
    let title = view
        .work
        .title_ja
        .clone()
        .unwrap_or_else(|| rj.clone());
    let (download_dir, _staging) = resolve_dirs(&conn)?;
    drop(conn);

    let manager = app.state::<DownloadManager>();
    let mut tasks = manager.tasks.lock().map_err(|e| e.to_string())?;

    if let Some(h) = tasks.get(&rj) {
        if h.task.status == "downloading" || h.task.status == "queued" {
            return Err(format!("RJ{} 已在下载队列中", rj));
        }
    }

    let handle = DownloadHandle {
        task: DownloadTask {
            rj_code: rj.clone(),
            title,
            status: "downloading".to_string(),
            progress: 0.0,
            speed: 0.0,
            bytes_done: 0,
            bytes_total: 0,
            error: None,
        },
        cancel: Arc::new(AtomicBool::new(false)),
    };
    tasks.insert(rj.clone(), handle);
    drop(tasks);

    let app2 = app.clone();
    let db_path = crate::db::get_db_path(&app).map_err(|e| e.to_string())?;

    std::thread::spawn(move || {
        let result = run_download(&app2, &db_path, &rj, &url, &download_dir);
        let task = result_to_task(result, &rj);
        if let Ok(conn) = rusqlite::Connection::open(&db_path) {
            emit_progress(&app2, &task);
            let _ = conn;
        }
        update_task_state(&app2, &rj, task);
    });

    Ok(())
}

fn result_to_task(res: Result<(), String>, rj: &str) -> DownloadTask {
    match res {
        Ok(()) => DownloadTask {
            rj_code: rj.to_string(),
            title: String::new(),
            status: "done".to_string(),
            progress: 100.0,
            speed: 0.0,
            bytes_done: 0,
            bytes_total: 0,
            error: None,
        },
        Err(e) => DownloadTask {
            rj_code: rj.to_string(),
            title: String::new(),
            status: "error".to_string(),
            progress: 0.0,
            speed: 0.0,
            bytes_done: 0,
            bytes_total: 0,
            error: Some(e),
        },
    }
}

fn emit_progress(app: &AppHandle, t: &DownloadTask) {
    let ev = DownloadProgressEvent {
        rj_code: t.rj_code.clone(),
        status: t.status.clone(),
        progress: t.progress,
        speed: t.speed,
        bytes_done: t.bytes_done,
        bytes_total: t.bytes_total,
        error: t.error.clone(),
    };
    let _ = app.emit("download-progress", ev);
}

fn run_download(
    app: &AppHandle,
    db_path: &PathBuf,
    rj: &str,
    url: &str,
    download_dir: &PathBuf,
) -> Result<(), String> {
    let client = reqwest::blocking::Client::builder()
        .user_agent(
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 Chrome/124.0 Safari/537.36",
        )
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| e.to_string())?;

    // Determine final filename from URL
    let filename = url
        .split('?')
        .next()
        .unwrap_or(url)
        .rsplit('/')
        .next()
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("{}.zip", rj));
    let staging = download_dir.join(".staging");
    let part_path = staging.join(format!("{}.part", filename));
    let final_path = download_dir.join(&filename);

    // Resume support: use existing .part size as Range start
    let mut resume_from: u64 = 0;
    if part_path.exists() {
        resume_from = std::fs::metadata(&part_path)
            .map(|m| m.len())
            .unwrap_or(0);
    }

    let mut req = client.get(url);
    if resume_from > 0 {
        req = req.header("Range", format!("bytes={}-", resume_from));
    }
    let mut resp = req.send().map_err(|e| format!("请求失败: {e}"))?;

    let total: u64 = if resp.status() == reqwest::StatusCode::PARTIAL_CONTENT {
        resp.content_length().unwrap_or(0) + resume_from
    } else if resp.status().is_success() {
        resp.content_length().unwrap_or(0)
    } else {
        return Err(format!("HTTP {}", resp.status()));
    };

    let manager = app.state::<DownloadManager>();
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&part_path)
        .map_err(|e| e.to_string())?;

    use std::io::{Read, Write};
    let mut done: u64 = resume_from;
    let mut last_update = std::time::Instant::now();
    let mut buf = [0u8; CHUNK];

    loop {
        let cancel_flag = manager
            .tasks
            .lock()
            .map_err(|e| e.to_string())?
            .get(rj)
            .map(|h| h.cancel.load(Ordering::SeqCst))
            .unwrap_or(false);
        if cancel_flag {
            return Err("已取消".to_string());
        }
        let n = match resp.read(&mut buf) {
            Ok(n) => n,
            Err(_) => {
                // transient read error — pause and continue
                std::thread::sleep(std::time::Duration::from_millis(500));
                continue;
            }
        };
        if n == 0 {
            break;
        }
        file.write_all(&buf[..n]).map_err(|e| e.to_string())?;
        done += n as u64;
        let elapsed = last_update.elapsed().as_secs_f64();
        if elapsed >= 0.2 {
            let spd = n as f64 / elapsed / 1024.0 / 1024.0;
            last_update = std::time::Instant::now();
            let progress = if total > 0 {
                (done as f64 / total as f64 * 100.0).min(100.0)
            } else {
                0.0
            };
            let task = DownloadTask {
                rj_code: rj.to_string(),
                title: String::new(),
                status: "downloading".to_string(),
                progress,
                speed: spd,
                bytes_done: done,
                bytes_total: total,
                error: None,
            };
            emit_progress(app, &task);
        }
    }
    drop(file);

    // Move .part to final
    std::fs::rename(&part_path, &final_path).map_err(|e| e.to_string())?;

    // Register local file
    if let Ok(conn) = rusqlite::Connection::open(db_path) {
        let work_id = queries::get_work_by_rj(&conn, rj)
            .ok()
            .flatten()
            .map(|w| w.id);
        if let Some(wid) = work_id {
            let _ = queries::upsert_local_file(
                &conn,
                wid,
                final_path.to_string_lossy().as_ref(),
            );
        }
        let _ = conn;
    }

    // Auto-extract if zip
    if final_path.extension().map(|e| e == "zip").unwrap_or(false) {
        if let Err(e) = extract_zip(&final_path, download_dir) {
            return Err(format!("解压失败: {e}"));
        }
        // Scan audio after extraction
        scan_audio_for_work(db_path, rj);
    }

    Ok(())
}

fn extract_zip(zip_path: &PathBuf, dest: &PathBuf) -> Result<(), String> {
    let file = std::fs::File::open(zip_path).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
        let out_path = dest.join(sanitize_rel_path(entry.name()));
        if entry.is_dir() {
            std::fs::create_dir_all(&out_path).map_err(|e| e.to_string())?;
            continue;
        }
        if let Some(parent) = out_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let mut out = std::fs::File::create(&out_path).map_err(|e| e.to_string())?;
        std::io::copy(&mut entry, &mut out).map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn sanitize_rel_path(name: &str) -> PathBuf {
    let p = PathBuf::from(name);
    p.components()
        .filter(|c| !matches!(c, std::path::Component::ParentDir | std::path::Component::RootDir))
        .collect()
}

/// Scan a work's folder for audio files and insert them into audio_tracks.
fn scan_audio_for_work(db_path: &PathBuf, rj: &str) {
    let conn = match rusqlite::Connection::open(db_path) {
        Ok(c) => c,
        Err(_) => return,
    };
    let work = match queries::get_work_by_rj(&conn, rj) {
        Ok(Some(w)) => w,
        _ => return,
    };
    // Find the local path
    let local_dir: Option<String> = conn
        .query_row(
            "SELECT local_path FROM local_files WHERE work_id = ?1 AND download_status = 'downloaded' ORDER BY id DESC LIMIT 1",
            rusqlite::params![work.id],
            |r| r.get(0),
        )
        .ok();
    let Some(local_dir) = local_dir else { return };
    let dir = PathBuf::from(&local_dir);
    // If the local path is a zip, look in the sibling folder
    let search_dir = if dir.is_file() {
        dir.parent().unwrap_or(&dir).to_path_buf()
    } else {
        dir.clone()
    };

    let mut audio_files: Vec<PathBuf> = Vec::new();
    collect_audio(&search_dir, &mut audio_files, 0);
    if audio_files.is_empty() {
        return;
    }
    audio_files.sort();
    let _ = queries::clear_tracks(&conn, work.id);
    for (idx, f) in audio_files.iter().enumerate() {
        let format = f
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        let size = std::fs::metadata(f).map(|m| m.len() as i64).ok();
        let title = f
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        let track = Track {
            work_id: work.id,
            file_path: f.to_string_lossy().to_string(),
            track_number: Some((idx + 1) as i64),
            title: Some(title),
            duration_sec: None,
            file_format: Some(format),
            file_size: size,
            ..Default::default()
        };
        let _ = queries::insert_track(&conn, &track);
    }
}

fn collect_audio(dir: &PathBuf, out: &mut Vec<PathBuf>, depth: u32) {
    if depth > 6 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            collect_audio(&p, out, depth + 1);
        } else if let Some(ext) = p.extension().map(|e| e.to_string_lossy().to_lowercase()) {
            if matches!(ext.as_str(), "mp3" | "flac" | "wav" | "m4a" | "ogg" | "aac") {
                out.push(p);
            }
        }
    }
}

// ============================= asmr.one 多文件下载 =============================

/// 用户在文件树中选中的单个待下载文件。
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AsmrDownloadFile {
    pub title: String,
    pub download_url: String,
    pub relative_path: String,
}

/// 从 asmr.one 下载用户选中的文件列表（后台线程逐个下载，复用 download-progress 事件）。
#[tauri::command]
pub fn download_asmrone_files(
    app: AppHandle,
    db: Db,
    work_id: i64,
    files: Vec<AsmrDownloadFile>,
) -> Result<(), String> {
    if files.is_empty() {
        return Err("未选择任何文件".to_string());
    }
    let conn = db.db.lock().map_err(|e| e.to_string())?;
    let view = queries::get_work(&conn, work_id)
        .map_err(|e| e.to_string())?
        .ok_or("作品不存在")?;
    let rj = view.work.rj_code.clone();
    let (base_dir, _) = resolve_dirs(&conn)?;
    let token = queries::get_setting(&conn, "asmr_one_token")
        .unwrap_or(None)
        .unwrap_or_default();
    drop(conn);

    let work_dir = base_dir.join(&rj);
    std::fs::create_dir_all(&work_dir).map_err(|e| e.to_string())?;

    // 检查是否已有同名下载任务
    let manager = app.state::<DownloadManager>();
    {
        let tasks = manager.tasks.lock().map_err(|e| e.to_string())?;
        if let Some(h) = tasks.get(&rj) {
            if matches!(h.task.status.as_str(), "downloading" | "queued") {
                return Err("该作品已有下载任务运行中".to_string());
            }
        }
    }

    // 注册任务
    let cancel = Arc::new(AtomicBool::new(false));
    {
        let mut tasks = manager.tasks.lock().map_err(|e| e.to_string())?;
        tasks.insert(
            rj.clone(),
            DownloadHandle {
                task: DownloadTask {
                    rj_code: rj.clone(),
                    title: format!("[asmr.one] {} 个文件", files.len()),
                    status: "downloading".to_string(),
                    progress: 0.0,
                    speed: 0.0,
                    bytes_done: 0,
                    bytes_total: 0,
                    error: None,
                },
                cancel: cancel.clone(),
            },
        );
    }

    let db_path = crate::db::get_db_path(&app).map_err(|e| e.to_string())?;
    let app2 = app.clone();

    std::thread::spawn(move || {
        let result = run_asmrone_download(&app2, &rj, &work_dir, &files, &token, &cancel);

        // 注册本地文件目录
        if result.is_ok() {
            if let Ok(conn) = rusqlite::Connection::open(&db_path) {
                let _ = queries::upsert_local_file(&conn, work_id, work_dir.to_string_lossy().as_ref());
            }
            scan_audio_for_work(&db_path, &rj);
        }

        let task = result_to_task(result, &rj);
        emit_progress(&app2, &task);
        update_task_state(&app2, &rj, task);
    });

    Ok(())
}

fn run_asmrone_download(
    app: &AppHandle,
    rj: &str,
    work_dir: &PathBuf,
    files: &[AsmrDownloadFile],
    token: &str,
    cancel: &Arc<AtomicBool>,
) -> Result<(), String> {
    let client = reqwest::blocking::Client::builder()
        .user_agent(crate::api::asmrone::USER_AGENT)
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| e.to_string())?;

    let total_files = files.len();
    // 预估总大小（用于整体进度）
    let mut overall_done: u64 = 0;
    let mut overall_total: u64 = 0;

    for (idx, file) in files.iter().enumerate() {
        if cancel.load(Ordering::SeqCst) {
            return Err("已取消".to_string());
        }

        // 解析实际下载 URL
        let url = if file.download_url.starts_with("http://asmr.localhost/file/") {
            let hash = file.download_url.strip_prefix("http://asmr.localhost/file/").unwrap_or("");
            format!("https://api.asmr-100.com/api/file/{}", hash)
        } else {
            file.download_url.clone()
        };

        // 目标文件路径
        let dest = work_dir.join(&file.relative_path);
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("创建目录失败: {e}"))?;
        }

        // 跳过已存在且大小一致的文件
        if dest.exists() {
            let existing_size = std::fs::metadata(&dest).map(|m| m.len()).unwrap_or(0);
            if existing_size > 0 {
                overall_done += existing_size;
                overall_total += existing_size;
                continue;
            }
        }

        // 发起 HTTP 请求
        let mut req = client.get(&url);
        let needs_token = url.contains("asmr-100.com") || url.contains("asmr-200.com");
        if needs_token && !token.trim().is_empty() {
            req = req.header("Authorization", format!("Bearer {}", token));
        }

        let mut resp = req.send().map_err(|e| format!("下载 {} 失败: {e}", file.title))?;
        if !resp.status().is_success() {
            return Err(format!("下载 {} 返回 HTTP {}", file.title, resp.status()));
        }

        let file_total = resp.content_length().unwrap_or(0);
        overall_total += file_total;

        let mut out = std::fs::File::create(&dest)
            .map_err(|e| format!("创建文件 {} 失败: {e}", file.relative_path))?;

        use std::io::{Read, Write};
        let mut buf = [0u8; CHUNK];
        let mut file_done: u64 = 0;
        let mut last_update = std::time::Instant::now();

        loop {
            if cancel.load(Ordering::SeqCst) {
                return Err("已取消".to_string());
            }
            let n = resp.read(&mut buf).map_err(|e| e.to_string())?;
            if n == 0 {
                break;
            }
            out.write_all(&buf[..n]).map_err(|e| e.to_string())?;
            file_done += n as u64;
            overall_done += n as u64;

            let elapsed = last_update.elapsed().as_secs_f64();
            if elapsed >= 0.3 {
                let spd = n as f64 / elapsed / 1024.0 / 1024.0;
                last_update = std::time::Instant::now();
                let progress = if overall_total > 0 {
                    (overall_done as f64 / overall_total as f64 * 100.0).min(99.0)
                } else {
                    ((idx as f64 + file_done as f64 / file_total.max(1) as f64) / total_files as f64 * 100.0).min(99.0)
                };
                let task = DownloadTask {
                    rj_code: rj.to_string(),
                    title: format!("[{}/{}] {}", idx + 1, total_files, file.title),
                    status: "downloading".to_string(),
                    progress,
                    speed: spd,
                    bytes_done: overall_done,
                    bytes_total: overall_total,
                    error: None,
                };
                emit_progress(app, &task);
            }
        }
    }

    Ok(())
}

// ============================= Queue control =============================

#[tauri::command]
pub fn pause_download(app: AppHandle, rj_code: String) -> Result<(), String> {
    let manager = app.state::<DownloadManager>();
    let mut tasks = manager.tasks.lock().map_err(|e| e.to_string())?;
    if let Some(h) = tasks.get_mut(&rj_code) {
        h.task.status = "paused".to_string();
        let ev = DownloadProgressEvent {
            rj_code: h.task.rj_code.clone(),
            status: "paused".to_string(),
            progress: h.task.progress,
            speed: 0.0,
            bytes_done: h.task.bytes_done,
            bytes_total: h.task.bytes_total,
            error: None,
        };
        let _ = app.emit("download-progress", ev);
    }
    Ok(())
}

#[tauri::command]
pub fn resume_download(app: AppHandle, rj_code: String) -> Result<(), String> {
    let manager = app.state::<DownloadManager>();
    let mut tasks = manager.tasks.lock().map_err(|e| e.to_string())?;
    if let Some(h) = tasks.get_mut(&rj_code) {
        h.task.status = "downloading".to_string();
        let ev = DownloadProgressEvent {
            rj_code: h.task.rj_code.clone(),
            status: "downloading".to_string(),
            progress: h.task.progress,
            speed: 0.0,
            bytes_done: h.task.bytes_done,
            bytes_total: h.task.bytes_total,
            error: None,
        };
        let _ = app.emit("download-progress", ev);
    }
    Ok(())
}

#[tauri::command]
pub fn cancel_download(app: AppHandle, rj_code: String) -> Result<(), String> {
    let manager = app.state::<DownloadManager>();
    let mut tasks = manager.tasks.lock().map_err(|e| e.to_string())?;
    if let Some(h) = tasks.get_mut(&rj_code) {
        h.cancel.store(true, Ordering::SeqCst);
        h.task.status = "cancelled".to_string();
    }
    Ok(())
}

/// Update a task's record in the manager state (called from download thread).
fn update_task_state(app: &AppHandle, rj: &str, task: DownloadTask) {
    let manager = app.state::<DownloadManager>();
    let lock_result = manager.tasks.lock();
    if let Ok(mut guard) = lock_result {
        if let Some(h) = guard.get_mut(rj) {
            h.task = task;
        }
    }
}

#[tauri::command]
pub fn get_download_queue(app: AppHandle) -> Result<Vec<DownloadTask>, String> {
    let manager = app.state::<DownloadManager>();
    let tasks = manager.tasks.lock().map_err(|e| e.to_string())?;
    Ok(tasks.values().map(|h| h.task.clone()).collect())
}

// ============================= Settings =============================

#[tauri::command]
pub fn set_download_settings(db: Db, download_dir: String, concurrency: u32) -> Result<(), String> {
    let conn = db.db.lock().map_err(|e| e.to_string())?;
    queries::set_setting(&conn, "download_dir", &download_dir).map_err(|e| e.to_string())?;
    queries::set_setting(&conn, "download_concurrency", &concurrency.to_string())
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn get_download_settings(db: Db) -> Result<serde_json::Value, String> {
    let conn = db.db.lock().map_err(|e| e.to_string())?;
    let dir = queries::get_setting(&conn, "download_dir")
        .ok()
        .flatten()
        .unwrap_or_else(|| {
            dirs::download_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .to_string_lossy()
                .to_string()
        });
    let concurrency: u32 = queries::get_setting(&conn, "download_concurrency")
        .ok()
        .flatten()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1);
    Ok(serde_json::json!({ "download_dir": dir, "concurrency": concurrency }))
}
