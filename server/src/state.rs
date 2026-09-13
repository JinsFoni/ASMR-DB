use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use serde::Serialize;
use tokio::sync::broadcast;

/// 全局共享状态：SQLite 连接 + 下载管理器 + 数据库路径。
pub struct AppState {
    pub db: Mutex<rusqlite::Connection>,
    pub manager: DownloadManager,
    /// 数据库文件绝对路径（下载线程等会用独立连接访问）
    pub db_path: PathBuf,
}

pub type SharedState = Arc<AppState>;

// ============================= 下载管理 =============================

#[derive(Clone, Serialize)]
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

pub struct DownloadManager {
    pub tasks: Mutex<HashMap<String, DownloadHandle>>,
    /// 下载进度广播（SSE 订阅），容量 256 足够覆盖进度节流频率
    pub tx: broadcast::Sender<DownloadProgressEvent>,
}

pub struct DownloadHandle {
    pub task: crate::db::models::DownloadTask,
    pub cancel: Arc<AtomicBool>,
}

impl DownloadManager {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(256);
        Self {
            tasks: Mutex::new(HashMap::new()),
            tx,
        }
    }

    /// 广播下载进度事件（无订阅者时静默丢弃）。
    pub fn emit(&self, ev: DownloadProgressEvent) {
        let _ = self.tx.send(ev);
    }

    pub fn is_busy(&self, rj: &str) -> bool {
        self.tasks
            .lock()
            .ok()
            .and_then(|t| t.get(rj).map(|h| h.task.status == "downloading" || h.task.status == "queued"))
            .unwrap_or(false)
    }

    pub fn cancel_flag(&self, rj: &str) -> Option<Arc<AtomicBool>> {
        self.tasks.lock().ok()?.get(rj).map(|h| h.cancel.clone())
    }

    pub fn check_cancel(&self, rj: &str) -> bool {
        self.cancel_flag(rj)
            .map(|f| f.load(Ordering::SeqCst))
            .unwrap_or(false)
    }
}

// ============================= 统一错误类型 =============================

/// 简单错误包装：所有 handler 返回 Result<_, AppError>，
/// 序列化为 500 + {"error": "..."}，前端 api 层会把 error 字段抛为异常。
pub struct AppError(pub String);

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl axum::response::IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            axum::Json(serde_json::json!({ "error": self.0 })),
        )
            .into_response()
    }
}

impl AppError {
    pub fn new<E: std::fmt::Display>(e: E) -> Self {
        AppError(e.to_string())
    }
}

impl From<rusqlite::Error> for AppError {
    fn from(e: rusqlite::Error) -> Self {
        AppError(e.to_string())
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError(e.to_string())
    }
}
