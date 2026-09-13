use std::path::PathBuf;

use axum::extract::{Query, State};
use axum::Json;
use serde::Deserialize;
use serde_json::json;

use crate::state::{AppError, SharedState};

#[derive(Deserialize)]
pub struct ListQuery {
    pub path: Option<String>,
}

/// 浏览服务器上的目录（替代桌面端原生目录选择对话框）。
/// 仅返回子目录；path 为空时从用户主目录开始。
pub async fn list_dir(
    State(_state): State<SharedState>,
    Query(q): Query<ListQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let result = tokio::task::spawn_blocking(move || {
        let dir: PathBuf = match q.path.as_deref().map(str::trim) {
            Some(p) if !p.is_empty() => PathBuf::from(p),
            _ => dirs::home_dir().unwrap_or_else(|| PathBuf::from("/")),
        };
        if !dir.is_dir() {
            return Err(AppError("路径不是文件夹".to_string()));
        }

        let mut entries: Vec<serde_json::Value> = Vec::new();
        if let Ok(rd) = std::fs::read_dir(&dir) {
            for e in rd.flatten() {
                let p = e.path();
                if !p.is_dir() {
                    continue;
                }
                let name = e.file_name().to_string_lossy().to_string();
                // 跳过隐藏目录
                if name.starts_with('.') {
                    continue;
                }
                entries.push(json!({
                    "name": name,
                    "path": p.to_string_lossy(),
                    "isDir": true,
                }));
            }
        }
        entries.sort_by(|a, b| {
            let an = a["name"].as_str().unwrap_or("").to_lowercase();
            let bn = b["name"].as_str().unwrap_or("").to_lowercase();
            an.cmp(&bn)
        });

        let parent = dir
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .map(|p| p.to_string_lossy().to_string());

        Ok(json!({
            "path": dir.to_string_lossy(),
            "parent": parent,
            "entries": entries,
        }))
    })
    .await
    .map_err(AppError::new)??;

    Ok(Json(result))
}
