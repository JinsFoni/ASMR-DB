use axum::extract::{Path, Query, State};
use axum::Json;
use serde::Deserialize;
use serde_json::json;

use crate::api::asmr_online::{self, AsmrOnlineWork};
use crate::db::models::Work;
use crate::db::queries;
use crate::handlers::works::{asmr_relogin, get_asmr_ctx};
use crate::state::{AppError, SharedState};

const PAGE_SIZE: i64 = 20;

/// 通用执行：先按当前 token 请求；返回 401 时重新登录并重试一次。
async fn run_with_relogin<T, F>(state: &SharedState, fetch: F) -> Result<T, AppError>
where
    F: Fn(&str) -> Result<T, String> + Clone + Send + 'static,
    T: Send + 'static,
{
    let ctx = get_asmr_ctx(state).await?;
    let token = ctx.token.clone();
    let f = fetch.clone();
    let first = tokio::task::spawn_blocking(move || f(&token))
        .await
        .map_err(AppError::new)?
        .map_err(AppError::new);
    match first {
        Ok(v) => Ok(v),
        Err(e) if e.0.contains("401") => {
            let new_token = asmr_relogin(
                state,
                &ctx.api_base,
                &ctx.username,
                &ctx.password,
                &ctx.proxy,
            )
            .await?;
            tokio::task::spawn_blocking(move || fetch(&new_token))
                .await
                .map_err(AppError::new)?
                .map_err(AppError::new)
        }
        Err(e) => Err(e.into()),
    }
}

/// 为在线条目标注本地已入库作品 id（按 RJ 号匹配，供前端显示已入库标识与跳转）。
async fn annotate_local(
    state: &SharedState,
    mut items: Vec<AsmrOnlineWork>,
) -> Result<Vec<AsmrOnlineWork>, AppError> {
    let state2 = state.clone();
    tokio::task::spawn_blocking(move || -> Result<Vec<AsmrOnlineWork>, AppError> {
        let conn = state2.db.lock().map_err(AppError::new)?;
        for it in items.iter_mut() {
            if let Some(rj) = &it.rj_code {
                if let Ok(Some(w)) = queries::get_work_by_rj(&conn, rj) {
                    it.local_work_id = Some(w.id);
                }
            }
        }
        Ok(items)
    })
    .await
    .map_err(AppError::new)?
}

#[derive(Deserialize)]
pub struct BrowseQuery {
    /// popular（热门，按下载量）| recommend（推荐，随机发现）| all（全部，可切排序）
    pub tab: Option<String>,
    /// all 下的排序：release（默认）| dl_count | rating
    pub order: Option<String>,
    pub page: Option<i64>,
}

/// GET /api/asmr/browse — 在线作品列表（热门 / 推荐 / 全部）。
pub async fn browse(
    State(state): State<SharedState>,
    Query(q): Query<BrowseQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let order = match q.tab.as_deref() {
        Some("recommend") => "random",
        Some("all") => match q.order.as_deref() {
            Some("dl_count") => "dl_count",
            Some("rating") => "rating",
            _ => "release",
        },
        _ => "dl_count", // popular
    };
    let page = q.page.unwrap_or(1).max(1);
    let ctx = get_asmr_ctx(&state).await?;
    let (api_base, proxy) = (ctx.api_base.clone(), ctx.proxy.clone());
    let page_data =
        run_with_relogin(&state, move |token: &str| {
            asmr_online::fetch_works_page(&api_base, token, &proxy, page, order)
        })
        .await?;
    let items = annotate_local(&state, page_data.items).await?;
    Ok(Json(json!({
        "type": "works",
        "items": items,
        "page": page_data.page,
        "hasNext": page_data.has_next,
    })))
}

#[derive(Deserialize)]
pub struct AccountQuery {
    /// favorite（收藏）| playlist（播放列表）| history（本地播放历史）
    pub tab: Option<String>,
    pub page: Option<i64>,
    /// playlist 的作品列表时传播放列表 id
    pub id: Option<String>,
}

/// GET /api/asmr/account — 账号相关数据（收藏 / 播放列表 / 历史）。
pub async fn account(
    State(state): State<SharedState>,
    Query(q): Query<AccountQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let page = q.page.unwrap_or(1).max(1);
    match q.tab.as_deref().unwrap_or("favorite") {
        "playlist" => {
            let ctx = get_asmr_ctx(&state).await?;
            let (api_base, proxy) = (ctx.api_base.clone(), ctx.proxy.clone());
            match q.id.filter(|s| !s.trim().is_empty()) {
                Some(id) => {
                    let page_data =
                        run_with_relogin(&state, move |token: &str| {
                            asmr_online::fetch_playlist_works(&api_base, token, &proxy, id.trim(), page)
                        })
                        .await?;
                    let items = annotate_local(&state, page_data.items).await?;
                    Ok(Json(json!({
                        "type": "works",
                        "items": items,
                        "page": page_data.page,
                        "hasNext": page_data.has_next,
                    })))
                }
                None => {
                    let playlists = run_with_relogin(&state, move |token: &str| {
                        asmr_online::fetch_playlists(&api_base, token, &proxy)
                    })
                    .await?;
                    Ok(Json(json!({ "type": "playlists", "playlists": playlists })))
                }
            }
        }
        "history" => {
            // asmr.one 无公开云端历史 API（实测 404），使用本应用的播放记录
            let (works, has_next) = tokio::task::spawn_blocking(move || -> Result<_, AppError> {
                let conn = state.db.lock().map_err(AppError::new)?;
                queries::list_play_history_works(&conn, page, PAGE_SIZE).map_err(AppError::new)
            })
            .await
            .map_err(AppError::new)??;
            let items: Vec<AsmrOnlineWork> = works.into_iter().map(work_to_online).collect();
            Ok(Json(json!({
                "type": "works",
                "items": items,
                "page": page,
                "hasNext": has_next,
            })))
        }
        _ => {
            // 收藏（asmr.one 的 review，即评分收藏的作品）
            let ctx = get_asmr_ctx(&state).await?;
            let (api_base, proxy) = (ctx.api_base.clone(), ctx.proxy.clone());
            let page_data =
                run_with_relogin(&state, move |token: &str| {
                    asmr_online::fetch_favorites(&api_base, token, &proxy, page)
                })
                .await?;
            let items = annotate_local(&state, page_data.items).await?;
            Ok(Json(json!({
                "type": "works",
                "items": items,
                "page": page_data.page,
                "hasNext": page_data.has_next,
            })))
        }
    }
}

/// 本地作品 → 在线条目结构（播放历史展示用）。
fn work_to_online(w: Work) -> AsmrOnlineWork {
    AsmrOnlineWork {
        id: 0,
        rj_code: Some(w.rj_code.clone()),
        title: w.title_zh.clone().filter(|s| !s.is_empty()).unwrap_or_else(|| {
            w.title_ja.clone().unwrap_or_else(|| w.rj_code.clone())
        }),
        circle_name: w.circle_name.clone(),
        cover_url: w.cover_url.clone(),
        release: w.sale_date.clone(),
        dl_count: None,
        price: w.price,
        rating: None,
        rate_count: None,
        duration_sec: w.duration_min.map(|m| m * 60),
        vas: Vec::new(),
        tags: Vec::new(),
        description: None,
        local_work_id: Some(w.id),
    }
}

/// GET /api/asmr/work/{id} — 在线作品详情。
pub async fn work_detail(
    State(state): State<SharedState>,
    Path(id): Path<i64>,
) -> Result<Json<AsmrOnlineWork>, AppError> {
    let ctx = get_asmr_ctx(&state).await?;
    let (api_base, proxy) = (ctx.api_base.clone(), ctx.proxy.clone());
    let detail = run_with_relogin(&state, move |token: &str| {
        asmr_online::fetch_work_detail(&api_base, token, &proxy, id)
    })
    .await?;
    let items = annotate_local(&state, vec![detail]).await?;
    Ok(Json(items.into_iter().next().unwrap_or_default()))
}
