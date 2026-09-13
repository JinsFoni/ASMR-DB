use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Instant;

use axum::extract::State;
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::db::models::{DownloadTask, Track};
use crate::db::queries;
use crate::state::{AppError, DownloadHandle, DownloadProgressEvent, SharedState};

const CHUNK: usize = 256 * 1024;

/// 从设置解析下载目录与暂存目录。
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

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadBody {
    pub work_id: i64,
    pub url: String,
}

/// 开始从直链下载作品压缩包。
pub async fn download_work(
    State(state): State<SharedState>,
    Json(body): Json<DownloadBody>,
) -> Result<Json<serde_json::Value>, AppError> {
    let (rj, title, download_dir) = {
        let conn = state.db.lock().map_err(AppError::new)?;
        let view = queries::get_work(&conn, body.work_id)
            .map_err(AppError::new)?
            .ok_or_else(|| AppError("作品不存在".to_string()))?;
        let rj = view.work.rj_code.clone();
        let title = view.work.title_ja.clone().unwrap_or_else(|| rj.clone());
        let (download_dir, _) = resolve_dirs(&conn).map_err(AppError::new)?;
        (rj, title, download_dir)
    };

    if state.manager.is_busy(&rj) {
        return Err(AppError(format!("{rj} 已在下载队列中")));
    }

    state.manager.tasks.lock().map_err(AppError::new)?.insert(
        rj.clone(),
        DownloadHandle {
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
            cancel: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        },
    );

    let state2 = state.clone();
    let db_path = state.db_path.clone();
    let rj2 = rj.clone();
    let url = body.url;

    std::thread::spawn(move || {
        let result = run_download(&state2, &rj2, &url, &download_dir);
        let task = match result {
            Ok(()) => DownloadTask {
                rj_code: rj2.clone(),
                title: String::new(),
                status: "done".to_string(),
                progress: 100.0,
                speed: 0.0,
                bytes_done: 0,
                bytes_total: 0,
                error: None,
            },
            Err(e) => DownloadTask {
                rj_code: rj2.clone(),
                title: String::new(),
                status: "error".to_string(),
                progress: 0.0,
                speed: 0.0,
                bytes_done: 0,
                bytes_total: 0,
                error: Some(e),
            },
        };
        emit(&state2, &task);
        if let Ok(mut tasks) = state2.manager.tasks.lock() {
            if let Some(h) = tasks.get_mut(&rj2) {
                h.task = task;
            }
        }
        let _ = db_path;
    });

    Ok(Json(json!({ "ok": true })))
}

fn emit(state: &SharedState, t: &DownloadTask) {
    state.manager.emit(DownloadProgressEvent {
        rj_code: t.rj_code.clone(),
        status: t.status.clone(),
        progress: t.progress,
        speed: t.speed,
        bytes_done: t.bytes_done,
        bytes_total: t.bytes_total,
        error: t.error.clone(),
    });
}

fn run_download(
    state: &SharedState,
    rj: &str,
    url: &str,
    download_dir: &PathBuf,
) -> Result<(), String> {
    let proxy = {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        crate::api::proxy::ProxyConfig::from_conn(&conn)
    };
    let builder = reqwest::blocking::Client::builder()
        .user_agent(
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 Chrome/124.0 Safari/537.36",
        )
        .timeout(std::time::Duration::from_secs(60));
    let client = proxy
        .apply_blocking(builder, crate::api::proxy::ProxyTarget::Dlsite)
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
    let resume_from: u64 = part_path
        .metadata()
        .map(|m| m.len())
        .unwrap_or(0);

    let mut req = client.get(url);
    if resume_from > 0 {
        req = req.header("Range", format!("bytes={resume_from}-"));
    }
    let mut resp = req.send().map_err(|e| format!("请求失败: {e}"))?;

    let total: u64 = if resp.status() == reqwest::StatusCode::PARTIAL_CONTENT {
        resp.content_length().unwrap_or(0) + resume_from
    } else if resp.status().is_success() {
        resp.content_length().unwrap_or(0)
    } else {
        return Err(format!("HTTP {}", resp.status()));
    };

    use std::io::{Read, Write};
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&part_path)
        .map_err(|e| e.to_string())?;

    let mut done: u64 = resume_from;
    let mut last_update = Instant::now();
    let mut buf = [0u8; CHUNK];

    loop {
        if state.manager.check_cancel(rj) {
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
            last_update = Instant::now();
            let progress = if total > 0 {
                (done as f64 / total as f64 * 100.0).min(100.0)
            } else {
                0.0
            };
            emit(
                state,
                &DownloadTask {
                    rj_code: rj.to_string(),
                    title: String::new(),
                    status: "downloading".to_string(),
                    progress,
                    speed: spd,
                    bytes_done: done,
                    bytes_total: total,
                    error: None,
                },
            );
        }
    }
    drop(file);

    // Move .part to final
    std::fs::rename(&part_path, &final_path).map_err(|e| e.to_string())?;

    // Register local file
    if let Ok(conn) = rusqlite::Connection::open(&state.db_path) {
        if let Ok(Some(w)) = queries::get_work_by_rj(&conn, rj) {
            let _ = queries::upsert_local_file(&conn, w.id, final_path.to_string_lossy().as_ref());
        }
    }

    // Auto-extract if zip
    if final_path.extension().map(|e| e == "zip").unwrap_or(false) {
        if let Err(e) = extract_zip(&final_path, download_dir) {
            return Err(format!("解压失败: {e}"));
        }
        // Scan audio after extraction
        scan_audio_for_work(&state.db_path, rj);
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

/// 扫描作品的本地目录并重建音轨列表（下载/解压完成后调用）。
fn scan_audio_for_work(db_path: &PathBuf, rj: &str) {
    let conn = match rusqlite::Connection::open(db_path) {
        Ok(c) => c,
        Err(_) => return,
    };
    let work = match queries::get_work_by_rj(&conn, rj) {
        Ok(Some(w)) => w,
        _ => return,
    };
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
        dir
    };

    let mut audio_files: Vec<PathBuf> = Vec::new();
    super::import::collect_audio(&search_dir, &mut audio_files, 0);
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

// ============================= asmr.one 多文件下载 =============================

/// 用户在文件树中选中的单个待下载文件。
#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AsmrDownloadFile {
    pub title: String,
    pub download_url: String,
    pub relative_path: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AsmrDownloadBody {
    pub work_id: i64,
    pub files: Vec<AsmrDownloadFile>,
}

/// 从 asmr.one 下载用户选中的文件列表（后台线程逐个下载，复用 download-progress 事件）。
pub async fn download_asmr(
    State(state): State<SharedState>,
    Json(body): Json<AsmrDownloadBody>,
) -> Result<Json<serde_json::Value>, AppError> {
    if body.files.is_empty() {
        return Err(AppError("未选择任何文件".to_string()));
    }
    let (rj, work_dir, token) = {
        let conn = state.db.lock().map_err(AppError::new)?;
        let view = queries::get_work(&conn, body.work_id)
            .map_err(AppError::new)?
            .ok_or_else(|| AppError("作品不存在".to_string()))?;
        let rj = view.work.rj_code.clone();
        let (base_dir, _) = resolve_dirs(&conn).map_err(AppError::new)?;
        let token = queries::get_setting(&conn, "asmr_one_token")
            .ok()
            .flatten()
            .unwrap_or_default();
        let work_dir = base_dir.join(&rj);
        (rj, work_dir, token)
    };
    std::fs::create_dir_all(&work_dir).map_err(AppError::new)?;

    if state.manager.is_busy(&rj) {
        return Err(AppError("该作品已有下载任务运行中".to_string()));
    }

    let cancel = Arc::new(std::sync::atomic::AtomicBool::new(false));
    state.manager.tasks.lock().map_err(AppError::new)?.insert(
        rj.clone(),
        DownloadHandle {
            task: DownloadTask {
                rj_code: rj.clone(),
                title: format!("[asmr.one] {} 个文件", body.files.len()),
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

    let state2 = state.clone();
    let files = body.files;

    std::thread::spawn(move || {
        let result = run_asmrone_download(&state2, &rj, &work_dir, &files, &token, &cancel);

        if result.is_ok() {
            if let Ok(conn) = rusqlite::Connection::open(&state2.db_path) {
                let _ = queries::upsert_local_file(
                    &conn,
                    body.work_id,
                    work_dir.to_string_lossy().as_ref(),
                );
            }
            scan_audio_for_work(&state2.db_path, &rj);
        }

        let task = match result {
            Ok(()) => DownloadTask {
                rj_code: rj.clone(),
                title: String::new(),
                status: "done".to_string(),
                progress: 100.0,
                speed: 0.0,
                bytes_done: 0,
                bytes_total: 0,
                error: None,
            },
            Err(e) => DownloadTask {
                rj_code: rj.clone(),
                title: String::new(),
                status: "error".to_string(),
                progress: 0.0,
                speed: 0.0,
                bytes_done: 0,
                bytes_total: 0,
                error: Some(e),
            },
        };
        emit(&state2, &task);
        if let Ok(mut tasks) = state2.manager.tasks.lock() {
            if let Some(h) = tasks.get_mut(&rj) {
                h.task = task;
            }
        }
    });

    Ok(Json(json!({ "ok": true })))
}

fn run_asmrone_download(
    state: &SharedState,
    rj: &str,
    work_dir: &PathBuf,
    files: &[AsmrDownloadFile],
    token: &str,
    cancel: &Arc<std::sync::atomic::AtomicBool>,
) -> Result<(), String> {
    let proxy = {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        crate::api::proxy::ProxyConfig::from_conn(&conn)
    };
    let builder = reqwest::blocking::Client::builder()
        .user_agent(crate::api::asmrone::USER_AGENT)
        .timeout(std::time::Duration::from_secs(120));
    let client = proxy
        .apply_blocking(builder, crate::api::proxy::ProxyTarget::AsmrOne)
        .build()
        .map_err(|e| e.to_string())?;

    let total_files = files.len();
    let mut overall_done: u64 = 0;
    let mut overall_total: u64 = 0;

    for (idx, file) in files.iter().enumerate() {
        if cancel.load(Ordering::SeqCst) {
            return Err("已取消".to_string());
        }

        // 解析实际下载 URL：本服务代理路径 → asmr.one 文件流端点
        let url = {
            let hash = file
                .download_url
                .strip_prefix("http://asmr.localhost/file/")
                .or_else(|| file.download_url.strip_prefix("/api/asmr/file/"));
            match hash {
                Some(h) => format!("https://api.asmr-100.com/api/file/{h}"),
                None => file.download_url.clone(),
            }
        };

        // 目标文件路径
        let dest = work_dir.join(&file.relative_path);
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("创建目录失败: {e}"))?;
        }

        // 跳过已存在且非空的文件
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
        let needs_token =
            url.contains("asmr-100.com") || url.contains("asmr-200.com");
        if needs_token && !token.trim().is_empty() {
            req = req.header("Authorization", format!("Bearer {token}"));
        }

        let mut resp = resp_check(req.send(), &file.title)?;

        let file_total = resp.content_length().unwrap_or(0);
        overall_total += file_total;

        let mut out =
            std::fs::File::create(&dest).map_err(|e| format!("创建文件 {} 失败: {e}", file.relative_path))?;

        use std::io::{Read, Write};
        let mut buf = [0u8; CHUNK];
        let mut file_done: u64 = 0;
        let mut last_update = Instant::now();

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
                last_update = Instant::now();
                let progress = if overall_total > 0 {
                    (overall_done as f64 / overall_total as f64 * 100.0).min(99.0)
                } else {
                    ((idx as f64 + file_done as f64 / file_total.max(1) as f64)
                        / total_files as f64
                        * 100.0)
                        .min(99.0)
                };
                emit(
                    state,
                    &DownloadTask {
                        rj_code: rj.to_string(),
                        title: format!("[{}/{}] {}", idx + 1, total_files, file.title),
                        status: "downloading".to_string(),
                        progress,
                        speed: spd,
                        bytes_done: overall_done,
                        bytes_total: overall_total,
                        error: None,
                    },
                );
            }
        }
    }

    Ok(())
}

fn resp_check(
    r: Result<reqwest::blocking::Response, reqwest::Error>,
    title: &str,
) -> Result<reqwest::blocking::Response, String> {
    let resp = r.map_err(|e| format!("下载 {title} 失败: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("下载 {title} 返回 HTTP {}", resp.status()));
    }
    Ok(resp)
}

// ============================= 队列控制 =============================

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RjBody {
    pub rj_code: String,
}

pub async fn pause_download(
    State(state): State<SharedState>,
    Json(body): Json<RjBody>,
) -> Result<Json<serde_json::Value>, AppError> {
    if let Ok(mut tasks) = state.manager.tasks.lock() {
        if let Some(h) = tasks.get_mut(&body.rj_code) {
            h.task.status = "paused".to_string();
            emit(&state, &h.task);
        }
    }
    Ok(Json(json!({ "ok": true })))
}

pub async fn resume_download(
    State(state): State<SharedState>,
    Json(body): Json<RjBody>,
) -> Result<Json<serde_json::Value>, AppError> {
    if let Ok(mut tasks) = state.manager.tasks.lock() {
        if let Some(h) = tasks.get_mut(&body.rj_code) {
            h.task.status = "downloading".to_string();
            emit(&state, &h.task);
        }
    }
    Ok(Json(json!({ "ok": true })))
}

pub async fn cancel_download(
    State(state): State<SharedState>,
    Json(body): Json<RjBody>,
) -> Result<Json<serde_json::Value>, AppError> {
    if let Ok(mut tasks) = state.manager.tasks.lock() {
        if let Some(h) = tasks.get_mut(&body.rj_code) {
            h.cancel.store(true, Ordering::SeqCst);
            h.task.status = "cancelled".to_string();
        }
    }
    Ok(Json(json!({ "ok": true })))
}

#[derive(Serialize)]
pub struct QueueResponse {
    pub tasks: Vec<DownloadTask>,
}

pub async fn get_queue(
    State(state): State<SharedState>,
) -> Result<Json<Vec<DownloadTask>>, AppError> {
    let tasks = state
        .manager
        .tasks
        .lock()
        .map_err(AppError::new)?
        .values()
        .map(|h| h.task.clone())
        .collect();
    Ok(Json(tasks))
}

// ============================= 下载设置 =============================

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadSettingsBody {
    pub download_dir: String,
    pub concurrency: u32,
}

pub async fn set_settings(
    State(state): State<SharedState>,
    Json(body): Json<DownloadSettingsBody>,
) -> Result<Json<serde_json::Value>, AppError> {
    tokio::task::spawn_blocking(move || {
        let conn = state.db.lock().map_err(AppError::new)?;
        queries::set_setting(&conn, "download_dir", &body.download_dir).map_err(AppError::new)?;
        queries::set_setting(&conn, "download_concurrency", &body.concurrency.to_string())
            .map_err(AppError::new)
    })
    .await
    .map_err(AppError::new)??;
    Ok(Json(json!({ "ok": true })))
}

#[derive(Serialize)]
pub struct DownloadSettings {
    pub download_dir: String,
    pub concurrency: u32,
}

pub async fn get_settings(
    State(state): State<SharedState>,
) -> Result<Json<DownloadSettings>, AppError> {
    let s = tokio::task::spawn_blocking(move || -> Result<DownloadSettings, AppError> {
        let conn = state.db.lock().map_err(AppError::new)?;
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
        Ok(DownloadSettings {
            download_dir: dir,
            concurrency,
        })
    })
    .await
    .map_err(AppError::new)??;
    Ok(Json(s))
}
