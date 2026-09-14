use std::path::{Path, PathBuf};

use axum::extract::State;
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::api::proxy::ProxyConfig;
use crate::api::scraper;
use crate::db::models::{ScrapedWork, Track};
use crate::db::queries;
use crate::state::{AppError, SharedState};

/// 根据数据源抓取作品元数据。
/// source: "asmrone" 强制 asmr.one；"dlsite" / "auto" / None 优先尝试 DLsite，
/// 若 DLsite 抓取失败（如网络被拦截/403/地区限制），自动回退 asmr.one。
fn fetch_metadata(
    rj_code: &str,
    source: Option<&str>,
    token: Option<&str>,
    proxy: &ProxyConfig,
    asmr_api_base: &str,
) -> Result<ScrapedWork, String> {
    match source {
        Some("asmrone") => {
            crate::api::asmrone::fetch_work_from_asmrone(rj_code, token, proxy, asmr_api_base)
                .map_err(|e| format!("asmr.one 抓取失败: {e}"))
        }
        _ => match scraper::fetch_work_metadata(rj_code, proxy) {
            Ok(w) => Ok(w),
            Err(dl_err) => match crate::api::asmrone::fetch_work_from_asmrone(
                rj_code,
                token,
                proxy,
                asmr_api_base,
            ) {
                Ok(w) => Ok(w),
                Err(asmr_err) => Err(format!(
                    "DLsite 抓取失败: {dl_err}；且 asmr.one 回退抓取失败: {asmr_err}"
                )),
            },
        },
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RjSourceBody {
    pub rj_code: String,
    pub source: Option<String>,
}

/// 按 RJ 号抓取元数据并入库，返回保存后的作品。
pub async fn import_by_rj(
    State(state): State<SharedState>,
    Json(body): Json<RjSourceBody>,
) -> Result<Json<serde_json::Value>, AppError> {
    let (token, proxy, asmr_api_base) = {
        let conn = state.db.lock().map_err(AppError::new)?;
        let token = queries::get_setting(&conn, "asmr_one_token")
            .ok()
            .flatten()
            .unwrap_or_default();
        let asmr_api_base = crate::api::asmrone::normalize_api_base(
            queries::get_setting(&conn, "asmr_one_address").ok().flatten().as_deref(),
        );
        (token, ProxyConfig::from_conn(&conn), asmr_api_base)
    };
    let rj = body.rj_code;
    let source = body.source;
    let scraped = tokio::task::spawn_blocking(move || {
        fetch_metadata(&rj, source.as_deref(), Some(&token), &proxy, &asmr_api_base)
            .map_err(AppError::new)
    })
    .await
    .map_err(AppError::new)??;

    let view = tokio::task::spawn_blocking(move || {
        let conn = state.db.lock().map_err(AppError::new)?;
        let work_id = queries::upsert_work(&conn, &scraped).map_err(AppError::new)?;
        queries::set_work_actors(&conn, work_id, &scraped.actors).map_err(AppError::new)?;
        queries::set_work_auto_tags(&conn, work_id, &scraped.tags).map_err(AppError::new)?;
        // 入库自动翻译：开关开启且有日文标题时自动入队
        let mut auto_translated = false;
        if queries::get_setting(&conn, "llm_auto").ok().flatten().is_some_and(|v| v == "1") {
            let has_ja = scraped
                .title_ja
                .as_deref()
                .is_some_and(|t| !t.trim().is_empty());
            if has_ja {
                queries::enqueue_translation(&conn, work_id).map_err(AppError::new)?;
                auto_translated = true;
            }
        }
        queries::get_work(&conn, work_id)
            .map_err(AppError::new)?
            .map(|v| (v, auto_translated))
            .ok_or_else(|| AppError("保存后未找到作品".to_string()))
    })
    .await
    .map_err(AppError::new)??;
    let (view, auto_translated) = view;

    Ok(Json(
        json!({ "ok": true, "work": view, "autoTranslated": auto_translated })
    ))
}

/// 只抓取元数据（不保存），用于导入前预览。
pub async fn preview_by_rj(
    State(state): State<SharedState>,
    Json(body): Json<RjSourceBody>,
) -> Result<Json<ScrapedWork>, AppError> {
    let (token, proxy, asmr_api_base) = {
        let conn = state.db.lock().map_err(AppError::new)?;
        let token = queries::get_setting(&conn, "asmr_one_token")
            .ok()
            .flatten()
            .unwrap_or_default();
        let asmr_api_base = crate::api::asmrone::normalize_api_base(
            queries::get_setting(&conn, "asmr_one_address").ok().flatten().as_deref(),
        );
        (token, ProxyConfig::from_conn(&conn), asmr_api_base)
    };
    let rj = body.rj_code;
    let source = body.source;
    let scraped = tokio::task::spawn_blocking(move || {
        fetch_metadata(&rj, source.as_deref(), Some(&token), &proxy, &asmr_api_base)
            .map_err(AppError::new)
    })
    .await
    .map_err(AppError::new)??;
    Ok(Json(scraped))
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

#[derive(Deserialize)]
pub struct ScanFolderBody {
    pub folder: String,
}

/// 扫描本地文件夹，识别 RJ 号目录。
pub async fn scan_folder(
    State(state): State<SharedState>,
    Json(body): Json<ScanFolderBody>,
) -> Result<Json<serde_json::Value>, AppError> {
    let items = tokio::task::spawn_blocking(move || {
        let root = PathBuf::from(&body.folder);
        if !root.is_dir() {
            return Err(AppError("所选路径不是文件夹".to_string()));
        }
        let mut items: Vec<ScannedItem> = Vec::new();
        collect_rj_dirs(&root, &mut items, 0);

        let conn = state.db.lock().map_err(AppError::new)?;
        for item in &mut items {
            match queries::get_work_by_rj(&conn, &item.rj_code) {
                Ok(Some(w)) => {
                    item.found = true;
                    item.title = w.title_ja;
                }
                _ => item.found = false,
            }
        }
        Ok(items)
    })
    .await
    .map_err(AppError::new)??;

    let count = items.len();
    Ok(Json(json!({ "items": items, "count": count })))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BindFolderBody {
    pub rj_code: String,
    pub path: String,
    pub group_name: Option<String>,
}

/// 把扫描或用户选择的本地文件夹注册到指定 RJ 作品下，并自动扫描音轨入库。
/// 若提供 `group_name`，则自动创建（或复用）对应分组并分配给该作品；
/// 否则自动尝试提取父目录名作为智能分组。
pub async fn bind_local_folder(
    State(state): State<SharedState>,
    Json(body): Json<BindFolderBody>,
) -> Result<Json<serde_json::Value>, AppError> {
    let result = tokio::task::spawn_blocking(
        move || -> Result<serde_json::Value, AppError> {
            let conn = state.db.lock().map_err(AppError::new)?;
            let work = queries::get_work_by_rj(&conn, &body.rj_code)
            .map_err(AppError::new)?
            .ok_or_else(|| AppError(format!("未找到 RJ{} 的作品，请先导入", body.rj_code)))?;
        queries::upsert_local_file(&conn, work.id, &body.path).map_err(AppError::new)?;
        let track_count = scan_audio_tracks_impl(&conn, work.id, &body.path)
            .map_err(AppError::new)?;

        let mut assigned_group: Option<String> = None;
        if let Some(gname) = body.group_name.filter(|s| !s.trim().is_empty()) {
            let gid = queries::create_group(&conn, gname.trim(), "#597ef7")
                .map_err(AppError::new)?;
            queries::set_work_group(&conn, work.id, Some(gid)).map_err(AppError::new)?;
            assigned_group = Some(gname.trim().to_string());
        } else if let Some(pname) = parent_dir_name(&body.path) {
            let upper = pname.to_uppercase();
            let is_rj = upper.starts_with("RJ") && upper[2..].chars().all(|c| c.is_ascii_digit());
            if !is_rj && !pname.ends_with(':') && !pname.trim().is_empty() {
                let gid = queries::create_group(&conn, pname.trim(), "#597ef7")
                    .map_err(AppError::new)?;
                queries::set_work_group(&conn, work.id, Some(gid)).map_err(AppError::new)?;
                assigned_group = Some(pname.trim().to_string());
            }
        }

        Ok(json!({
            "ok": true,
            "trackCount": track_count,
            "groupName": assigned_group,
        }))
    })
    .await
    .map_err(AppError::new)??;
    Ok(Json(result))
}

/// 提取路径的直接父目录名（跨平台，处理尾部分隔符）。
fn parent_dir_name(path: &str) -> Option<String> {
    let trimmed = path.trim_end_matches(['/', '\\']);
    if trimmed.is_empty() {
        return None;
    }
    Path::new(trimmed)
        .parent()
        .and_then(|p| p.file_name())
        .map(|s| s.to_string_lossy().to_string())
}

/// 实际扫描实现：递归收集音频文件，清空并重建该作品的音轨列表。返回音轨数。
pub fn scan_audio_tracks_impl(
    conn: &rusqlite::Connection,
    work_id: i64,
    local_path: &str,
) -> Result<usize, String> {
    let dir = PathBuf::from(local_path);
    let search_dir = if dir.is_file() {
        dir.parent().unwrap_or(&dir).to_path_buf()
    } else {
        dir
    };

    let mut audio_files: Vec<PathBuf> = Vec::new();
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
pub fn collect_audio(dir: &Path, out: &mut Vec<PathBuf>, depth: u32) {
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
            let rest = &upper[i + 2..];
            let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
            if digits.len() >= 4 {
                return Some(format!("RJ{}", digits));
            }
        }
    }
    None
}
