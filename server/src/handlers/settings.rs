use std::path::{Path, PathBuf};

use axum::body::Body;
use axum::extract::{Multipart, State};
use axum::http::{header, StatusCode};
use axum::response::IntoResponse;
use axum::Json;
use serde::Deserialize;
use serde_json::json;

use crate::db::queries;
use crate::state::{AppError, SharedState};

// ============================= 设置读写 =============================

#[derive(serde::Serialize)]
pub struct SettingsResponse {
    pub download_dir: Option<String>,
    pub theme: String,
    pub asmr_one_token: Option<String>,
    /// 出站代理地址（如 http://127.0.0.1:8502），空 = 未配置
    pub proxy_url: Option<String>,
    /// DLsite 请求是否走代理
    pub proxy_dlsite: bool,
    /// asmr.one 请求是否走代理
    pub proxy_asmrone: bool,
    /// asmr.one 站点/API 地址（空 = 默认 api.asmr.one）
    pub asmr_one_address: Option<String>,
    pub asmr_one_username: Option<String>,
    pub asmr_one_password: Option<String>,
    /// LLM 请求是否走代理
    pub proxy_llm: bool,
    /// LLM 翻译配置
    pub llm_endpoint: Option<String>,
    pub llm_api_key: Option<String>,
    pub llm_model: Option<String>,
    pub llm_max_retry: i64,
    pub llm_rps: i64,
    pub llm_threads: i64,
    pub llm_auto: bool,
    pub llm_prompt: Option<String>,
}

pub async fn get_settings(
    State(state): State<SharedState>,
) -> Result<Json<SettingsResponse>, AppError> {
    let s = tokio::task::spawn_blocking(move || -> Result<SettingsResponse, AppError> {
        let conn = state.db.lock().map_err(AppError::new)?;
        Ok(SettingsResponse {
            download_dir: queries::get_setting(&conn, "download_dir").ok().flatten(),
            theme: queries::get_setting(&conn, "theme")
                .ok()
                .flatten()
                .unwrap_or_else(|| "dark".to_string()),
            asmr_one_token: queries::get_setting(&conn, "asmr_one_token")
                .ok()
                .flatten(),
            proxy_url: queries::get_setting(&conn, "proxy_url").ok().flatten(),
            proxy_dlsite: queries::get_setting(&conn, "proxy_dlsite")
                .ok()
                .flatten()
                .is_some_and(|v| v == "1"),
            proxy_asmrone: queries::get_setting(&conn, "proxy_asmrone")
                .ok()
                .flatten()
                .is_some_and(|v| v == "1"),
            asmr_one_address: queries::get_setting(&conn, "asmr_one_address").ok().flatten(),
            asmr_one_username: queries::get_setting(&conn, "asmr_one_username").ok().flatten(),
            asmr_one_password: queries::get_setting(&conn, "asmr_one_password").ok().flatten(),
            proxy_llm: queries::get_setting(&conn, "proxy_llm")
                .ok()
                .flatten()
                .is_some_and(|v| v == "1"),
            llm_endpoint: queries::get_setting(&conn, "llm_endpoint").ok().flatten(),
            llm_api_key: queries::get_setting(&conn, "llm_api_key").ok().flatten(),
            llm_model: queries::get_setting(&conn, "llm_model").ok().flatten(),
            llm_max_retry: queries::get_setting(&conn, "llm_max_retry")
                .ok()
                .flatten()
                .and_then(|v| v.parse().ok())
                .unwrap_or(3),
            llm_rps: queries::get_setting(&conn, "llm_rps")
                .ok()
                .flatten()
                .and_then(|v| v.parse().ok())
                .unwrap_or(2),
            llm_threads: queries::get_setting(&conn, "llm_threads")
                .ok()
                .flatten()
                .and_then(|v| v.parse().ok())
                .unwrap_or(1),
            llm_auto: queries::get_setting(&conn, "llm_auto")
                .ok()
                .flatten()
                .is_some_and(|v| v == "1"),
            llm_prompt: queries::get_setting(&conn, "llm_prompt").ok().flatten(),
        })
    })
    .await
    .map_err(AppError::new)??;
    Ok(Json(s))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetSettingsBody {
    pub theme: Option<String>,
    pub download_dir: Option<String>,
    pub proxy_url: Option<String>,
    pub proxy_dlsite: Option<bool>,
    pub proxy_asmrone: Option<bool>,
    pub proxy_llm: Option<bool>,
    pub asmr_one_address: Option<String>,
    pub asmr_one_username: Option<String>,
    pub asmr_one_password: Option<String>,
    pub llm_endpoint: Option<String>,
    pub llm_api_key: Option<String>,
    pub llm_model: Option<String>,
    pub llm_max_retry: Option<i64>,
    pub llm_rps: Option<i64>,
    pub llm_threads: Option<i64>,
    pub llm_auto: Option<bool>,
    pub llm_prompt: Option<String>,
}

pub async fn set_settings(
    State(state): State<SharedState>,
    Json(body): Json<SetSettingsBody>,
) -> Result<Json<serde_json::Value>, AppError> {
    tokio::task::spawn_blocking(move || -> Result<(), AppError> {
        let conn = state.db.lock().map_err(AppError::new)?;
        if let Some(t) = body.theme {
            queries::set_setting(&conn, "theme", &t).map_err(AppError::new)?;
        }
        if let Some(d) = body.download_dir {
            queries::set_setting(&conn, "download_dir", &d).map_err(AppError::new)?;
        }
        if let Some(u) = body.proxy_url {
            queries::set_setting(&conn, "proxy_url", u.trim()).map_err(AppError::new)?;
        }
        if let Some(b) = body.proxy_dlsite {
            queries::set_setting(&conn, "proxy_dlsite", if b { "1" } else { "0" })
                .map_err(AppError::new)?;
        }
        if let Some(b) = body.proxy_asmrone {
            queries::set_setting(&conn, "proxy_asmrone", if b { "1" } else { "0" })
                .map_err(AppError::new)?;
        }
        if let Some(a) = body.asmr_one_address {
            queries::set_setting(&conn, "asmr_one_address", a.trim()).map_err(AppError::new)?;
        }
        if let Some(u) = body.asmr_one_username {
            queries::set_setting(&conn, "asmr_one_username", u.trim()).map_err(AppError::new)?;
        }
        if let Some(p) = body.asmr_one_password {
            queries::set_setting(&conn, "asmr_one_password", p.as_str()).map_err(AppError::new)?;
        }
        if let Some(b) = body.proxy_llm {
            queries::set_setting(&conn, "proxy_llm", if b { "1" } else { "0" }).map_err(AppError::new)?;
        }
        if let Some(v) = body.llm_endpoint {
            queries::set_setting(&conn, "llm_endpoint", v.trim()).map_err(AppError::new)?;
        }
        if let Some(v) = body.llm_api_key {
            queries::set_setting(&conn, "llm_api_key", v.trim()).map_err(AppError::new)?;
        }
        if let Some(v) = body.llm_model {
            queries::set_setting(&conn, "llm_model", v.trim()).map_err(AppError::new)?;
        }
        if let Some(v) = body.llm_max_retry {
            queries::set_setting(&conn, "llm_max_retry", &v.clamp(0, 10).to_string())
                .map_err(AppError::new)?;
        }
        if let Some(v) = body.llm_rps {
            queries::set_setting(&conn, "llm_rps", &v.clamp(1, 100).to_string())
                .map_err(AppError::new)?;
        }
        if let Some(v) = body.llm_threads {
            queries::set_setting(&conn, "llm_threads", &v.clamp(1, 4).to_string())
                .map_err(AppError::new)?;
        }
        if let Some(b) = body.llm_auto {
            queries::set_setting(&conn, "llm_auto", if b { "1" } else { "0" }).map_err(AppError::new)?;
        }
        if let Some(v) = body.llm_prompt {
            queries::set_setting(&conn, "llm_prompt", v.trim()).map_err(AppError::new)?;
        }
        Ok(())
    })
    .await
    .map_err(AppError::new)??;
    Ok(Json(json!({ "ok": true })))
}

#[derive(Deserialize)]
pub struct TokenBody {
    pub token: String,
}

pub async fn set_asmr_token(
    State(state): State<SharedState>,
    Json(body): Json<TokenBody>,
) -> Result<Json<serde_json::Value>, AppError> {
    tokio::task::spawn_blocking(move || {
        let conn = state.db.lock().map_err(AppError::new)?;
        queries::set_setting(&conn, "asmr_one_token", &body.token).map_err(AppError::new)
    })
    .await
    .map_err(AppError::new)??;
    Ok(Json(json!({ "ok": true })))
}

pub async fn clear_asmr_token(
    State(state): State<SharedState>,
) -> Result<Json<serde_json::Value>, AppError> {
    tokio::task::spawn_blocking(move || {
        let conn = state.db.lock().map_err(AppError::new)?;
        queries::set_setting(&conn, "asmr_one_token", "").map_err(AppError::new)
    })
    .await
    .map_err(AppError::new)??;
    Ok(Json(json!({ "ok": true })))
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AsmrLoginBody {
    /// 不传时使用设置里已保存的账号密码
    pub username: Option<String>,
    pub password: Option<String>,
}

/// 使用账号密码登录 asmr.one：成功后把返回的 token 存入设置（替代手动粘贴 Token）。
pub async fn asmr_login(
    State(state): State<SharedState>,
    body: Option<Json<AsmrLoginBody>>,
) -> Result<Json<serde_json::Value>, AppError> {
    let body = body.map(|Json(b)| b).unwrap_or_default();
    let (address, saved_username, saved_password, proxy) = {
        let conn = state.db.lock().map_err(AppError::new)?;
        (
            queries::get_setting(&conn, "asmr_one_address").ok().flatten(),
            queries::get_setting(&conn, "asmr_one_username").ok().flatten(),
            queries::get_setting(&conn, "asmr_one_password").ok().flatten(),
            crate::api::proxy::ProxyConfig::from_conn(&conn),
        )
    };
    // 请求体优先，其次设置里保存的
    let provided_password = body.password.clone();
    let username = body
        .username
        .or(saved_username)
        .unwrap_or_default()
        .trim()
        .to_string();
    let password = provided_password.clone().or(saved_password).unwrap_or_default();
    if username.is_empty() || password.is_empty() {
        return Err(AppError("请先填写 asmr.one 账号和密码".to_string()));
    }
    let api_base = crate::api::asmrone::normalize_api_base(address.as_deref());

    let username2 = username.clone();
    let token = tokio::task::spawn_blocking(move || {
        crate::api::asmrone::login(&api_base, &username, &password, &proxy).map_err(AppError::new)
    })
    .await
    .map_err(AppError::new)??;

    // 登录成功：保存 token（账号密码由「保存设置」或本次请求体负责持久化）
    tokio::task::spawn_blocking(move || -> Result<(), AppError> {
        let conn = state.db.lock().map_err(AppError::new)?;
        queries::set_setting(&conn, "asmr_one_token", &token).map_err(AppError::new)?;
        queries::set_setting(&conn, "asmr_one_username", username2.trim()).map_err(AppError::new)?;
        if let Some(p) = provided_password.filter(|p| !p.is_empty()) {
            queries::set_setting(&conn, "asmr_one_password", &p).map_err(AppError::new)?;
        }
        Ok(())
    })
    .await
    .map_err(AppError::new)??;

    Ok(Json(json!({ "ok": true, "message": "登录成功，Token 已自动保存" })))
}

pub async fn db_path(
    State(state): State<SharedState>,
) -> Result<Json<String>, AppError> {
    Ok(Json(state.db_path.to_string_lossy().to_string()))
}

// ============================= 数据库导出 / 导入 =============================

/// 导出（备份）数据库：VACUUM INTO 生成一致性快照后作为浏览器下载返回。
pub async fn export_database(
    State(state): State<SharedState>,
) -> Result<impl IntoResponse, AppError> {
    let bytes = tokio::task::spawn_blocking(move || {
        // 生成不重复的临时备份路径
        let tmp = std::env::temp_dir().join(format!(
            "dlsite_manager_backup_{}.db",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or(0)
        ));
        let conn = state.db.lock().map_err(AppError::new)?;
        let sql = format!("VACUUM INTO '{}';", tmp.to_string_lossy().replace('\'', "''"));
        conn.execute_batch(&sql).map_err(AppError::new)?;
        drop(conn);
        let bytes = std::fs::read(&tmp).map_err(AppError::new)?;
        let _ = std::fs::remove_file(&tmp);
        Ok::<Vec<u8>, AppError>(bytes)
    })
    .await
    .map_err(AppError::new)??;

    let headers = [
        (header::CONTENT_TYPE, "application/octet-stream".to_string()),
        (
            header::CONTENT_DISPOSITION,
            "attachment; filename=\"dlsite_manager_backup.db\"".to_string(),
        ),
    ];
    Ok((StatusCode::OK, headers, Body::from(bytes)))
}

/// 导入（还原）数据库：接收上传的 .db 文件，校验后覆盖当前数据库。
pub async fn import_database(
    State(state): State<SharedState>,
    mut multipart: Multipart,
) -> Result<Json<serde_json::Value>, AppError> {
    let mut file_bytes: Option<Vec<u8>> = None;
    while let Some(field) = multipart.next_field().await.map_err(AppError::new)? {
        if field.name() == Some("file") {
            let data = field.bytes().await.map_err(AppError::new)?;
            file_bytes = Some(data.to_vec());
        }
    }
    let bytes = file_bytes.ok_or_else(|| AppError("未接收到数据库文件".to_string()))?;
    if bytes.is_empty() {
        return Err(AppError("上传的文件为空".to_string()));
    }

    // 写入临时文件用于校验与复制
    let tmp = std::env::temp_dir().join(format!(
        "dlsite_manager_import_{}.db",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0)
    ));
    std::fs::write(&tmp, &bytes).map_err(AppError::new)?;

    let result = tokio::task::spawn_blocking(move || {
        // 验证源文件是合法的 SQLite 数据库且包含 works 表
        {
            let test_conn = rusqlite::Connection::open(&tmp)
                .map_err(|e| AppError(format!("无法打开源数据库: {e}")))?;
            let has_works: bool = test_conn
                .prepare("SELECT 1 FROM sqlite_master WHERE type='table' AND name='works'")
                .and_then(|mut stmt| stmt.exists([]))
                .map_err(|e| AppError(format!("数据库格式异常: {e}")))?;
            if !has_works {
                return Err(AppError(
                    "该文件不是有效的 ASMR DB 数据库（缺少 works 表）".to_string(),
                ));
            }
        }

        let current_db_path = state.db_path.clone();
        let mut conn_guard = state.db.lock().map_err(AppError::new)?;

        // WAL checkpoint 确保当前数据落盘
        let _ = conn_guard.pragma_update(None, "journal_mode", "WAL");

        // 备份当前数据库（以防万一）
        let backup_path = current_db_path.with_extension("db.bak");
        let _ = std::fs::copy(&current_db_path, &backup_path);

        // 用内存库占位释放文件锁，再覆盖数据库文件
        let placeholder = rusqlite::Connection::open_in_memory().map_err(AppError::new)?;
        *conn_guard = placeholder;

        std::fs::copy(&tmp, &current_db_path)
            .map_err(|e| AppError(format!("覆盖数据库失败: {e}")))?;
        let _ = std::fs::remove_file(current_db_path.with_extension("db-wal"));
        let _ = std::fs::remove_file(current_db_path.with_extension("db-shm"));
        let _ = std::fs::remove_file(&tmp);

        // 重新打开新的数据库并运行迁移
        let new_conn = crate::db::init_database(&current_db_path)
            .map_err(|e| AppError(format!("重新初始化数据库失败: {e}")))?;
        *conn_guard = new_conn;

        let count: i64 = conn_guard
            .query_row("SELECT COUNT(*) FROM works", [], |r| r.get(0))
            .unwrap_or(0);

        Ok(format!(
            "导入成功！已加载 {count} 部作品。旧数据已备份到 {}",
            backup_path.display()
        ))
    })
    .await
    .map_err(AppError::new)??;

    Ok(Json(json!({ "message": result })))
}

// ============================= 打开目录 / 文件 =============================

#[derive(Deserialize)]
pub struct PathBody {
    pub path: String,
}

/// 在服务器所在机器上用文件管理器打开目录。
pub async fn open_folder(
    Json(body): Json<PathBody>,
) -> Result<Json<serde_json::Value>, AppError> {
    let p = PathBuf::from(&body.path);
    let target = if p.is_file() {
        p.parent().map(|d| d.to_path_buf()).unwrap_or(p)
    } else {
        p
    };
    if !target.exists() {
        return Err(AppError(format!("路径不存在: {}", target.display())));
    }
    spawn_reveal(&target).map_err(AppError::new)?;
    Ok(Json(json!({ "ok": true })))
}

/// 用系统默认播放器打开音频文件。
pub async fn open_audio_file(
    Json(body): Json<PathBody>,
) -> Result<Json<serde_json::Value>, AppError> {
    let p = PathBuf::from(&body.path);
    if !p.exists() {
        return Err(AppError(format!("文件不存在: {}", p.display())));
    }
    spawn_reveal(&p).map_err(AppError::new)?;
    Ok(Json(json!({ "ok": true })))
}

fn spawn_reveal(path: &Path) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        std::process::Command::new("xdg-open")
            .arg(path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

// ============================= 字幕查找 / 拉取 =============================

/// 递归收集目录下的字幕文件（.lrc / .vtt / .srt）。
fn collect_subtitles(dir: &Path, out: &mut Vec<PathBuf>, depth: u32) {
    if depth > 4 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            collect_subtitles(&p, out, depth + 1);
        } else if let Some(ext) = p.extension().map(|e| e.to_string_lossy().to_lowercase()) {
            if matches!(ext.as_str(), "lrc" | "vtt" | "srt") {
                out.push(p);
            }
        }
    }
}

/// 解码文本：优先 UTF-8，其次 Shift-JIS，最后 UTF-8 lossy。
fn decode_text(bytes: &[u8]) -> String {
    let bytes = if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        &bytes[3..]
    } else {
        bytes
    };
    match std::str::from_utf8(bytes) {
        Ok(s) => s.to_string(),
        Err(_) => {
            let (cow, _, _) = encoding_rs::SHIFT_JIS.decode(bytes);
            let s = cow.into_owned();
            if s.chars().any(|c| c != '\u{FFFD}') {
                s
            } else {
                String::from_utf8_lossy(bytes).into_owned()
            }
        }
    }
}

/// 读取字幕文件并解码，返回 `{ name, content }`。
fn read_subtitle(path: &Path) -> Result<Option<serde_json::Value>, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("读取字幕失败: {e}"))?;
    let content = decode_text(&bytes);
    if content.trim().is_empty() {
        return Ok(None);
    }
    Ok(Some(json!({
        "name": path.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default(),
        "content": content,
    })))
}

#[derive(Deserialize)]
pub struct SubtitleQuery {
    pub path: String,
}

/// 查找音轨对应的字幕文件（.lrc / .vtt / .srt）并读取内容。
///
/// 查找顺序：
/// 1. 音轨同目录下同 basename 的 .lrc / .vtt / .srt
/// 2. 向上定位到作品根目录（目录名含 RJ 号），递归搜索其中的字幕文件，
///    优先同名文件，其次文件名包含音轨 basename 的。
pub async fn find_subtitle(
    axum::extract::Query(q): axum::extract::Query<SubtitleQuery>,
) -> Result<Json<Option<serde_json::Value>>, AppError> {
    let result = tokio::task::spawn_blocking(move || find_subtitle_impl(&q.path).map_err(AppError::new))
        .await
        .map_err(AppError::new)??;
    Ok(Json(result))
}

fn find_subtitle_impl(track_path: &str) -> Result<Option<serde_json::Value>, String> {
    let tp = PathBuf::from(track_path);
    let dir = tp.parent().map(|d| d.to_path_buf()).unwrap_or_else(|| tp.clone());
    let stem = tp
        .file_stem()
        .map(|s| s.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    // 1) 同目录同 basename（大小写不敏感）
    for ext in ["lrc", "vtt", "srt"] {
        for cand_name in [format!("{stem}.{ext}"), format!("{}.{ext}", stem.to_uppercase())] {
            let cand = dir.join(&cand_name);
            if cand.is_file() {
                if let Some(f) = read_subtitle(&cand)? {
                    return Ok(Some(f));
                }
            }
        }
    }

    // 2) 向上找到作品根目录，递归搜索
    let mut root: Option<PathBuf> = None;
    let mut cur = dir.clone();
    for _ in 0..8 {
        let name = cur
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        if super::import::extract_rj_code(&name).is_some() {
            root = Some(cur.clone());
            break;
        }
        if !cur.pop() {
            break;
        }
    }
    let search_dir = root.unwrap_or(dir);

    let mut found: Vec<PathBuf> = Vec::new();
    collect_subtitles(&search_dir, &mut found, 0);
    // 优先同 basename，其次包含 basename，最后按路径排序
    found.sort_by_key(|p| {
        let p_stem = p
            .file_stem()
            .map(|s| s.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        let score = if p_stem == stem {
            0
        } else if p_stem.contains(&stem) {
            1
        } else {
            2
        };
        (score, p.to_string_lossy().to_lowercase())
    });
    for p in found {
        if let Some(f) = read_subtitle(&p)? {
            return Ok(Some(f));
        }
    }
    Ok(None)
}

#[derive(Deserialize)]
pub struct SubtitleFetchQuery {
    pub url: String,
}

/// 拉取在线字幕文件文本内容并自动解码（支持本地路径 / http(s) / asmr 代理路径）。
pub async fn fetch_subtitle(
    State(state): State<SharedState>,
    axum::extract::Query(q): axum::extract::Query<SubtitleFetchQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let (token, proxy, api_base) = {
        let conn = state.db.lock().map_err(AppError::new)?;
        (
            queries::get_setting(&conn, "asmr_one_token")
                .ok()
                .flatten()
                .unwrap_or_default(),
            crate::api::proxy::ProxyConfig::from_conn(&conn),
            crate::api::asmrone::normalize_api_base(
                queries::get_setting(&conn, "asmr_one_address").ok().flatten().as_deref(),
            ),
        )
    };
    let url = q.url;
    let content = tokio::task::spawn_blocking(move || {
        fetch_subtitle_impl(&url, &token, &proxy, &api_base).map_err(AppError::new)
    })
    .await
    .map_err(AppError::new)??;
    Ok(Json(json!({ "content": content })))
}

fn fetch_subtitle_impl(
    url: &str,
    token: &str,
    proxy: &crate::api::proxy::ProxyConfig,
    api_base: &str,
) -> Result<String, String> {
    let url_trimmed = url.trim();
    if url_trimmed.is_empty() {
        return Ok(String::new());
    }

    // 1) 本地路径
    if !url_trimmed.starts_with("http://")
        && !url_trimmed.starts_with("https://")
        && !url_trimmed.starts_with("asmr://")
        && !url_trimmed.starts_with("/api/")
    {
        let clean_path = url_trimmed.strip_prefix("file://").unwrap_or(url_trimmed);
        let p = PathBuf::from(clean_path);
        if p.is_file() {
            let bytes = std::fs::read(&p).map_err(|e| format!("读取本地字幕失败: {e}"))?;
            return Ok(decode_text(&bytes));
        }
    }

    // 2) 在线 URL / 本服务 asmr 代理路径 / 旧自定义协议路径
    let target_url = if let Some(hash) = url_trimmed.strip_prefix("/api/asmr/file/") {
        format!("{api_base}/api/file/{hash}")
    } else if let Some(hash) = url_trimmed.strip_prefix("http://asmr.localhost/file/") {
        format!("{api_base}/api/file/{hash}")
    } else if let Some(hash) = url_trimmed.strip_prefix("asmr://file/") {
        format!("{api_base}/api/file/{hash}")
    } else if let Some(hash) = url_trimmed.strip_prefix("asmr://") {
        format!("{api_base}/api/file/{hash}")
    } else {
        url_trimmed.to_string()
    };

    let builder = reqwest::blocking::Client::builder()
        .user_agent(crate::api::asmrone::USER_AGENT)
        .timeout(std::time::Duration::from_secs(30));
    // asmr.one 相关域名按 asmr.one 的开关走代理，其余 URL 不走代理
    let is_asmr = target_url.contains("asmr-100.com")
        || target_url.contains("asmr-200.com")
        || target_url.contains("asmr.one");
    let builder = if is_asmr {
        proxy.apply_blocking(builder, crate::api::proxy::ProxyTarget::AsmrOne)
    } else {
        builder
    };
    let client = builder
        .build()
        .map_err(|e| format!("构建请求客户端失败: {e}"))?;

    let mut req = client.get(&target_url);
    if is_asmr {
        if !token.trim().is_empty() {
            req = req.header("Authorization", format!("Bearer {}", token.trim()));
        }
    }

    let resp = req.send().map_err(|e| format!("拉取字幕失败: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("拉取字幕 HTTP 状态异常: {}", resp.status()));
    }

    let bytes = resp.bytes().map_err(|e| format!("读取字幕数据流失败: {e}"))?;
    Ok(decode_text(&bytes))
}
