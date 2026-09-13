mod api;
mod commands;
mod db;

use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::sync::Mutex;
use tauri::Manager;

// #region debug-point A:rust-startup
fn dbg_event(hypothesis_id: &str, location: &str, msg: &str, data: serde_json::Value) {
    let mut server_url = "http://127.0.0.1:7777/event".to_string();
    let mut session_id = "startup-blank-pages".to_string();
    if let Ok(env_content) = std::fs::read_to_string(".dbg/startup-blank-pages.env") {
        for line in env_content.lines() {
            if let Some(v) = line.strip_prefix("DEBUG_SERVER_URL=") {
                server_url = v.trim().to_string();
            } else if let Some(v) = line.strip_prefix("DEBUG_SESSION_ID=") {
                session_id = v.trim().to_string();
            }
        }
    }

    let body = serde_json::json!({
        "sessionId": session_id,
        "runId": "pre-fix",
        "hypothesisId": hypothesis_id,
        "location": location,
        "msg": msg,
        "data": data,
        "ts": SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    });

    let _ = reqwest::blocking::Client::new()
        .post(server_url)
        .timeout(Duration::from_millis(300))
        .header("Content-Type", "application/json")
        .body(body.to_string())
        .send();
}
// #endregion

/// 把 asmr protocol 的调试信息追加写入运行目录下的 asmr_protocol.log。
fn log_asmr(msg: &str) {
    use std::io::Write;
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("asmr_protocol.log")
    {
        let _ = writeln!(f, "{}", msg);
    }
}

/// App state holding the shared SQLite connection.
pub struct AppState {
    pub db: Mutex<rusqlite::Connection>,
}

/// Convenience alias used by command modules for `State<'_, AppState>`.
pub type AppStateRef = AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // #region debug-point A:rust-run
    dbg_event("A", "src-tauri/src/lib.rs", "[DEBUG] run() entered", serde_json::json!({}));
    // #endregion

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        // asmr URI scheme：前端用 http://asmr.localhost/file/{id} 访问 asmr.one 音频，
        // 此处拦截请求，附加 Bearer token 并透传 Range，把响应（含 206/Content-Range）
        // 原样返回，使 Howler 的 HTML5 audio 能加载与 seek。token 不暴露给前端。
        .register_uri_scheme_protocol("asmr", move |app, request| {
            // 解析文件 hash：路径形如 /file/{workId}/{fileId}
            let path = request.uri().path();
            let hash = path.strip_prefix("/file/").unwrap_or("");
            if hash.is_empty() {
                return tauri::http::Response::builder()
                    .status(tauri::http::StatusCode::BAD_REQUEST)
                    .body(b"bad file hash".to_vec())
                    .unwrap();
            }

            // hash 形如 "workId/fileId"，文件流端点用 fileId（最后一段）
            let file_id = hash.rsplit('/').next().unwrap_or(hash);

            // 从 DB 读 asmr.one token
            let token = app
                .app_handle()
                .state::<AppState>()
                .db
                .lock()
                .ok()
                .and_then(|c| db::queries::get_setting(&c, "asmr_one_token").ok().flatten())
                .unwrap_or_default();

            // asmr.one 文件流端点。tracks 返回的 hash 形如 "workId/fileId"，
            // 先试完整 hash；若 404 再退回纯 fileId（两种可能性都覆盖）。
            let client = reqwest::blocking::Client::builder()
                .user_agent(api::asmrone::USER_AGENT)
                .timeout(std::time::Duration::from_secs(60))
                .build();
            let client = match client {
                Ok(c) => c,
                Err(e) => {
                    return tauri::http::Response::builder()
                        .status(tauri::http::StatusCode::BAD_GATEWAY)
                        .body(e.to_string().into_bytes())
                        .unwrap();
                }
            };
            // 发起一次带 token + Range 的转发请求
            let do_fetch = |url: &str| -> Result<reqwest::blocking::Response, reqwest::Error> {
                let mut req = client.get(url);
                if !token.trim().is_empty() {
                    req = req.header("Authorization", format!("Bearer {}", token));
                }
                // 透传 Range header（HTML5 audio seek 依赖）
                if let Some(range) = request.headers().get("range") {
                    req = req.header("Range", range);
                }
                req.send()
            };

            let url_hash = format!("https://api.asmr-100.com/api/file/{}", hash);
            let url_fid = format!("https://api.asmr-100.com/api/file/{}", file_id);

            let (resp, used_url) = match do_fetch(&url_hash) {
                Ok(r) if r.status() == reqwest::StatusCode::NOT_FOUND && file_id != hash => {
                    log_asmr(&format!("hash 404, retry file_id path={}", path));
                    match do_fetch(&url_fid) {
                        Ok(r2) => (r2, url_fid),
                        Err(e) => {
                            log_asmr(&format!("err path={} msg={}", path, e));
                            return tauri::http::Response::builder()
                                .status(tauri::http::StatusCode::BAD_GATEWAY)
                                .body(format!("asmr.one 请求失败: {e}").into_bytes())
                                .unwrap();
                        }
                    }
                }
                Ok(r) => (r, url_hash),
                Err(e) => {
                    log_asmr(&format!("err path={} msg={}", path, e));
                    return tauri::http::Response::builder()
                        .status(tauri::http::StatusCode::BAD_GATEWAY)
                        .body(format!("asmr.one 请求失败: {e}").into_bytes())
                        .unwrap();
                }
            };

            // 透传状态码与流式/缓存相关 header，body 原样返回
            let status = tauri::http::StatusCode::from_u16(resp.status().as_u16())
                .unwrap_or(tauri::http::StatusCode::OK);
            let ct = resp.headers().get("content-type").and_then(|v| v.to_str().ok()).unwrap_or("").to_string();
            let cl = resp.headers().get("content-length").and_then(|v| v.to_str().ok()).unwrap_or("").to_string();
            let cr = resp.headers().get("content-range").and_then(|v| v.to_str().ok()).unwrap_or("").to_string();
            let body_len = resp.content_length().unwrap_or(0);
            log_asmr(&format!("ok url={} status={} ct={} cl={} cr={} body_len={}", used_url, status.as_u16(), ct, cl, cr, body_len));
            let mut builder = tauri::http::Response::builder().status(status);
            for key in &[
                "content-type",
                "content-length",
                "content-range",
                "accept-ranges",
                "last-modified",
                "etag",
            ] {
                if let Some(val) = resp.headers().get(*key) {
                    builder = builder.header(*key, val);
                }
            }
            let body = resp.bytes().unwrap_or_default();
            builder.body(body.to_vec()).unwrap_or_else(|_| {
                tauri::http::Response::builder()
                    .status(tauri::http::StatusCode::INTERNAL_SERVER_ERROR)
                    .body(b"build response failed".to_vec())
                    .unwrap()
            })
        })
        .setup(|app| {
            // #region debug-point A:rust-setup
            dbg_event("A", "src-tauri/src/lib.rs", "[DEBUG] setup() start", serde_json::json!({}));
            // #endregion

            // Initialize the database in the app data directory
            // #region debug-point A:db-init
            dbg_event("A", "src-tauri/src/lib.rs", "[DEBUG] db::init_database() begin", serde_json::json!({}));
            // #endregion
            let db = match db::init_database(app.handle()) {
                Ok(db) => {
                    // #region debug-point A:db-init-ok
                    dbg_event("A", "src-tauri/src/lib.rs", "[DEBUG] db::init_database() ok", serde_json::json!({}));
                    // #endregion
                    db
                }
                Err(e) => {
                    // #region debug-point A:db-init-err
                    dbg_event(
                        "A",
                        "src-tauri/src/lib.rs",
                        "[DEBUG] db::init_database() err",
                        serde_json::json!({ "error": e.to_string() }),
                    );
                    // #endregion
                    return Err(e);
                }
            };
            app.manage(AppState { db: Mutex::new(db) });
            app.manage(commands::download::DownloadManager::default());

            // #region debug-point A:rust-setup-ok
            dbg_event("A", "src-tauri/src/lib.rs", "[DEBUG] setup() ok", serde_json::json!({}));
            // #endregion
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // works
            commands::works::list_works,
            commands::works::get_work,
            commands::works::update_work,
            commands::works::delete_work,
            commands::works::delete_works,
            commands::works::list_tags,
            commands::works::create_tag,
            commands::works::assign_tag,
            commands::works::unassign_tag,
            commands::works::delete_tag,
            commands::works::list_actors,
            commands::works::list_tracks,
            commands::works::get_download_status,
            // groups
            commands::works::list_groups,
            commands::works::create_group,
            commands::works::set_work_group,
            commands::works::delete_group,
            // import
            commands::import::import_by_rj,
            commands::import::preview_by_rj,
            commands::import::scan_folder,
            commands::import::bind_local_folder,
            commands::import::scan_audio_tracks,
            // download
            commands::download::download_work,
            commands::download::download_asmrone_files,
            commands::download::pause_download,
            commands::download::resume_download,
            commands::download::cancel_download,
            commands::download::get_download_queue,
            commands::download::set_download_settings,
            commands::download::get_download_settings,
            // player
            commands::works::save_play_progress,
            commands::works::get_play_progress,
            // settings
            commands::settings::get_settings,
            commands::settings::set_settings,
            commands::settings::get_db_path,
            commands::settings::export_database,
            commands::settings::import_database,
            commands::settings::open_folder,
            commands::settings::open_audio_file,
            commands::settings::find_subtitle_for_track,
            commands::settings::fetch_subtitle_content,
            // asmr.one token / webview
            commands::settings::set_asmr_token,
            commands::settings::clear_asmr_token,
            commands::settings::eval_in_window,
            commands::settings::get_window_url,
            commands::settings::open_asmr_login_window,
            commands::settings::close_asmr_login_window,
            commands::settings::eval_in_window_result,
            // asmr.one online playback & tree
            commands::works::list_asmrone_tracks,
            commands::works::list_asmrone_tree,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
