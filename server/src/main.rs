mod api;
mod db;
mod handlers;
mod state;

use std::path::PathBuf;
use std::sync::Mutex;

use axum::routing::{delete, get, post, put};
use axum::Router;
use state::{AppState, DownloadManager};
use tower_http::cors::CorsLayer;
use tower_http::services::{ServeDir, ServeFile};

use state::SharedState;

/// 解析数据库路径：优先 ASMR_DB_DIR 环境变量，其次可执行文件同级目录（便携模式），
/// 最后退回当前工作目录。
pub fn resolve_db_path() -> PathBuf {
    if let Ok(dir) = std::env::var("ASMR_DB_DIR") {
        if !dir.trim().is_empty() {
            let p = PathBuf::from(dir.trim());
            let _ = std::fs::create_dir_all(&p);
            return p.join("dlsite_manager.db");
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let candidate = dir.join("dlsite_manager.db");
            if std::fs::OpenOptions::new()
                .create(true)
                .write(true)
                .open(&candidate)
                .is_ok()
            {
                return candidate;
            }
        }
    }
    PathBuf::from("dlsite_manager.db")
}

/// 解析前端构建产物目录：优先 ASMR_DIST_DIR，其次当前目录的 dist / ../dist，
/// 最后尝试 exe 同级 dist。
fn resolve_dist_dir() -> PathBuf {
    if let Ok(d) = std::env::var("ASMR_DIST_DIR") {
        if !d.trim().is_empty() {
            return PathBuf::from(d.trim());
        }
    }
    let cwd = std::env::current_dir().unwrap_or_default();
    for cand in [cwd.join("dist"), cwd.join("../dist")] {
        if cand.join("index.html").is_file() {
            return cand;
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let cand = dir.join("dist");
            if cand.join("index.html").is_file() {
                return cand;
            }
        }
    }
    cwd.join("dist")
}

#[tokio::main]
async fn main() {
    let db_path = resolve_db_path();
    let conn = match db::init_database(&db_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("数据库初始化失败 ({}): {e}", db_path.display());
            std::process::exit(1);
        }
    };
    println!("📦 数据库: {}", db_path.display());

    let dist_dir = resolve_dist_dir();
    if !dist_dir.join("index.html").is_file() {
        eprintln!(
            "⚠️  未找到前端构建产物（{}），请先运行 npm run build；API 仍可正常使用",
            dist_dir.display()
        );
    } else {
        println!("🖥️  前端资源: {}", dist_dir.display());
    }

    let state: SharedState = std::sync::Arc::new(AppState {
        db: Mutex::new(conn),
        manager: DownloadManager::new(),
        db_path,
    });

    let static_service = ServeDir::new(&dist_dir)
        .append_index_html_on_directories(true)
        .not_found_service(ServeFile::new(dist_dir.join("index.html")));

    let app = Router::new()
        // ============ 作品 ============
        .route("/api/works", get(handlers::works::list_works))
        .route("/api/works/delete-batch", post(handlers::works::delete_works))
        .route("/api/works", put(handlers::works::update_work))
        .route("/api/works/{id}", get(handlers::works::get_work))
        .route("/api/works/{id}", delete(handlers::works::delete_work))
        .route(
            "/api/works/{id}/download-status",
            get(handlers::works::get_download_status),
        )
        .route(
            "/api/works/{id}/group",
            put(handlers::works::set_work_group),
        )
        .route(
            "/api/works/{id}/tags",
            post(handlers::works::assign_tag),
        )
        .route(
            "/api/works/{id}/tags/{tag_id}",
            delete(handlers::works::unassign_tag),
        )
        .route(
            "/api/works/{id}/tracks",
            get(handlers::works::list_tracks),
        )
        .route(
            "/api/works/{id}/scan-tracks",
            post(handlers::works::scan_audio_tracks),
        )
        .route(
            "/api/works/{id}/progress",
            get(handlers::works::get_play_progress),
        )
        .route(
            "/api/works/{id}/progress",
            post(handlers::works::save_play_progress),
        )
        // ============ 标签 / 分组 / 声优 ============
        .route("/api/tags", get(handlers::works::list_tags))
        .route("/api/tags", post(handlers::works::create_tag))
        .route("/api/tags/{id}", delete(handlers::works::delete_tag))
        .route("/api/actors", get(handlers::works::list_actors))
        .route("/api/groups", get(handlers::works::list_groups))
        .route("/api/groups", post(handlers::works::create_group))
        .route("/api/groups/{id}", delete(handlers::works::delete_group))
        // ============ 导入 ============
        .route("/api/import/preview", post(handlers::import::preview_by_rj))
        .route("/api/import/by-rj", post(handlers::import::import_by_rj))
        .route("/api/import/scan-folder", post(handlers::import::scan_folder))
        .route(
            "/api/import/bind-folder",
            post(handlers::import::bind_local_folder),
        )
        // ============ asmr.one ============
        .route("/api/asmr/tracks", get(handlers::works::list_asmrone_tracks))
        .route("/api/asmr/tree", get(handlers::works::list_asmrone_tree))
        .route("/api/asmr/file/{*hash}", get(handlers::media::asmr_file))
        // ============ 下载 ============
        .route("/api/download", post(handlers::download::download_work))
        .route("/api/download/asmr", post(handlers::download::download_asmr))
        .route("/api/download/pause", post(handlers::download::pause_download))
        .route(
            "/api/download/resume",
            post(handlers::download::resume_download),
        )
        .route(
            "/api/download/cancel",
            post(handlers::download::cancel_download),
        )
        .route("/api/download/queue", get(handlers::download::get_queue))
        .route(
            "/api/download/settings",
            get(handlers::download::get_settings),
        )
        .route(
            "/api/download/settings",
            post(handlers::download::set_settings),
        )
        // ============ 设置 ============
        .route("/api/settings", get(handlers::settings::get_settings))
        .route("/api/settings", post(handlers::settings::set_settings))
        .route(
            "/api/settings/asmr-token",
            post(handlers::settings::set_asmr_token),
        )
        .route(
            "/api/settings/asmr-token",
            delete(handlers::settings::clear_asmr_token),
        )
        .route("/api/settings/db-path", get(handlers::settings::db_path))
        .route(
            "/api/settings/db/export",
            get(handlers::settings::export_database),
        )
        .route(
            "/api/settings/db/import",
            post(handlers::settings::import_database),
        )
        .route(
            "/api/settings/open-folder",
            post(handlers::settings::open_folder),
        )
        .route(
            "/api/settings/open-audio",
            post(handlers::settings::open_audio_file),
        )
        // ============ 字幕 ============
        .route(
            "/api/subtitle/find",
            get(handlers::settings::find_subtitle),
        )
        .route(
            "/api/subtitle/fetch",
            get(handlers::settings::fetch_subtitle),
        )
        // ============ 文件系统 / 音频 / 事件 ============
        .route("/api/fs/list", get(handlers::fs::list_dir))
        .route("/api/audio", get(handlers::media::local_audio))
        .route("/api/events", get(handlers::events::events))
        .fallback_service(static_service)
        .layer(CorsLayer::permissive())
        .with_state(state);

    let host = std::env::var("ASMR_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port: u16 = std::env::var("ASMR_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(1421);
    let addr = format!("{host}:{port}");

    let listener = match tokio::net::TcpListener::bind(&addr).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("监听 {addr} 失败: {e}");
            std::process::exit(1);
        }
    };
    println!("🎧 DLsite ASMR Manager 服务已启动: http://localhost:{port}");

    if let Err(e) = axum::serve(listener, app).await {
        eprintln!("服务运行错误: {e}");
        std::process::exit(1);
    }
}
