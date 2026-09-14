//! asmr.one 在线浏览：热门/推荐/全部列表、收藏、播放列表、作品详情。
//!
//! 端点（实测于 api.asmr.one，全部公开可读；收藏需登录态）：
//! - 列表   GET /api/works?page&order&sort（dl_count=热门下载量 / release=最新 / rating=评分 / random=随机推荐）
//! - 收藏   GET /api/review?page&order&sort（登录用户的收藏，即评分过的作品）
//! - 播放列表 GET /api/playlist/get-playlists、GET /api/playlist/get-playlist-works?id&page&pageSize
//! - 详情   GET /api/work/{id}（source_id 即 RJ 号）

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::api::proxy::{ProxyConfig, ProxyTarget};

pub const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0 Safari/537.36";

/// 在线作品条目（列表卡片 / 详情共用）。
/// local_work_id 仅用于本站数据（如播放历史）直接关联本地作品 id。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AsmrOnlineWork {
    /// asmr.one 作品数字 id；本地数据条目为 0
    pub id: i64,
    #[serde(rename = "rj_code")]
    pub rj_code: Option<String>,
    pub title: String,
    pub circle_name: Option<String>,
    pub cover_url: Option<String>,
    pub release: Option<String>,
    pub dl_count: Option<i64>,
    pub price: Option<i64>,
    pub rating: Option<f64>,
    pub rate_count: Option<i64>,
    pub duration_sec: Option<i64>,
    pub vas: Vec<String>,
    pub tags: Vec<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub local_work_id: Option<i64>,
}

/// 分页列表（每页 20 条，与 asmr.one 官方 pageSize 一致）。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AsmrWorksPage {
    pub items: Vec<AsmrOnlineWork>,
    pub page: i64,
    pub has_next: bool,
}

/// 播放列表概要。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AsmrPlaylist {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub works_count: Option<i64>,
    pub main_cover_url: Option<String>,
    pub user_name: Option<String>,
}

fn build_client(proxy: &ProxyConfig) -> Result<reqwest::blocking::Client, String> {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        reqwest::header::REFERER,
        "https://www.asmr.one/".parse().unwrap(),
    );
    let builder = reqwest::blocking::Client::builder()
        .user_agent(USER_AGENT)
        .default_headers(headers)
        .timeout(std::time::Duration::from_secs(30));
    proxy
        .apply_blocking(builder, ProxyTarget::AsmrOne)
        .build()
        .map_err(|e| format!("构建请求客户端失败: {e}"))
}

/// GET 请求并解析 JSON；非 2xx 返回 "HTTP {status} ..." 格式错误（401 由调用方识别重登）。
fn get_json(
    client: &reqwest::blocking::Client,
    url: &str,
    token: &str,
) -> Result<Value, String> {
    let mut req = client.get(url);
    if !token.trim().is_empty() {
        req = req.header("Authorization", format!("Bearer {}", token.trim()));
    }
    let resp = req.send().map_err(|e| format!("连接 asmr.one 失败: {e}"))?;
    let status = resp.status();
    if !status.is_success() {
        return Err(format!("HTTP {} asmr.one 响应异常", status.as_u16()));
    }
    resp.json()
        .map_err(|e| format!("解析 asmr.one 响应失败: {e}"))
}

/// 从 JSON 对象归一化在线作品条目。RJ 号优先取 source_id，其次从标题提取。
fn parse_work(v: &Value) -> Option<AsmrOnlineWork> {
    let id = v.get("id").and_then(|x| {
        x.as_i64().or_else(|| x.as_str().and_then(|s| s.parse().ok()))
    })?;
    let title = v
        .get("title")
        .and_then(|x| x.as_str())
        .or_else(|| v.get("name").and_then(|x| x.as_str()))
        .unwrap_or_default()
        .trim()
        .to_string();
    let rj_code = v
        .get("source_id")
        .and_then(|x| x.as_str())
        .map(str::trim)
        .filter(|s| s.starts_with("RJ"))
        .map(String::from)
        .or_else(|| {
            crate::handlers::import::extract_rj_code(&title)
                .or_else(|| v.get("name").and_then(|x| x.as_str()).and_then(crate::handlers::import::extract_rj_code))
        });
    let circle_name = v
        .get("circle")
        .and_then(|c| c.get("name"))
        .and_then(|x| x.as_str())
        .map(String::from);
    let names = |key: &str| -> Vec<String> {
        v.get(key)
            .and_then(|x| x.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|o| o.get("name").and_then(|n| n.as_str()).map(String::from))
                    .collect()
            })
            .unwrap_or_default()
    };
    Some(AsmrOnlineWork {
        id,
        rj_code,
        title,
        circle_name,
        cover_url: v
            .get("mainCoverUrl")
            .and_then(|x| x.as_str())
            .map(String::from),
        release: v.get("release").and_then(|x| x.as_str()).map(String::from),
        dl_count: v.get("dl_count").and_then(|x| x.as_i64()),
        price: v.get("price").and_then(|x| x.as_i64()),
        rating: v.get("rate_average_2dp").and_then(|x| x.as_f64()),
        rate_count: v.get("rate_count").and_then(|x| x.as_i64()),
        duration_sec: v.get("duration").and_then(|x| x.as_i64()),
        vas: names("vas"),
        tags: names("tags"),
        description: None,
        local_work_id: None,
    })
}

/// 解析 asmr.one 分页对象为 has_next：currentPage * pageSize < totalCount。
fn has_next_from(pagination: Option<&Value>) -> bool {
    pagination
        .and_then(|p| {
            Some(
                p.get("currentPage").and_then(|x| x.as_i64()).unwrap_or(1)
                    * p.get("pageSize").and_then(|x| x.as_i64()).unwrap_or(0)
                    < p.get("totalCount").and_then(|x| x.as_i64()).unwrap_or(0),
            )
        })
        .unwrap_or(false)
}

fn parse_works_page(d: &Value, page: i64) -> AsmrWorksPage {
    AsmrWorksPage {
        items: d
            .get("works")
            .and_then(|x| x.as_array())
            .map(|arr| arr.iter().filter_map(parse_work).collect())
            .unwrap_or_default(),
        page,
        has_next: has_next_from(d.get("pagination")),
    }
}

/// 全部列表（order: dl_count=热门 / release=最新 / rating=评分 / random=随机推荐）。
pub fn fetch_works_page(
    api_base: &str,
    token: &str,
    proxy: &ProxyConfig,
    page: i64,
    order: &str,
) -> Result<AsmrWorksPage, String> {
    let client = build_client(proxy)?;
    let url = format!("{api_base}/api/works?page={page}&order={order}&sort=desc");
    let d = get_json(&client, &url, token)?;
    Ok(parse_works_page(&d, page))
}

/// 收藏（登录用户评分/收藏的作品）。
pub fn fetch_favorites(
    api_base: &str,
    token: &str,
    proxy: &ProxyConfig,
    page: i64,
) -> Result<AsmrWorksPage, String> {
    let client = build_client(proxy)?;
    let url = format!("{api_base}/api/review?page={page}&order=release&sort=desc");
    let d = get_json(&client, &url, token)?;
    Ok(parse_works_page(&d, page))
}

/// 播放列表概要（不分页返回当前页列表）。
pub fn fetch_playlists(
    api_base: &str,
    token: &str,
    proxy: &ProxyConfig,
) -> Result<Vec<AsmrPlaylist>, String> {
    let client = build_client(proxy)?;
    let url = format!("{api_base}/api/playlist/get-playlists");
    let d = get_json(&client, &url, token)?;
    Ok(d.get("playlists")
        .and_then(|x| x.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|p| {
                    Some(AsmrPlaylist {
                        id: p.get("id").and_then(|x| x.as_str()).map(String::from)?,
                        name: p
                            .get("name")
                            .and_then(|x| x.as_str())
                            .unwrap_or("未命名")
                            .to_string(),
                        description: p
                            .get("description")
                            .and_then(|x| x.as_str())
                            .map(String::from),
                        works_count: p.get("works_count").and_then(|x| x.as_i64()),
                        main_cover_url: p
                            .get("mainCoverUrl")
                            .and_then(|x| x.as_str())
                            .map(String::from),
                        user_name: p.get("user_name").and_then(|x| x.as_str()).map(String::from),
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default())
}

/// 某个播放列表内的作品（分页）。
pub fn fetch_playlist_works(
    api_base: &str,
    token: &str,
    proxy: &ProxyConfig,
    playlist_id: &str,
    page: i64,
) -> Result<AsmrWorksPage, String> {
    let client = build_client(proxy)?;
    let url = format!(
        "{api_base}/api/playlist/get-playlist-works?id={playlist_id}&page={page}&pageSize=20"
    );
    let d = get_json(&client, &url, token)?;
    Ok(parse_works_page(&d, page))
}

/// 作品详情。
pub fn fetch_work_detail(
    api_base: &str,
    token: &str,
    proxy: &ProxyConfig,
    id: i64,
) -> Result<AsmrOnlineWork, String> {
    let client = build_client(proxy)?;
    let url = format!("{api_base}/api/work/{id}");
    let d = get_json(&client, &url, token)?;
    parse_work(&d).ok_or_else(|| "响应中没有作品数据".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parse_work_extracts_rj_and_fields() {
        let w = parse_work(&json!({
            "id": 1657200,
            "title": "RJ01657200 巨乳ギャルJK",
            "name": "巨乳ギャルJK",
            "circle": { "name": "サークルA" },
            "mainCoverUrl": "https://img.example/main.jpg",
            "release": "2026-09-10",
            "dl_count": 1234,
            "price": 1320,
            "rate_average_2dp": 4.5,
            "rate_count": 67,
            "duration": 579,
            "source_id": "RJ01657200",
            "vas": [{ "name": "声優1" }],
            "tags": [{ "name": "耳かき" }]
        }))
        .unwrap();
        assert_eq!(w.id, 1657200);
        assert_eq!(w.rj_code.as_deref(), Some("RJ01657200"));
        assert_eq!(w.circle_name.as_deref(), Some("サークルA"));
        assert_eq!(w.rating, Some(4.5));
        assert_eq!(w.duration_sec, Some(579));
        assert_eq!(w.vas, vec!["声優1"]);
        assert_eq!(w.tags, vec!["耳かき"]);
    }

    #[test]
    fn parse_work_rj_from_title_fallback() {
        let w = parse_work(&json!({
            "id": 1,
            "title": "【日本語】RJ01234567 テスト作品",
            "name": "テスト作品"
        }))
        .unwrap();
        assert_eq!(w.rj_code.as_deref(), Some("RJ01234567"));
    }

    #[test]
    fn parse_work_missing_id_is_skipped() {
        assert!(parse_work(&json!({ "title": "no id" })).is_none());
    }

    #[test]
    fn has_next_pagination_logic() {
        let p = |cur: i64, size: i64, total: i64| {
            json!({ "currentPage": cur, "pageSize": size, "totalCount": total })
        };
        assert!(has_next_from(Some(&p(1, 20, 100))));
        assert!(!has_next_from(Some(&p(5, 20, 100))));
        assert!(!has_next_from(None));
    }
}
