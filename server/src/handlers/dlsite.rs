use axum::extract::{Query, State};
use axum::Json;
use serde::Deserialize;
use serde_json::json;

use crate::api::proxy::ProxyConfig;
use crate::api::ranking;
use crate::db::queries;
use crate::state::{AppError, SharedState};

/// 兼容常见别名单词，统一映射为 DLsite 的周期路径段。
fn normalize_term(term: Option<&str>) -> String {
    match term.map(|t| t.trim().to_lowercase()).as_deref() {
        Some("day") | Some("daily") => "day",
        Some("week") | Some("weekly") => "week",
        Some("month") | Some("monthly") => "month",
        Some("year") | Some("yearly") => "year",
        Some("total") => "total",
        _ => "day",
    }
    .to_string()
}

#[derive(Deserialize)]
pub struct RankingQuery {
    pub term: Option<String>,
    pub limit: Option<i64>,
}

/// GET /api/dlsite/ranking?term=day&limit=20
/// 返回榜单条目（含已入库状态）与最近抓取时间。
pub async fn get_ranking(
    State(state): State<SharedState>,
    Query(q): Query<RankingQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let term = normalize_term(q.term.as_deref());
    let limit = q.limit.unwrap_or(20).clamp(1, 100);
    let (items, last_fetched) = tokio::task::spawn_blocking(
        move || -> Result<(Vec<_>, Option<String>), AppError> {
            let conn = state.db.lock().map_err(AppError::new)?;
            let items = queries::list_rankings(&conn, &term, limit)?;
            let last = queries::ranking_last_fetch(&conn, &term)?;
            Ok((items, last))
        },
    )
    .await
    .map_err(AppError::new)??;

    Ok(Json(json!({ "items": items, "lastFetched": last_fetched })))
}

#[derive(Deserialize)]
pub struct RefreshBody {
    /// 不传或为空时刷新全部 5 个周期
    pub term: Option<String>,
}

/// POST /api/dlsite/ranking/refresh — 手动触发抓取（同步完成后返回）。
pub async fn refresh_ranking(
    State(state): State<SharedState>,
    body: Option<Json<RefreshBody>>,
) -> Result<Json<serde_json::Value>, AppError> {
    let terms: Vec<String> = match body.and_then(|Json(b)| b.term) {
        Some(t) if !t.trim().is_empty() => vec![normalize_term(Some(&t))],
        _ => ranking::RANKING_TERMS.iter().map(|s| s.to_string()).collect(),
    };
    let refreshed = do_refresh(state, terms).await?;
    Ok(Json(json!({ "ok": true, "refreshed": refreshed })))
}

/// 逐个抓取指定周期并落库；单个周期失败只记日志，不中断其余周期。
pub async fn do_refresh(
    state: SharedState,
    terms: Vec<String>,
) -> Result<Vec<String>, AppError> {
    // 抓取前读取一次代理配置，本次刷新全程使用
    let proxy = tokio::task::spawn_blocking({
        let state = state.clone();
        move || -> Result<ProxyConfig, AppError> {
            let conn = state.db.lock().map_err(AppError::new)?;
            Ok(ProxyConfig::from_conn(&conn))
        }
    })
    .await
    .map_err(AppError::new)??;

    let mut refreshed = Vec::new();
    for (i, term) in terms.iter().enumerate() {
        let entries = tokio::task::spawn_blocking({
            let term = term.clone();
            let proxy = proxy.clone();
            move || ranking::fetch_ranking(&term, &proxy)
        })
        .await
        .map_err(AppError::new)?;

        match entries {
            Ok(entries) => {
                let count = entries.len();
                let db_term = term.clone();
                tokio::task::spawn_blocking({
                    let state = state.clone();
                    move || -> Result<(), AppError> {
                        let conn = state.db.lock().map_err(AppError::new)?;
                        queries::replace_rankings(&conn, &db_term, &entries)?;
                        Ok(())
                    }
                })
                .await
                .map_err(AppError::new)??;
                refreshed.push(term.clone());
                println!("🏆 DLsite 榜单 [{term}] 已更新 {count} 条");
            }
            Err(e) => eprintln!("⚠️ DLsite 榜单 [{term}] 抓取失败: {e}"),
        }
        // 抓取多个周期时礼貌性间隔，避免连续请求
        if terms.len() > 1 && i + 1 < terms.len() {
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }
    }
    Ok(refreshed)
}
