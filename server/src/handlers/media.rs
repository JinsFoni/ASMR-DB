use std::time::Duration;

use axum::body::Body;
use axum::extract::{Path, Query, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use tokio::io::AsyncReadExt;

use crate::db::queries;
use crate::state::SharedState;

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

    // 从 DB 读 asmr.one token、代理配置与 API 地址（spawn_blocking：池满时 pool.get()
    // 会阻塞，不能卡在 async executor 线程上）
    let (token, proxy, api_base) = {
        let state = state.clone();
        tokio::task::spawn_blocking(move || {
            let conn = state.db.get();
            match conn {
                Ok(c) => (
                    queries::get_setting(&c, "asmr_one_token")
                        .ok()
                        .flatten()
                        .unwrap_or_default(),
                    crate::api::proxy::ProxyConfig::from_conn(&c),
                    crate::api::asmrone::normalize_api_base(
                        queries::get_setting(&c, "asmr_one_address").ok().flatten().as_deref(),
                    ),
                ),
                Err(_) => (String::new(), Default::default(), crate::api::asmrone::DEFAULT_API_BASE.to_string()),
            }
        })
        .await
        .unwrap_or_else(|_| {
            (String::new(), Default::default(), crate::api::asmrone::DEFAULT_API_BASE.to_string())
        })
    };

    let builder = reqwest::Client::builder()
        .user_agent(crate::api::asmrone::USER_AGENT)
        .timeout(Duration::from_secs(60));
    let client = match proxy
        .apply_async(builder, crate::api::proxy::ProxyTarget::AsmrOne)
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
    let url_hash = format!("{api_base}/api/file/{hash}");
    let url_fid = format!("{api_base}/api/file/{file_id}");

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

// ============================= 图片缓存代理 =============================

#[derive(serde::Deserialize)]
pub struct ImgQuery {
    pub url: String,
}

/// GET /api/img?url=... — 远端图片代理：结果落盘缓存（大小/TTL 由设置表控制）。
/// 前端封面统一走该端点，重复浏览不再重复回源外网。
pub async fn img_proxy(State(state): State<SharedState>, Query(q): Query<ImgQuery>) -> Response {
    let url = q.url.trim().to_string();
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return (StatusCode::BAD_REQUEST, "bad image url").into_response();
    }
    if url.contains("localhost") || url.contains("127.0.0.1") {
        // 防止把本机服务当图源（SSRF 面）
        return (StatusCode::BAD_REQUEST, "bad image url").into_response();
    }

    // 设置项 + 缓存目录（spawn_blocking：DB 池与文件 IO 都不能占 async 线程）
    let state2 = state.clone();
    let lookup_url = url.clone();
    let (cache_cfg, cached) =
        tokio::task::spawn_blocking(move || -> ((u64, std::time::Duration), Option<(Vec<u8>, String)>) {
            let (max_mb, ttl) = match state2.db.get() {
                Ok(conn) => crate::api::img_cache::config_from(&conn),
                Err(_) => (0, std::time::Duration::from_secs(3600)),
            };
            let dir = crate::api::img_cache::cache_dir(&state2.db_path);
            let hit = if max_mb == 0 {
                None
            } else {
                crate::api::img_cache::get(&dir, &lookup_url, ttl)
            };
            ((max_mb, ttl), hit)
        })
        .await
        .unwrap_or(((0, std::time::Duration::from_secs(3600)), None));

    let (max_mb, ttl) = cache_cfg;
    if let Some((bytes, mime)) = cached {
        return (
            StatusCode::OK,
            [
                (header::CONTENT_TYPE, mime),
                (header::CACHE_CONTROL, "max-age=3600".to_string()),
            ],
            Body::from(bytes),
        )
            .into_response();
    }
    if max_mb == 0 {
        // 缓存关闭：直接回源，不落盘
        return match fetch_remote_image(&url, &state).await {
            Ok((bytes, mime)) => (
                StatusCode::OK,
                [(header::CONTENT_TYPE, mime)],
                Body::from(bytes),
            )
                .into_response(),
            Err(e) => (StatusCode::BAD_GATEWAY, e).into_response(),
        };
    }

    // 回源并整体读入（封面几百 KB 量级，可接受；失败时无缓存直通）
    let (bytes, mime) = match fetch_remote_image(&url, &state).await {
        Ok(v) => v,
        Err(e) => return (StatusCode::BAD_GATEWAY, e).into_response(),
    };
    let dir = crate::api::img_cache::cache_dir(&state.db_path);
    let store_url = url.clone();
    let store_bytes = bytes.clone();
    let store_mime = mime.clone();
    let _ = tokio::task::spawn_blocking(move || {
        crate::api::img_cache::put(&dir, &store_url, &store_bytes, &store_mime, max_mb, ttl);
    })
    .await;
    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, mime),
            (header::CACHE_CONTROL, "max-age=3600".to_string()),
        ],
        Body::from(bytes),
    )
        .into_response()
}

/// 拉取远端图片为字节（上限 16 MB，防御异常大文件）。
async fn fetch_remote_image(url: &str, state: &SharedState) -> Result<(Vec<u8>, String), String> {
    let proxy = {
        let st = state.clone();
        tokio::task::spawn_blocking(move || {
            st.db.get().ok().map_or_else(Default::default, |conn| {
                crate::api::proxy::ProxyConfig::from_conn(&conn)
            })
        })
        .await
        .unwrap_or_default()
    };
    let builder = reqwest::Client::builder()
        .user_agent(crate::api::asmrone::USER_AGENT)
        .timeout(Duration::from_secs(30));
    // asmr.one 相关域名按 ASMR ONE 开关走代理，其余（DLsite CDN 等）按 DLsite 开关
    let is_asmr = url.contains("asmr-100.com")
        || url.contains("asmr-200.com")
        || url.contains("asmr.one");
    let builder = if is_asmr {
        proxy.apply_async(builder, crate::api::proxy::ProxyTarget::AsmrOne)
    } else {
        proxy.apply_async(builder, crate::api::proxy::ProxyTarget::Dlsite)
    };
    let client = builder.build().map_err(|e| format!("构建请求客户端失败: {e}"))?;
    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("拉取图片失败: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("拉取图片 HTTP 状态异常: {}", resp.status()));
    }
    let mime = resp
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .filter(|v| v.starts_with("image/"))
        .unwrap_or("image/jpeg")
        .to_string();
    let mut bytes = Vec::new();
    let mut stream = resp.bytes_stream();
    use futures::StreamExt;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("读取图片数据流失败: {e}"))?;
        bytes.extend_from_slice(&chunk);
        if bytes.len() > 16 * 1024 * 1024 {
            return Err("图片超过大小限制".to_string());
        }
    }
    Ok((bytes, mime))
}
