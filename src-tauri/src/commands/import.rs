use crate::api::scraper;
use crate::db::models::ScrapedWork;
use crate::db::queries;
use crate::AppState;
use serde::Serialize;
use std::path::{Path, PathBuf};
use tauri::State;

type Db<'a> = State<'a, AppState>;

/// 读取设置项（宽松处理）。
fn get_setting(conn: &rusqlite::Connection, key: &str) -> String {
    queries::get_setting(conn, key).ok().flatten().unwrap_or_default()
}

/// 根据数据源抓取作品元数据。
/// source: "asmrone" 强制 asmr.one；"dlsite" 或 "auto" 或 None 优先尝试 DLsite，
/// 若 DLsite 抓取失败（如网络被拦截/403/地区限制），会自动智能回退到 asmr.one 抓取。
fn fetch_metadata(
    rj_code: &str,
    source: Option<&str>,
    token: Option<&str>,
) -> Result<ScrapedWork, String> {
    match source {
        Some("asmrone") => crate::api::asmrone::fetch_work_from_asmrone(rj_code, token)
            .map_err(|e| format!("asmr.one 抓取失败: {e}")),
        _ => {
            // 优先尝试 DLsite 官方页面
            match scraper::fetch_work_metadata(rj_code) {
                Ok(w) => Ok(w),
                Err(dl_err) => {
                    // DLsite 失败，自动尝试 asmr.one 数据源兜底
                    match crate::api::asmrone::fetch_work_from_asmrone(rj_code, token) {
                        Ok(w) => Ok(w),
                        Err(asmr_err) => Err(format!(
                            "DLsite 抓取失败: {dl_err}；且 asmr.one 回退抓取失败: {asmr_err}"
                        )),
                    }
                }
            }
        }
    }
}

/// 按 RJ 号抓取元数据并入库，返回保存后的作品。
#[tauri::command]
pub fn import_by_rj(db: Db, rj_code: String, source: Option<String>) -> Result<serde_json::Value, String> {
    let token = {
        let conn = db.db.lock().map_err(|e| e.to_string())?;
        get_setting(&conn, "asmr_one_token")
    };
    let scraped = fetch_metadata(&rj_code, source.as_deref(), Some(&token))?;

    let conn = db.db.lock().map_err(|e| e.to_string())?;
    let work_id = queries::upsert_work(&conn, &scraped).map_err(|e| e.to_string())?;
    queries::set_work_actors(&conn, work_id, &scraped.actors).map_err(|e| e.to_string())?;
    queries::set_work_auto_tags(&conn, work_id, &scraped.tags).map_err(|e| e.to_string())?;

    let view = queries::get_work(&conn, work_id)
        .map_err(|e| e.to_string())?
        .ok_or("保存后未找到作品")?;
    Ok(serde_json::json!({ "ok": true, "work": view }))
}

/// 只抓取元数据（不保存），用于导入前预览。
#[tauri::command]
pub fn preview_by_rj(db: Db, rj_code: String, source: Option<String>) -> Result<ScrapedWork, String> {
    let token = {
        let conn = db.db.lock().map_err(|e| e.to_string())?;
        get_setting(&conn, "asmr_one_token")
    };
    fetch_metadata(&rj_code, source.as_deref(), Some(&token))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScannedItem {
    pub rj_code: String,
    pub path: String,
    pub found: bool,
    pub title: Option<String>,
    /// RJ 目录的直接父目录名（若不是 RJ 号则为分类分组，如「耳かき」）
    pub group_name: Option<String>,
}

/// 扫描本地文件夹，识别 RJ 号目录。
#[tauri::command]
pub fn scan_folder(
    db: Db,
    folder: String,
    r#async: bool,
) -> Result<serde_json::Value, String> {
    let _ = r#async;
    let root = PathBuf::from(&folder);
    if !root.is_dir() {
        return Err("所选路径不是文件夹".to_string());
    }

    let mut items: Vec<ScannedItem> = Vec::new();
    collect_rj_dirs(&root, &mut items, 0);

    let conn = db.db.lock().map_err(|e| e.to_string())?;
    for item in &mut items {
        match queries::get_work_by_rj(&conn, &item.rj_code) {
            Ok(Some(w)) => {
                item.found = true;
                item.title = w.title_ja;
            }
            _ => {
                item.found = false;
            }
        }
    }
    Ok(serde_json::json!({ "items": items, "count": items.len() }))
}

/// 把扫描或用户选择的本地文件夹注册到指定 RJ 作品下，并自动扫描音轨入库。
/// 若提供 `group_name`，则自动创建（或复用）对应分组并分配给该作品。
/// 若未提供 `group_name`，则自动尝试提取父目录名作为智能分组。
#[tauri::command]
pub fn bind_local_folder(
    db: Db,
    rj_code: String,
    path: String,
    group_name: Option<String>,
) -> Result<serde_json::Value, String> {
    let conn = db.db.lock().map_err(|e| e.to_string())?;
    let work = queries::get_work_by_rj(&conn, &rj_code)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("未找到 RJ{} 的作品，请先导入", rj_code))?;
    queries::upsert_local_file(&conn, work.id, &path).map_err(|e| e.to_string())?;
    let track_count = scan_audio_tracks_impl(&conn, work.id, &path).map_err(|e| e.to_string())?;

    let mut assigned_group: Option<String> = None;
    if let Some(gname) = group_name.filter(|s| !s.trim().is_empty()) {
        let gid = queries::create_group(&conn, gname.trim(), "#597ef7").map_err(|e| e.to_string())?;
        queries::set_work_group(&conn, work.id, Some(gid)).map_err(|e| e.to_string())?;
        assigned_group = Some(gname.trim().to_string());
    } else {
        // 自动提取父目录作为智能分组
        let p = std::path::PathBuf::from(&path);
        if let Some(parent) = p.parent() {
            if let Some(pname) = parent.file_name().and_then(|s| s.to_str()) {
                let upper = pname.to_uppercase();
                let is_rj = upper.starts_with("RJ") && upper[2..].chars().all(|c| c.is_ascii_digit());
                if !is_rj && !pname.ends_with(':') && !pname.trim().is_empty() {
                    let gid = queries::create_group(&conn, pname.trim(), "#597ef7").map_err(|e| e.to_string())?;
                    queries::set_work_group(&conn, work.id, Some(gid)).map_err(|e| e.to_string())?;
                    assigned_group = Some(pname.trim().to_string());
                }
            }
        }
    }

    Ok(serde_json::json!({
        "ok": true,
        "trackCount": track_count,
        "groupName": assigned_group,
    }))
}

/// 扫描作品本地文件夹下的音频文件（mp3/wav/flac 等），写入 audio_tracks 表。
#[tauri::command]
pub fn scan_audio_tracks(db: Db, work_id: i64) -> Result<serde_json::Value, String> {
    let conn = db.db.lock().map_err(|e| e.to_string())?;
    let local_path = conn
        .query_row(
            "SELECT local_path FROM local_files WHERE work_id = ?1 AND download_status = 'downloaded' ORDER BY id DESC LIMIT 1",
            rusqlite::params![work_id],
            |r| r.get::<_, String>(0),
        )
        .ok();
    let Some(local_path) = local_path else {
        return Ok(serde_json::json!({ "ok": false, "count": 0, "message": "该作品没有已下载的本地文件夹" }));
    };
    let n = scan_audio_tracks_impl(&conn, work_id, &local_path).map_err(|e| e.to_string())?;
    Ok(serde_json::json!({ "ok": true, "count": n }))
}

/// 实际扫描实现：递归收集音频文件，清空并重建该作品的音轨列表。返回音轨数。
fn scan_audio_tracks_impl(
    conn: &rusqlite::Connection,
    work_id: i64,
    local_path: &str,
) -> Result<usize, String> {
    use crate::db::models::Track;

    let dir = std::path::PathBuf::from(local_path);
    let search_dir = if dir.is_file() {
        dir.parent().unwrap_or(&dir).to_path_buf()
    } else {
        dir
    };

    let mut audio_files: Vec<std::path::PathBuf> = Vec::new();
    collect_audio(&search_dir, &mut audio_files, 0);
    audio_files.sort();
    if audio_files.is_empty() {
        queries::clear_tracks(conn, work_id).map_err(|e| e.to_string())?;
        return Ok(0);
    }

    queries::clear_tracks(conn, work_id).map_err(|e| e.to_string())?;
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
            work_id,
            file_path: f.to_string_lossy().to_string(),
            track_number: Some((idx + 1) as i64),
            title: Some(title),
            duration_sec: None,
            file_format: Some(format),
            file_size: size,
            ..Default::default()
        };
        queries::insert_track(conn, &track).map_err(|e| e.to_string())?;
    }
    Ok(audio_files.len())
}

/// 递归收集音频文件（支持 mp3/wav/flac/m4a/ogg/aac）。
fn collect_audio(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>, depth: u32) {
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
            if matches!(ext.as_str(), "mp3" | "wav" | "flac" | "m4a" | "ogg" | "aac") {
                out.push(p);
            }
        }
    }
}

fn collect_rj_dirs(dir: &Path, out: &mut Vec<ScannedItem>, depth: u32) {
    if depth > 4 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if let Some(rj) = extract_rj_code(&name) {
            // 直接父目录名（若非 RJ 号则作为分类分组）
            let parent_name = path
                .parent()
                .and_then(|p| p.file_name())
                .map(|s| s.to_string_lossy().to_string());
            let group_name = match parent_name {
                Some(p) if extract_rj_code(&p).is_none() && !p.trim().is_empty() => Some(p),
                _ => None,
            };
            out.push(ScannedItem {
                rj_code: rj,
                path: path.to_string_lossy().to_string(),
                found: false,
                title: None,
                group_name,
            });
        }
        collect_rj_dirs(&path, out, depth + 1);
    }
}

/// 从文件夹/文件名中提取 RJ 号（如 "RJ01014447" 或 "RJ01014447xxx"）。
pub fn extract_rj_code(name: &str) -> Option<String> {
    let upper = name.to_uppercase();
    let b = upper.as_bytes();
    for (i, w) in b.windows(2).enumerate() {
        if w == b"RJ" {
            // Following chars must be digits
            let rest = &upper[i + 2..];
            let digits: String = rest
                .chars()
                .take_while(|c| c.is_ascii_digit())
                .collect();
            if digits.len() >= 4 {
                return Some(format!("RJ{}", digits));
            }
        }
    }
    None
}
