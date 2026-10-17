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

/// 为在线条目标注收藏状态：列表接口不返回 progress/userRating，纯收藏（仅打
/// "想听"标记不打分）在列表响应里无从判定，用全量收藏 id 集合（短 TTL 缓存）补标。
/// 已带 user_marked=true 的条目不动（详情/收藏列表接口本身携带真实状态）。
async fn annotate_marked(
    state: &SharedState,
    mut items: Vec<AsmrOnlineWork>,
) -> Result<Vec<AsmrOnlineWork>, AppError> {
    let any_unknown = items.iter().any(|it| !it.user_marked);
    if !any_unknown {
        return Ok(items);
    }
    let ctx = get_asmr_ctx(state).await?;
    let (api_base, proxy, token) = (ctx.api_base.clone(), ctx.proxy.clone(), ctx.token.clone());
    let ids = crate::api::cache::asmrcache::asmr_favorite_ids()
        .get_or_fetch("all", move || async move {
            tokio::task::spawn_blocking(move || {
                asmr_online::fetch_favorite_ids(&api_base, &token, &proxy)
            })
            .await
            .map_err(|e| e.to_string())?
        })
        .await
        .map_err(AppError::new)?;
    for it in items.iter_mut() {
        if !it.user_marked && ids.contains(&it.id) {
            it.user_marked = true;
        }
    }
    Ok(items)
}

/// 为在线条目标注本地已入库作品 id（按 RJ 号匹配，供前端显示已入库标识与跳转）。
/// dlsite latest 等其他在线列表也复用。
pub async fn annotate_local_pub(
    state: &SharedState,
    items: Vec<AsmrOnlineWork>,
) -> Result<Vec<AsmrOnlineWork>, AppError> {
    annotate_local(state, items).await
}

async fn annotate_local(
    state: &SharedState,
    mut items: Vec<AsmrOnlineWork>,
) -> Result<Vec<AsmrOnlineWork>, AppError> {
    let state2 = state.clone();
    tokio::task::spawn_blocking(move || -> Result<Vec<AsmrOnlineWork>, AppError> {
        let conn = state2.db.get().map_err(AppError::new)?;
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

    // recommend 是随机发现，缓存会把随机结果冻住，直接回源；其余排序读穿
    // 内存 5 分钟 → web_cache（1 小时新鲜度）→ asmr.one 回源写回，
    // 回源失败回退 DB 旧快照。缓存的是 asmr.one 原始列表，
    // local_work_id / 收藏标注在后面每次重算，不受缓存影响。
    const BROWSE_MAX_AGE_SECS: i64 = 3600;
    let page_data = if order == "random" {
        run_with_relogin(&state, {
            let api_base = ctx.api_base.clone();
            let proxy = ctx.proxy.clone();
            move |token: &str| asmr_online::fetch_works_page(&api_base, token, &proxy, page, "random")
        })
        .await?
    } else {
        let cache_key = format!("asmr:browse:{order}:p{page}");
        crate::api::cache::dlsitecache::browse_page()
            .get_or_fetch(&format!("order:{order}:page:{page}"), {
                let state = state.clone();
                let api_base = ctx.api_base.clone();
                let proxy = ctx.proxy.clone();
                let cache_key = cache_key.clone();
                move || async move {
                    // 内存未命中 → 查 DB（1 小时内的条目才算命中）
                    let from_db = tokio::task::spawn_blocking({
                        let state = state.clone();
                        let cache_key = cache_key.clone();
                        move || -> Option<asmr_online::AsmrWorksPage> {
                            let conn = state.db.get().ok()?;
                            queries::web_cache_get_fresh(&conn, &cache_key, BROWSE_MAX_AGE_SECS)
                                .ok()
                                .flatten()
                        }
                    })
                    .await
                    .map_err(|e| e.to_string())?;
                    if let Some(p) = from_db {
                        return Ok(p);
                    }
                    // DB 没有（或超过 1 小时新鲜度）→ asmr.one 回源，成功后写回 DB；
                    // 回源失败时回退返回 DB 里的旧快照（如有），避免整页报错
                    let fetched = run_with_relogin(&state, {
                        let api_base = api_base.clone();
                        let proxy = proxy.clone();
                        move |token: &str| {
                            asmr_online::fetch_works_page(&api_base, token, &proxy, page, order)
                        }
                    })
                    .await;
                    match &fetched {
                        Ok(p) => {
                            let _ = tokio::task::spawn_blocking({
                                let state = state.clone();
                                let cache_key = cache_key.clone();
                                let p = p.clone();
                                move || {
                                    if let Ok(conn) = state.db.get() {
                                        let _ = queries::web_cache_put(&conn, &cache_key, &p);
                                    }
                                }
                            })
                            .await;
                        }
                        Err(_) => {
                            let stale = tokio::task::spawn_blocking({
                                let state = state.clone();
                                let cache_key = cache_key.clone();
                                move || -> Option<asmr_online::AsmrWorksPage> {
                                    let conn = state.db.get().ok()?;
                                    queries::web_cache_get(&conn, &cache_key).ok().flatten()
                                }
                            })
                            .await
                            .ok()
                            .flatten();
                            if let Some(p) = stale {
                                return Ok(p);
                            }
                        }
                    }
                    fetched.map_err(|e| e.0)
                }
            })
            .await
            .map_err(AppError::new)?
    };

    let items = annotate_local(&state, page_data.items).await?;
    let items = annotate_marked(&state, items).await?;
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
                let conn = state.db.get().map_err(AppError::new)?;
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
        rating: w.rating,
        rate_count: None,
        duration_sec: w.duration_min.map(|m| m * 60),
        age_class: w.age_class,
        vas: Vec::new(),
        tags: Vec::new(),
        has_subtitle: false,
        description: None,
        local_work_id: Some(w.id),
        user_marked: false,
    }
}

/// GET /api/asmr/work/{id} — 在线作品详情。
/// 短 TTL 缓存：详情页反复浏览不再重复打外网（0.5s/次 → 命中后毫秒级）。
pub async fn work_detail(
    State(state): State<SharedState>,
    Path(id): Path<i64>,
) -> Result<Json<AsmrOnlineWork>, AppError> {
    let ctx = get_asmr_ctx(&state).await?;
    let (api_base, proxy, token) = (ctx.api_base.clone(), ctx.proxy.clone(), ctx.token.clone());
    let detail = crate::api::cache::asmrcache::work_detail()
        .get_or_fetch(&format!("work:{}:{id}", ctx.api_base), move || async move {
            tokio::task::spawn_blocking(move || {
                asmr_online::fetch_work_detail(&api_base, &token, &proxy, id)
            })
            .await
            .map_err(|e| e.to_string())?
        })
        .await
        .map_err(AppError::new)?;
    let items = annotate_local(&state, vec![detail]).await?;
    let items = annotate_marked(&state, items).await?;
    Ok(Json(items.into_iter().next().unwrap_or_default()))
}

#[derive(Deserialize)]
pub struct FavoriteBody {
    /// asmr.one 作品数字 id
    pub id: i64,
    /// true=收藏（marked），false=取消收藏
    pub favorite: bool,
}

/// POST /api/asmr/favorite — 同步收藏到 asmr.one 账号（PUT / DELETE /api/review）。
pub async fn set_favorite(
    State(state): State<SharedState>,
    Json(body): Json<FavoriteBody>,
) -> Result<Json<serde_json::Value>, AppError> {
    let ctx = get_asmr_ctx(&state).await?;
    let (api_base, proxy) = (ctx.api_base.clone(), ctx.proxy.clone());
    let fav = body.favorite;
    let id = body.id;
    let message = run_with_relogin(&state, move |token: &str| {
        if fav {
            asmr_online::add_favorite(&api_base, token, &proxy, id)
        } else {
            asmr_online::remove_favorite(&api_base, token, &proxy, id)
        }
    })
    .await?;
    // 写穿：收藏集合已变化，清掉缓存让下一次列表标注拿到新状态
    crate::api::cache::asmrcache::asmr_favorite_ids().invalidate("all");
    Ok(Json(json!({ "ok": true, "message": message })))
}
