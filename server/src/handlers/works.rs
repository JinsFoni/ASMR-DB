use axum::extract::{Path, Query, State};
use axum::Json;
use serde::Deserialize;
use serde_json::json;

use crate::db::models::{Tag, Track, Work, WorkView};
use crate::db::queries;
use crate::state::{AppError, SharedState};

// ============================= 作品 =============================

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkListQuery {
    pub search: Option<String>,
    pub status: Option<String>,
    pub tag_id: Option<i64>,
    pub group_id: Option<i64>,
    pub sort_by: Option<String>,
    pub page: Option<u32>,
    pub per_page: Option<u32>,
}

pub async fn list_works(
    State(state): State<SharedState>,
    Query(q): Query<WorkListQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let items = tokio::task::spawn_blocking(
        move || -> Result<(Vec<WorkView>, u32), AppError> {
            let conn = state.db.lock().map_err(AppError::new)?;
            let (items, total) = queries::list_works(
                &conn,
                q.search.as_deref(),
                q.status.as_deref(),
                q.tag_id,
                q.group_id,
                q.sort_by.as_deref().unwrap_or("id"),
                q.page.unwrap_or(1),
                q.per_page.unwrap_or(48),
            )
            .map_err(AppError::new)?;
            Ok((items, total))
        },
    )
    .await
    .map_err(AppError::new)??;

    Ok(Json(json!({ "items": items.0, "total": items.1 })))
}

pub async fn get_work(
    State(state): State<SharedState>,
    Path(id): Path<i64>,
) -> Result<Json<Option<WorkView>>, AppError> {
    let view = tokio::task::spawn_blocking(move || {
        let conn = state.db.lock().map_err(AppError::new)?;
        queries::get_work(&conn, id).map_err(AppError::new)
    })
    .await
    .map_err(AppError::new)??;
    Ok(Json(view))
}

pub async fn update_work(
    State(state): State<SharedState>,
    Json(work): Json<Work>,
) -> Result<Json<serde_json::Value>, AppError> {
    tokio::task::spawn_blocking(move || {
        let conn = state.db.lock().map_err(AppError::new)?;
        queries::update_work(&conn, &work).map_err(AppError::new)
    })
    .await
    .map_err(AppError::new)??;
    Ok(Json(json!({ "ok": true })))
}

pub async fn delete_work(
    State(state): State<SharedState>,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, AppError> {
    tokio::task::spawn_blocking(move || {
        let conn = state.db.lock().map_err(AppError::new)?;
        queries::delete_work(&conn, id).map_err(AppError::new)
    })
    .await
    .map_err(AppError::new)??;
    Ok(Json(json!({ "ok": true })))
}

#[derive(Deserialize)]
pub struct DeleteWorksBody {
    pub ids: Vec<i64>,
}

pub async fn delete_works(
    State(state): State<SharedState>,
    Json(body): Json<DeleteWorksBody>,
) -> Result<Json<serde_json::Value>, AppError> {
    tokio::task::spawn_blocking(move || {
        let conn = state.db.lock().map_err(AppError::new)?;
        queries::delete_works(&conn, &body.ids).map_err(AppError::new)
    })
    .await
    .map_err(AppError::new)??;
    Ok(Json(json!({ "ok": true })))
}

pub async fn get_download_status(
    State(state): State<SharedState>,
    Path(id): Path<i64>,
) -> Result<Json<String>, AppError> {
    let status = tokio::task::spawn_blocking(move || -> Result<String, AppError> {
        let conn = state.db.lock().map_err(AppError::new)?;
        let (status, _) = queries::work_download_status(&conn, id).map_err(AppError::new)?;
        Ok(status)
    })
    .await
    .map_err(AppError::new)??;
    Ok(Json(status))
}

// ============================= 分组 =============================

pub async fn list_groups(
    State(state): State<SharedState>,
) -> Result<Json<Vec<crate::db::models::Group>>, AppError> {
    let groups = tokio::task::spawn_blocking(move || {
        let conn = state.db.lock().map_err(AppError::new)?;
        queries::list_groups(&conn).map_err(AppError::new)
    })
    .await
    .map_err(AppError::new)??;
    Ok(Json(groups))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateGroupBody {
    pub name: String,
    pub color: Option<String>,
}

pub async fn create_group(
    State(state): State<SharedState>,
    Json(body): Json<CreateGroupBody>,
) -> Result<Json<i64>, AppError> {
    let id = tokio::task::spawn_blocking(move || {
        let conn = state.db.lock().map_err(AppError::new)?;
        queries::create_group(
            &conn,
            &body.name,
            body.color.as_deref().unwrap_or("#597ef7"),
        )
        .map_err(AppError::new)
    })
    .await
    .map_err(AppError::new)??;
    Ok(Json(id))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetWorkGroupBody {
    pub group_id: Option<i64>,
}

pub async fn set_work_group(
    State(state): State<SharedState>,
    Path(work_id): Path<i64>,
    Json(body): Json<SetWorkGroupBody>,
) -> Result<Json<serde_json::Value>, AppError> {
    tokio::task::spawn_blocking(move || {
        let conn = state.db.lock().map_err(AppError::new)?;
        queries::set_work_group(&conn, work_id, body.group_id).map_err(AppError::new)
    })
    .await
    .map_err(AppError::new)??;
    Ok(Json(json!({ "ok": true })))
}

pub async fn delete_group(
    State(state): State<SharedState>,
    Path(group_id): Path<i64>,
) -> Result<Json<serde_json::Value>, AppError> {
    tokio::task::spawn_blocking(move || {
        let conn = state.db.lock().map_err(AppError::new)?;
        queries::delete_group(&conn, group_id).map_err(AppError::new)
    })
    .await
    .map_err(AppError::new)??;
    Ok(Json(json!({ "ok": true })))
}

// ============================= 标签 =============================

pub async fn list_tags(State(state): State<SharedState>) -> Result<Json<Vec<Tag>>, AppError> {
    let tags = tokio::task::spawn_blocking(move || {
        let conn = state.db.lock().map_err(AppError::new)?;
        queries::list_tags(&conn).map_err(AppError::new)
    })
    .await
    .map_err(AppError::new)??;
    Ok(Json(tags))
}

#[derive(Deserialize)]
pub struct CreateTagBody {
    pub name: String,
    pub color: String,
}

pub async fn create_tag(
    State(state): State<SharedState>,
    Json(body): Json<CreateTagBody>,
) -> Result<Json<i64>, AppError> {
    let id = tokio::task::spawn_blocking(move || {
        let conn = state.db.lock().map_err(AppError::new)?;
        queries::create_tag(&conn, &body.name, &body.color).map_err(AppError::new)
    })
    .await
    .map_err(AppError::new)??;
    Ok(Json(id))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssignTagBody {
    pub tag_id: i64,
}

pub async fn assign_tag(
    State(state): State<SharedState>,
    Path(work_id): Path<i64>,
    Json(body): Json<AssignTagBody>,
) -> Result<Json<serde_json::Value>, AppError> {
    tokio::task::spawn_blocking(move || {
        let conn = state.db.lock().map_err(AppError::new)?;
        queries::assign_tag(&conn, work_id, body.tag_id).map_err(AppError::new)
    })
    .await
    .map_err(AppError::new)??;
    Ok(Json(json!({ "ok": true })))
}

pub async fn unassign_tag(
    State(state): State<SharedState>,
    Path((work_id, tag_id)): Path<(i64, i64)>,
) -> Result<Json<serde_json::Value>, AppError> {
    tokio::task::spawn_blocking(move || {
        let conn = state.db.lock().map_err(AppError::new)?;
        queries::unassign_tag(&conn, work_id, tag_id).map_err(AppError::new)
    })
    .await
    .map_err(AppError::new)??;
    Ok(Json(json!({ "ok": true })))
}

pub async fn delete_tag(
    State(state): State<SharedState>,
    Path(tag_id): Path<i64>,
) -> Result<Json<serde_json::Value>, AppError> {
    tokio::task::spawn_blocking(move || {
        let conn = state.db.lock().map_err(AppError::new)?;
        queries::delete_tag(&conn, tag_id).map_err(AppError::new)
    })
    .await
    .map_err(AppError::new)??;
    Ok(Json(json!({ "ok": true })))
}

// ============================= 声优 / 音轨 / 进度 =============================

pub async fn list_actors(
    State(state): State<SharedState>,
) -> Result<Json<Vec<String>>, AppError> {
    let actors = tokio::task::spawn_blocking(move || {
        let conn = state.db.lock().map_err(AppError::new)?;
        queries::list_all_actors(&conn).map_err(AppError::new)
    })
    .await
    .map_err(AppError::new)??;
    Ok(Json(actors))
}

pub async fn list_tracks(
    State(state): State<SharedState>,
    Path(work_id): Path<i64>,
) -> Result<Json<Vec<Track>>, AppError> {
    let tracks = tokio::task::spawn_blocking(move || {
        let conn = state.db.lock().map_err(AppError::new)?;
        queries::list_tracks(&conn, work_id).map_err(AppError::new)
    })
    .await
    .map_err(AppError::new)??;
    Ok(Json(tracks))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveProgressBody {
    pub track_id: Option<i64>,
    pub position: f64,
}

pub async fn save_play_progress(
    State(state): State<SharedState>,
    Path(work_id): Path<i64>,
    Json(body): Json<SaveProgressBody>,
) -> Result<Json<serde_json::Value>, AppError> {
    tokio::task::spawn_blocking(move || {
        let conn = state.db.lock().map_err(AppError::new)?;
        queries::save_play_progress(&conn, work_id, body.track_id, body.position)
            .map_err(AppError::new)
    })
    .await
    .map_err(AppError::new)??;
    Ok(Json(json!({ "ok": true })))
}

pub async fn get_play_progress(
    State(state): State<SharedState>,
    Path(work_id): Path<i64>,
) -> Result<Json<Option<crate::db::models::PlayHistory>>, AppError> {
    let progress = tokio::task::spawn_blocking(move || {
        let conn = state.db.lock().map_err(AppError::new)?;
        queries::get_play_progress(&conn, work_id).map_err(AppError::new)
    })
    .await
    .map_err(AppError::new)??;
    Ok(Json(progress))
}

pub async fn scan_audio_tracks(
    State(state): State<SharedState>,
    Path(work_id): Path<i64>,
) -> Result<Json<serde_json::Value>, AppError> {
    let result = tokio::task::spawn_blocking(
        move || -> Result<serde_json::Value, AppError> {
            let conn = state.db.lock().map_err(AppError::new)?;
            let local_path: Option<String> = conn
                .query_row(
                    "SELECT local_path FROM local_files WHERE work_id = ?1 AND download_status = 'downloaded' ORDER BY id DESC LIMIT 1",
                    rusqlite::params![work_id],
                    |r| r.get(0),
                )
                .ok();
            let Some(local_path) = local_path else {
                return Ok(json!({ "ok": false, "count": 0, "message": "该作品没有已下载的本地文件夹" }));
            };
            let n = super::import::scan_audio_tracks_impl(&conn, work_id, &local_path)
                .map_err(AppError::new)?;
            Ok(json!({ "ok": true, "count": n }))
        },
    )
    .await
    .map_err(AppError::new)??;
    Ok(Json(result))
}

// ============================= asmr.one 在线试听 / 文件树 =============================

/// 拉取 asmr.one 上某作品的音频文件列表，构造为临时 `Track` 返回给前端在线播放。
/// `file_path` 直接用 asmr.one 公开 CDN 的 `mediaStreamUrl`（无需 token，支持 Range
/// seek）；仅当 CDN URL 缺失时回退到本服务的代理路径 `/api/asmr/file/{hash}`。
pub async fn list_asmrone_tracks(
    State(state): State<SharedState>,
    Query(q): Query<AsmrRjQuery>,
) -> Result<Json<Vec<Track>>, AppError> {
    let (token, proxy, api_base) = get_asmr_ctx(&state).await?;
    let files = tokio::task::spawn_blocking(move || {
        crate::api::asmrone::fetch_audio_files(&q.rj, Some(&token), &proxy, &api_base)
            .map_err(AppError::new)
    })
    .await
    .map_err(AppError::new)??;

    if files.is_empty() {
        return Err(AppError(
            "asmr.one 未返回音频文件，可能需要登录或该作品无音频".to_string(),
        ));
    }
    let tracks = files
        .into_iter()
        .enumerate()
        .filter_map(|(idx, f)| {
            if f.stream_url.trim().is_empty() {
                return None;
            }
            Some(Track {
                id: idx as i64,
                work_id: 0,
                file_path: f.stream_url,
                track_number: Some((idx + 1) as i64),
                title: Some(f.title),
                duration_sec: f.duration_sec,
                file_format: Some(f.media_extension),
                file_size: f.size_bytes,
                subtitles: f.subtitles,
            })
        })
        .collect();
    Ok(Json(tracks))
}

#[derive(Deserialize)]
pub struct AsmrRjQuery {
    pub rj: String,
}

pub async fn list_asmrone_tree(
    State(state): State<SharedState>,
    Query(q): Query<AsmrRjQuery>,
) -> Result<Json<Vec<crate::api::asmrone::AsmrTreeNode>>, AppError> {
    let (token, proxy, api_base) = get_asmr_ctx(&state).await?;
    let rj = q.rj;
    let tree = tokio::task::spawn_blocking(move || {
        crate::api::asmrone::fetch_file_tree(&rj, Some(&token), &proxy, &api_base)
            .map_err(AppError::new)
    })
    .await
    .map_err(AppError::new)??;
    Ok(Json(tree))
}

/// 读取 asmr.one 请求上下文：(token, 代理配置, API 基址)。
/// token 为空且设置了账号密码时自动登录并保存新 token。
async fn get_asmr_ctx(
    state: &SharedState,
) -> Result<(String, crate::api::proxy::ProxyConfig, String), AppError> {
    let state2 = state.clone();
    let (mut token, proxy, api_base, username, password) = tokio::task::spawn_blocking(
        move || -> Result<_, AppError> {
            let conn = state2.db.lock().map_err(AppError::new)?;
            let get = |k: &str| {
                queries::get_setting(&conn, k).ok().flatten().unwrap_or_default()
            };
            let api_base = crate::api::asmrone::normalize_api_base(
                queries::get_setting(&conn, "asmr_one_address").ok().flatten().as_deref(),
            );
            Ok((
                get("asmr_one_token"),
                crate::api::proxy::ProxyConfig::from_conn(&conn),
                api_base,
                get("asmr_one_username"),
                get("asmr_one_password"),
            ))
        },
    )
    .await
    .map_err(AppError::new)??;

    // 自动登录：token 为空且配置了账号密码（网络请求放在锁外，避免阻塞其他请求）
    if token.trim().is_empty() && !username.trim().is_empty() && !password.is_empty() {
        let api_base2 = api_base.clone();
        let proxy2 = proxy.clone();
        match tokio::task::spawn_blocking(move || {
            crate::api::asmrone::login(&api_base2, username.trim(), &password, &proxy2)
        })
        .await
        .map_err(AppError::new)?
        {
            Ok(t) => {
                let state2 = state.clone();
                let t2 = t.clone();
                tokio::task::spawn_blocking(move || {
                    let conn = state2.db.lock().map_err(AppError::new)?;
                    queries::set_setting(&conn, "asmr_one_token", &t2).map_err(AppError::new)
                })
                .await
                .map_err(AppError::new)??;
                println!("🔑 asmr.one 自动登录成功，token 已保存");
                token = t;
            }
            Err(e) => eprintln!("⚠️ asmr.one 自动登录失败: {e}"),
        }
    }
    Ok((token, proxy, api_base))
}
