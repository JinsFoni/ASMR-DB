use std::sync::Mutex;
use std::time::{Duration, Instant};

use axum::extract::{Path, Query, State};
use axum::Json;
use rusqlite::params;
use rusqlite::OptionalExtension;
use serde::Deserialize;
use serde_json::json;

use crate::api::llm::{self, LlmConfig};
use crate::db::queries;
use crate::state::{AppError, SharedState};

pub const PER_PAGE: i64 = 10;
// ============================= 全局限速 =============================

/// 全局限速状态：任意两次 LLM 请求之间至少间隔 1000/rps 毫秒。
static LAST_REQUEST: Mutex<Option<Instant>> = Mutex::new(None);

async fn acquire_rate_slot(cfg: &LlmConfig) {
    let min_gap = Duration::from_millis((1000 / cfg.rps.max(1)) as u64);
    loop {
        let wait = {
            let mut last = LAST_REQUEST
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            let now = Instant::now();
            let elapsed = now.duration_since(last.unwrap_or(now - min_gap));
            if elapsed >= min_gap {
                *last = Some(now);
                Duration::ZERO
            } else {
                min_gap - elapsed
            }
        };
        if wait.is_zero() {
            return;
        }
        tokio::time::sleep(wait).await;
    }
}

// ============================= 调度器 =============================

/// 启动翻译调度器：每秒检查一次，按线程数认领排队任务并发执行。
pub fn spawn_scheduler(state: SharedState) {
    tokio::spawn(async move {
        loop {
            let cfg = tokio::task::spawn_blocking({
                let state = state.clone();
                move || match state.db.lock() {
                    Ok(conn) => LlmConfig::from_conn(&conn),
                    Err(_) => LlmConfig::default(),
                }
            })
            .await
            .unwrap_or_default();

            if cfg.ready() {
                let processing = match tokio::task::spawn_blocking({
                    let state = state.clone();
                    move || -> Result<i64, AppError> {
                        let conn = state.db.lock().map_err(AppError::new)?;
                        queries::count_processing_translations(&conn).map_err(AppError::new)
                    }
                })
                .await
                {
                    Ok(Ok(n)) => n,
                    _ => i64::MAX, // 统计失败时本轮不认领
                };

                let free = cfg.threads - processing;
                if free > 0 {
                    let claimed = match tokio::task::spawn_blocking({
                        let state = state.clone();
                        move || -> Result<Vec<(i64, i64)>, AppError> {
                            let conn = state.db.lock().map_err(AppError::new)?;
                            queries::claim_pending_translations(&conn, free)
                                .map_err(AppError::new)
                        }
                    })
                    .await
                    {
                        Ok(Ok(v)) => v,
                        _ => Vec::new(),
                    };

                    for (task_id, work_id) in claimed {
                        let state = state.clone();
                        let cfg = cfg.clone();
                        tokio::spawn(async move {
                            process_task(state, cfg, task_id, work_id).await;
                        });
                    }
                }
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    });
}

/// 执行单个翻译任务：标题必翻，简介有则翻；按最大重试次数自动重试。
async fn process_task(state: SharedState, cfg: LlmConfig, task_id: i64, work_id: i64) {
    let st = state.clone();
    let work = tokio::task::spawn_blocking(move || -> Result<(String, Option<String>), AppError> {
        let conn = st.db.lock().map_err(AppError::new)?;
        let view = queries::get_work(&conn, work_id)?
            .ok_or_else(|| AppError("作品不存在".to_string()))?;
        let source_title = view
            .work
            .title_ja
            .clone()
            .filter(|s| !s.trim().is_empty())
            .ok_or_else(|| AppError("作品没有日文标题，无需翻译".to_string()))?;
        let source_desc = view.work.description.clone().filter(|s| !s.trim().is_empty());
        Ok((source_title, source_desc))
    })
    .await;

    let (source_title, source_desc) = match work {
        Ok(Ok(v)) => v,
        Ok(Err(e)) => {
            fail_task(&state, task_id, &e.0).await;
            return;
        }
        Err(e) => {
            eprintln!("⚠️ 翻译任务 {task_id}: {e}");
            return;
        }
    };

    // 翻译标题（含自动重试）
    let mut attempts = cfg.max_retry + 1;
    let mut translated_title: Option<String> = None;
    let mut last_err = String::new();
    while attempts > 0 {
        acquire_rate_slot(&cfg).await;
        let r = tokio::task::spawn_blocking({
            let cfg = cfg.clone();
            let text = source_title.clone();
            move || llm::translate(&cfg, &text)
        })
        .await
        .map_err(|e| format!("{e}"))
        .and_then(|r| r);
        match r {
            Ok(t) => {
                translated_title = Some(t);
                break;
            }
            Err(e) => {
                last_err = e;
                attempts -= 1;
                if attempts > 0 {
                    tokio::time::sleep(Duration::from_secs(2)).await;
                }
            }
        }
    }

    let Some(translated_title) = translated_title else {
        fail_task(&state, task_id, &format!("标题翻译失败：{last_err}")).await;
        return;
    };

    // 翻译简介（若有）
    let mut translated_desc: Option<String> = None;
    if let Some(desc) = &source_desc {
        let mut attempts = cfg.max_retry + 1;
        while attempts > 0 {
            acquire_rate_slot(&cfg).await;
            let r = tokio::task::spawn_blocking({
                let cfg = cfg.clone();
                let text = desc.clone();
                move || llm::translate(&cfg, &text)
            })
            .await
            .map_err(|e| format!("{e}"))
            .and_then(|r| r);
            match r {
                Ok(t) => {
                    translated_desc = Some(t);
                    break;
                }
                Err(e) => {
                    last_err = e;
                    attempts -= 1;
                    if attempts > 0 {
                        tokio::time::sleep(Duration::from_secs(2)).await;
                    }
                }
            }
        }
        if translated_desc.is_none() {
            fail_task(
                &state,
                task_id,
                &format!("简介翻译失败（标题已翻译）：{last_err}"),
            )
            .await;
            return;
        }
    }

    // 落库：写入作品中文字段 + 任务成功
    let result = tokio::task::spawn_blocking(move || -> Result<(), AppError> {
        let conn = state.db.lock().map_err(AppError::new)?;
        queries::set_work_translation(
            &conn,
            work_id,
            Some(&translated_title),
            translated_desc.as_deref(),
        )
        .map_err(AppError::new)?;
        conn.execute(
            r#"UPDATE translation_tasks SET status='success',
               translated_title=?1, translated_desc=?2, error=NULL, updated_at=datetime('now')
               WHERE id=?3"#,
            task_result_params(&translated_title, translated_desc.as_deref(), task_id),
        )
        .map_err(AppError::new)?;
        Ok(())
    })
    .await
    .map_err(|e| eprintln!("⚠️ 翻译任务 {task_id}: {e}"))
    .ok();

    match result {
        Some(Ok(())) => println!("🌍 作品 {work_id} 翻译完成"),
        Some(Err(e)) => eprintln!("⚠️ 翻译任务 {task_id} 保存失败: {e}"),
        _ => {}
    }
}

async fn fail_task(state: &SharedState, task_id: i64, error: &str) {
    eprintln!("⚠️ 翻译任务 {task_id} 失败: {error}");
    let state = state.clone();
    let error = error.to_string();
    let _ = tokio::task::spawn_blocking(move || {
        let conn = state.db.lock().map_err(AppError::new)?;
        conn.execute(
            "UPDATE translation_tasks SET status='failed', error=?1, updated_at=datetime('now') WHERE id=?2",
            task_error_params(&error, task_id),
        )
        .map_err(AppError::new)
    })
    .await;
}

fn task_result_params<'a>(
    t: &'a str,
    d: Option<&'a str>,
    id: i64,
) -> impl rusqlite::Params + 'a {
    (t, d, id)
}

fn task_error_params<'a>(e: &'a str, id: i64) -> impl rusqlite::Params + 'a {
    (e, id)
}

// ============================= API 端点 =============================

#[derive(Deserialize)]
pub struct ListQuery {
    pub page: Option<i64>,
}

/// GET /api/translations?page=1 — 翻译任务列表（每页 10 条）。
pub async fn list_translations(
    State(state): State<SharedState>,
    Query(q): Query<ListQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let page = q.page.unwrap_or(1).max(1);
    let (items, total) = tokio::task::spawn_blocking(move || -> Result<_, AppError> {
        let conn = state.db.lock().map_err(AppError::new)?;
        queries::list_translation_tasks(&conn, page, PER_PAGE).map_err(AppError::new)
    })
    .await
    .map_err(AppError::new)??;
    let total_pages = (total + PER_PAGE - 1) / PER_PAGE;
    Ok(Json(json!({
        "items": items,
        "page": page,
        "total": total,
        "totalPages": total_pages.max(1),
    })))
}

#[derive(Deserialize)]
pub struct EnqueueBody {
    pub work_id: i64,
}

/// POST /api/translations/enqueue — 单作品入队（详情页按钮）。
pub async fn enqueue(
    State(state): State<SharedState>,
    Json(body): Json<EnqueueBody>,
) -> Result<Json<serde_json::Value>, AppError> {
    let queued = tokio::task::spawn_blocking(move || -> Result<bool, AppError> {
        let conn = state.db.lock().map_err(AppError::new)?;
        let exists = conn
            .query_row(
                "SELECT 1 FROM works WHERE id=?1",
                params![body.work_id],
                |_| Ok(()),
            )
            .optional()?;
        if exists.is_none() {
            return Ok(false);
        }
        queries::enqueue_translation(&conn, body.work_id).map_err(AppError::new)?;
        Ok(true)
    })
    .await
    .map_err(AppError::new)??;
    if queued {
        Ok(Json(json!({ "ok": true, "message": "已加入翻译队列" })))
    } else {
        Err(AppError("作品不存在".to_string()))
    }
}

/// POST /api/translations/enqueue-missing — 批量入队全部未翻译作品。
pub async fn enqueue_missing(
    State(state): State<SharedState>,
) -> Result<Json<serde_json::Value>, AppError> {
    let n = tokio::task::spawn_blocking(move || -> Result<usize, AppError> {
        let conn = state.db.lock().map_err(AppError::new)?;
        queries::enqueue_missing_translations(&conn).map_err(AppError::new)
    })
    .await
    .map_err(AppError::new)??;
    Ok(Json(json!({ "ok": true, "queued": n })))
}

/// POST /api/translations/{id}/retry — 重试任务。
pub async fn retry(
    State(state): State<SharedState>,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, AppError> {
    let ok = tokio::task::spawn_blocking(move || -> Result<bool, AppError> {
        let conn = state.db.lock().map_err(AppError::new)?;
        queries::retry_translation(&conn, id).map_err(AppError::new)
    })
    .await
    .map_err(AppError::new)??;
    if ok {
        Ok(Json(json!({ "ok": true })))
    } else {
        Err(AppError("任务不存在".to_string()))
    }
}

/// DELETE /api/translations/{id} — 删除任务记录（不清除已写入的译文）。
pub async fn delete_task(
    State(state): State<SharedState>,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, AppError> {
    let ok = tokio::task::spawn_blocking(move || -> Result<bool, AppError> {
        let conn = state.db.lock().map_err(AppError::new)?;
        queries::delete_translation(&conn, id).map_err(AppError::new)
    })
    .await
    .map_err(AppError::new)??;
    if ok {
        Ok(Json(json!({ "ok": true })))
    } else {
        Err(AppError("任务不存在".to_string()))
    }
}
