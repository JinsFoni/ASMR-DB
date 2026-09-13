use std::time::Duration;

use axum::body::Body;
use axum::extract::{Path, Query, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use tokio::io::AsyncReadExt;

use crate::db::queries;
use crate::state::{AppError, SharedState};

// ============================= 本地音频流 =============================

#[derive(serde::Deserialize)]
pub struct AudioQuery {
    pub path: String,
}

/// 以 HTTP Range 方式伺服本地音频文件，使浏览器 <audio> / Howler 能 seek。
pub async fn local_audio(
    Query(q): Query<AudioQuery>,
    headers: HeaderMap,
) -> Response {
    let p = std::path::PathBuf::from(&q.path);
    if !p.is_file() {
        return (StatusCode::NOT_FOUND, "file not found").into_response();
    }
    let size = match tokio::fs::metadata(&p).await {
        Ok(m) => m.len(),
        Err(_) => return (StatusCode::NOT_FOUND, "file not found").into_response(),
    };
    let mime = mime_guess::from_path(&p).first_or_octet_stream().to_string();

    let range_spec = headers
        .get(header::RANGE)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let (start, end, status) = match range_spec.as_deref().and_then(|s| parse_range(s, size)) {
        Some((s, e)) => {
            if s >= size {
                return Response::builder()
                    .status(StatusCode::RANGE_NOT_SATISFIABLE)
                    .header(header::CONTENT_RANGE, format!("bytes */{size}"))
                    .body(Body::empty())
                    .unwrap();
            }
            (s, e, StatusCode::PARTIAL_CONTENT)
        }
        None => (0u64, size.saturating_sub(1), StatusCode::OK),
    };

    let mut file = match tokio::fs::File::open(&p).await {
        Ok(f) => f,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("open failed: {e}")).into_response(),
    };
    if start > 0 {
        use tokio::io::AsyncSeekExt;
        if let Err(e) = file.seek(std::io::SeekFrom::Start(start)).await {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("seek failed: {e}")).into_response();
        }
    }

    let mut builder = Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, mime)
        .header(header::ACCEPT_RANGES, "bytes")
        .header(header::CONTENT_LENGTH, (end - start + 1).to_string());
    if status == StatusCode::PARTIAL_CONTENT {
        builder = builder.header(header::CONTENT_RANGE, format!("bytes {start}-{end}/{size}"));
    }

    // take() 限制读取长度，处理 bytes=a-b 的闭合区间
    let total_len = end - start + 1;
    let reader = tokio_util::io::ReaderStream::with_capacity(file.take(total_len), 128 * 1024);
    builder
        .body(Body::from_stream(reader))
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}

/// 解析单区间 Range 头：bytes=a-b / bytes=a- / bytes=-suffix。
fn parse_range(spec: &str, size: u64) -> Option<(u64, u64)> {
    let spec = spec.strip_prefix("bytes=")?.trim();
    let (s, e) = spec.split_once('-')?;
    if s.is_empty() {
        let n: u64 = e.trim().parse().ok()?;
        if size == 0 {
            return Some((0, 0));
        }
        let start = size.saturating_sub(n);
        return Some((start, size - 1));
    }
    let start: u64 = s.trim().parse().ok()?;
    let end = if e.trim().is_empty() {
        size.saturating_sub(1)
    } else {
        e.trim().parse::<u64>().ok()?.min(size.saturating_sub(1))
    };
    Some((start, end))
}

// ============================= asmr.one 文件代理 =============================

/// 代理 asmr.one 文件流：注入 Bearer token、透传 Range（206/Content-Range 原样返回），
/// 使浏览器 <audio> 能加载与 seek，token 不暴露给前端。
pub async fn asmr_file(
    State(state): State<SharedState>,
    Path(hash): Path<String>,
    headers: HeaderMap,
) -> Response {
    let hash = hash.trim().trim_start_matches('/').to_string();
    if hash.is_empty() {
        return (StatusCode::BAD_REQUEST, "bad file hash").into_response();
    }

    // 从 DB 读 asmr.one token
    let token = {
        let conn = state.db.lock().map_err(AppError::new);
        match conn {
            Ok(c) => queries::get_setting(&c, "asmr_one_token")
                .ok()
                .flatten()
                .unwrap_or_default(),
            Err(_) => String::new(),
        }
    };

    let client = match reqwest::Client::builder()
        .user_agent(crate::api::asmrone::USER_AGENT)
        .timeout(Duration::from_secs(60))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            return (StatusCode::BAD_GATEWAY, format!("client build failed: {e}")).into_response()
        }
    };

    let file_id = hash.rsplit('/').next().unwrap_or(&hash).to_string();

    let do_fetch = |client: &reqwest::Client, url: &str| -> reqwest::RequestBuilder {
        let mut req = client.get(url);
        if !token.trim().is_empty() {
            req = req.header(header::AUTHORIZATION, format!("Bearer {}", token.trim()));
        }
        if let Some(range) = headers.get(header::RANGE) {
            req = req.header(header::RANGE, range);
        }
        req
    };

    // 先试完整 hash；若 404 再退回纯 fileId（两种 hash 形态都覆盖）
    let url_hash = format!("https://api.asmr-100.com/api/file/{}", hash);
    let url_fid = format!("https://api.asmr-100.com/api/file/{}", file_id);

    let resp = match do_fetch(&client, &url_hash).send().await {
        Ok(r) if r.status() == reqwest::StatusCode::NOT_FOUND && file_id != hash => {
            match do_fetch(&client, &url_fid).send().await {
                Ok(r2) => r2,
                Err(e) => {
                    return (
                        StatusCode::BAD_GATEWAY,
                        format!("asmr.one 请求失败: {e}"),
                    )
                        .into_response()
                }
            }
        }
        Ok(r) => r,
        Err(e) => {
            return (
                StatusCode::BAD_GATEWAY,
                format!("asmr.one 请求失败: {e}"),
            )
                .into_response()
        }
    };

    // 透传状态码与流式/缓存相关 header，body 原样流式返回
    let status =
        StatusCode::from_u16(resp.status().as_u16()).unwrap_or(StatusCode::OK);
    let mut builder = Response::builder().status(status);
    for key in &[
        header::CONTENT_TYPE,
        header::CONTENT_LENGTH,
        header::CONTENT_RANGE,
        header::ACCEPT_RANGES,
        header::LAST_MODIFIED,
        header::ETAG,
    ] {
        if let Some(val) = resp.headers().get(key) {
            builder = builder.header(key, val);
        }
    }
    let stream = resp.bytes_stream();
    builder
        .body(Body::from_stream(stream))
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}
