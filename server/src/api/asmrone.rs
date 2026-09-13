//! asmr.one 数据源。
//!
//! asmr.one 有反爬，单一方式不稳定，因此采用多策略：
//! 1. 先试公开 JSON API `https://api.asmr-200.com/api/work/{RJ}`（无需登录，部分端点匿名可访问）
//! 2. 失败再抓网页版 `https://www.asmr.one/work/{RJ}`，解析 Next.js `__NEXT_DATA__` JSON
//! 3. 再失败解析页面 og: meta
//! 所有请求带浏览器 UA 和完整请求头，尽量绕过基础反爬。若仍被拦截，返回明确错误。

use crate::db::models::ScrapedWork;
use crate::api::proxy::{ProxyConfig, ProxyTarget};
use crate::api::scraper::{guess_work_type, parse_file_size};
use scraper::{Html, Selector};
use std::error::Error;

/// 公开给 lib.rs 的 custom protocol handler 复用。
pub const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0 Safari/537.36";

/// 默认 API 基址（asmr.one 官方主域名）。
pub const DEFAULT_API_BASE: &str = "https://api.asmr.one";

const WEB_BASE: &str = "https://www.asmr.one/work";

/// 根据用户配置的站点地址推导 API 基址。
/// 规则：无 scheme 补 https://；非 api. 开头的主机（如 www.asmr.one / asmr.one）
/// 自动换成 api. 前缀；已带 api. 前缀（含镜像 api.asmr-100/200/300.com）原样使用。
pub fn normalize_api_base(input: Option<&str>) -> String {
    let Some(raw) = input.map(str::trim).filter(|s| !s.is_empty()) else {
        return DEFAULT_API_BASE.to_string();
    };
    let with_scheme = if raw.contains("://") {
        raw.to_string()
    } else {
        format!("https://{raw}")
    };
    let host = with_scheme
        .split("://")
        .nth(1)
        .unwrap_or_default()
        .split('/')
        .next()
        .unwrap_or_default()
        .to_string();
    if host.is_empty() {
        return DEFAULT_API_BASE.to_string();
    }
    if host.starts_with("api.") {
        format!("https://{host}")
    } else {
        let bare = host.trim_start_matches("www.");
        format!("https://api.{bare}")
    }
}

/// 使用账号密码登录 asmr.one，成功返回 JWT token。
/// 登录端点：POST {base}/api/auth/me，请求体 {"name": 账号, "password": 密码}。
pub fn login(
    api_base: &str,
    username: &str,
    password: &str,
    proxy: &ProxyConfig,
) -> Result<String, String> {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        reqwest::header::REFERER,
        "https://www.asmr.one/".parse().unwrap(),
    );
    let builder = reqwest::blocking::Client::builder()
        .user_agent(USER_AGENT)
        .default_headers(headers)
        .timeout(std::time::Duration::from_secs(30));
    let client = proxy
        .apply_blocking(builder, ProxyTarget::AsmrOne)
        .build()
        .map_err(|e| format!("构建请求客户端失败: {e}"))?;

    let resp = client
        .post(format!("{api_base}/api/auth/me"))
        .json(&serde_json::json!({ "name": username, "password": password }))
        .send()
        .map_err(|e| format!("连接 asmr.one 失败: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!(
            "登录失败: HTTP {}（请检查账号密码或站点地址）",
            resp.status()
        ));
    }
    let v: serde_json::Value = resp.json().map_err(|e| format!("解析登录响应失败: {e}"))?;
    v.get("token")
        .and_then(|t| t.as_str())
        .map(String::from)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| "登录响应中没有 token".to_string())
}

fn build_client(proxy: &ProxyConfig) -> Result<reqwest::blocking::Client, Box<dyn Error>> {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(reqwest::header::USER_AGENT, USER_AGENT.parse()?);
    headers.insert(reqwest::header::ACCEPT, "text/html,application/json,application/xhtml+xml,*/*;q=0.8".parse()?);
    headers.insert(reqwest::header::ACCEPT_LANGUAGE, "zh-CN,zh;q=0.9,ja;q=0.8,en;q=0.7".parse()?);
    headers.insert(reqwest::header::REFERER, "https://www.asmr.one/".parse()?);
    let builder = reqwest::blocking::Client::builder()
        .default_headers(headers)
        .redirect(reqwest::redirect::Policy::limited(5))
        .timeout(std::time::Duration::from_secs(30));
    Ok(proxy.apply_blocking(builder, ProxyTarget::AsmrOne).build()?)
}

fn dlsite_url(rj: &str) -> String {
    format!(
        "https://www.dlsite.com/maniax/work/=/product_id/{}.html",
        rj
    )
}

/// 抓取 asmr.one 作品元数据（多策略）。
/// token 为 asmr.one 登录后的 Bearer token（可选），api_base 为可配置的 API 基址。
pub fn fetch_work_from_asmrone(
    rj_code: &str,
    token: Option<&str>,
    proxy: &ProxyConfig,
    api_base: &str,
) -> Result<ScrapedWork, Box<dyn Error>> {
    let rj = rj_code.trim().to_uppercase();
    let client = build_client(proxy)?;

    // 1) 带 token 的 API（登录态）
    if let Some(t) = token.filter(|t| !t.trim().is_empty()) {
        if let Some(w) = fetch_from_api(&client, api_base, &rj, Some(t)) {
            return Ok(w);
        }
    }

    // 2) 公开 JSON API（匿名）
    let api_err = match fetch_from_api(&client, api_base, &rj, None) {
        Some(w) => return Ok(w),
        None => "API 返回空数据".to_string(),
    };

    // 3) 网页版
    let web_err = match fetch_from_web(&client, &rj) {
        Some(w) => return Ok(w),
        None => "网页解析不到作品数据".to_string(),
    };

    Err(format!(
        "asmr.one 抓取失败。API: {}；网页: {}。若为 403/被拦截说明有反爬，可在「设置 → asmr.one」里登录获取 Token 后再试",
        api_err, web_err
    )
    .into())
}

// ==================== 音频与字幕文件列表（在线试听用）====================

use crate::db::models::SubtitleInfo;

/// asmr.one 上的一个音频文件（含匹配的字幕列表）。
#[derive(Debug, Clone, serde::Serialize)]
pub struct AsmrFile {
    pub hash: String,
    pub title: String,
    pub media_extension: String,
    pub stream_url: String,
    pub duration_sec: Option<f64>,
    pub size_bytes: Option<i64>,
    pub subtitles: Vec<SubtitleInfo>,
}

/// asmr.one 文件树节点（用于下载选择界面展示完整目录结构）。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AsmrTreeNode {
    pub title: String,
    pub node_type: String,
    pub extension: String,
    pub hash: String,
    pub download_url: String,
    pub size: Option<i64>,
    pub duration: Option<f64>,
    pub children: Vec<AsmrTreeNode>,
}

/// 递归构建完整文件树（保留文件夹层级，含所有文件类型）。
pub fn build_file_tree(root: &serde_json::Value) -> Vec<AsmrTreeNode> {
    match root {
        serde_json::Value::Array(arr) => arr.iter().map(|item| build_tree_node(item)).collect(),
        serde_json::Value::Object(_) => {
            // 单个节点直接包装
            vec![build_tree_node(root)]
        }
        _ => Vec::new(),
    }
}

fn build_tree_node(v: &serde_json::Value) -> AsmrTreeNode {
    let node_type_raw = get_str(v, &["type"]).unwrap_or_default().to_lowercase();
    let title = get_str(v, &["title", "name"]).unwrap_or_default();
    let ext = title
        .rsplit_once('.')
        .map(|(_, e)| e.to_lowercase())
        .unwrap_or_default();
    let hash = get_str(v, &["hash", "id"]).unwrap_or_default();

    let has_children = v.get("children").and_then(|c| c.as_array()).map(|a| !a.is_empty()).unwrap_or(false);
    let is_folder = node_type_raw == "folder" || has_children;

    let node_type = if is_folder {
        "folder".to_string()
    } else if node_type_raw == "audio" || matches!(ext.as_str(), "mp3" | "m4a" | "wav" | "flac" | "ogg" | "aac") {
        "audio".to_string()
    } else if node_type_raw == "image" || matches!(ext.as_str(), "jpg" | "jpeg" | "png" | "gif" | "bmp" | "webp") {
        "image".to_string()
    } else if node_type_raw == "text" || matches!(ext.as_str(), "vtt" | "srt" | "lrc" | "txt") {
        "text".to_string()
    } else {
        "other".to_string()
    };

    // 下载 URL：优先 mediaDownloadUrl > mediaStreamUrl > streamLowQualityUrl > 代理兜底
    let download_url = ["mediaDownloadUrl", "mediaStreamUrl", "streamLowQualityUrl"]
        .iter()
        .find_map(|k| get_str(v, &[k]).filter(|s| !s.trim().is_empty()))
        .unwrap_or_else(|| {
            if !hash.is_empty() && !is_folder {
                format!("/api/asmr/file/{}", hash)
            } else {
                String::new()
            }
        });

    let children = if is_folder {
        v.get("children")
            .and_then(|c| c.as_array())
            .map(|arr| arr.iter().map(|item| build_tree_node(item)).collect())
            .unwrap_or_default()
    } else {
        Vec::new()
    };

    AsmrTreeNode {
        title,
        node_type,
        extension: ext,
        hash,
        download_url,
        size: get_i64(v, &["size"]).filter(|s| *s > 0),
        duration: get_f64(v, &["duration"]).filter(|d| *d > 0.0),
        children,
    }
}

/// 抓取 asmr.one 上某作品的完整文件树（含所有类型），用于下载文件选择界面。
pub fn fetch_file_tree(
    rj_code: &str,
    token: Option<&str>,
    proxy: &ProxyConfig,
    api_base: &str,
) -> Result<Vec<AsmrTreeNode>, String> {
    let rj = rj_code.trim().to_uppercase();
    let work_id: u64 = rj
        .trim_start_matches("RJ")
        .parse()
        .map_err(|_| format!("无效的 RJ 号: {rj}"))?;
    let client = build_client(proxy).map_err(|e| e.to_string())?;
    let url = format!("{api_base}/api/tracks/{work_id}");
    let mut req = client.get(&url);
    if let Some(t) = token.filter(|t| !t.trim().is_empty()) {
        req = req.bearer_auth(t);
    }
    let resp = req
        .send()
        .map_err(|e| format!("请求 asmr.one 文件列表失败: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("asmr.one 文件列表返回 {}", resp.status()));
    }
    let v: serde_json::Value = resp
        .json()
        .map_err(|e| format!("解析文件列表 JSON 失败: {e}"))?;
    Ok(build_file_tree(&v))
}

/// 抓取 asmr.one 上某作品的音频文件及关联字幕列表（递归解析文件树）。
/// 端点：`GET /api/tracks/{workId}`（workId 为 RJ 号去 "RJ" 前缀的数字）。
pub fn fetch_audio_files(
    rj_code: &str,
    token: Option<&str>,
    proxy: &ProxyConfig,
    api_base: &str,
) -> Result<Vec<AsmrFile>, String> {
    let rj = rj_code.trim().to_uppercase();
    let work_id: u64 = rj
        .trim_start_matches("RJ")
        .parse()
        .map_err(|_| format!("无效的 RJ 号: {rj}"))?;
    let client = build_client(proxy).map_err(|e| e.to_string())?;
    let url = format!("{api_base}/api/tracks/{work_id}");
    let mut req = client.get(&url);
    if let Some(t) = token.filter(|t| !t.trim().is_empty()) {
        req = req.bearer_auth(t);
    }
    let resp = req
        .send()
        .map_err(|e| format!("请求 asmr.one 文件列表失败: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("asmr.one 文件列表返回 {}", resp.status()));
    }
    let v: serde_json::Value = resp
        .json()
        .map_err(|e| format!("解析文件列表 JSON 失败: {e}"))?;
    Ok(collect_tracks_and_subtitles(&v))
}

struct RawAudioItem {
    folder: Vec<String>,
    file: AsmrFile,
}

struct RawSubItem {
    folder: Vec<String>,
    sub: SubtitleInfo,
}

fn normalize_stem(name: &str) -> String {
    let stem = name.rsplit_once('.').map(|(s, _)| s).unwrap_or(name);
    stem.to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
        .collect()
}

/// 递归遍历 asmr.one tracks 文件树，收集音频与字幕节点并进行智能匹配。
fn collect_tracks_and_subtitles(root: &serde_json::Value) -> Vec<AsmrFile> {
    let mut audio_items: Vec<RawAudioItem> = Vec::new();
    let mut sub_items: Vec<RawSubItem> = Vec::new();
    let mut folder_stack: Vec<String> = Vec::new();

    traverse_tree(root, &mut folder_stack, &mut audio_items, &mut sub_items);

    let total_audio = audio_items.len();
    let total_subs = sub_items.len();

    // 智能匹配字幕到对应音轨
    let mut result: Vec<AsmrFile> = Vec::with_capacity(total_audio);
    for mut item in audio_items {
        let audio_stem = normalize_stem(&item.file.title);
        for sub_item in &sub_items {
            let sub_stem = normalize_stem(&sub_item.sub.title);
            let same_folder = item.folder == sub_item.folder;

            let matched = if audio_stem == sub_stem {
                true
            } else if same_folder && (sub_stem.contains(&audio_stem) || audio_stem.contains(&sub_stem)) {
                true
            } else if sub_stem.contains(&audio_stem) {
                true
            } else if total_audio == 1 && total_subs == 1 {
                true
            } else {
                false
            };

            if matched {
                if !item.file.subtitles.iter().any(|s| s.url == sub_item.sub.url || s.title == sub_item.sub.title) {
                    item.file.subtitles.push(sub_item.sub.clone());
                }
            }
        }
        result.push(item.file);
    }

    result
}

fn traverse_tree(
    v: &serde_json::Value,
    folder_stack: &mut Vec<String>,
    audio_items: &mut Vec<RawAudioItem>,
    sub_items: &mut Vec<RawSubItem>,
) {
    match v {
        serde_json::Value::Array(arr) => {
            for item in arr {
                traverse_tree(item, folder_stack, audio_items, sub_items);
            }
        }
        serde_json::Value::Object(_) => {
            let node_type = get_str(v, &["type"]).unwrap_or_default().to_lowercase();
            let title = get_str(v, &["title", "name"]).unwrap_or_default();
            let ext = title
                .rsplit_once('.')
                .map(|(_, e)| e.to_lowercase())
                .unwrap_or_default();
            let hash = get_str(v, &["hash", "id"]).unwrap_or_default();

            let is_audio_ext = matches!(ext.as_str(), "mp3" | "m4a" | "wav" | "flac" | "ogg" | "aac");
            let is_sub_ext = matches!(ext.as_str(), "vtt" | "srt" | "lrc")
                || (ext == "txt" && (title.contains("字幕") || title.contains("歌詞") || title.to_lowercase().contains("lrc") || title.to_lowercase().contains("vtt") || title.to_lowercase().contains("srt")));

            let stream_url = ["mediaStreamUrl", "streamLowQualityUrl", "mediaDownloadUrl"]
                .iter()
                .find_map(|k| get_str(v, &[k]).filter(|s| !s.trim().is_empty()))
                .unwrap_or_default();

            let is_audio = node_type == "audio" || (is_audio_ext && !stream_url.is_empty());
            let is_sub = is_sub_ext || (node_type == "text" && matches!(ext.as_str(), "vtt" | "srt" | "lrc" | "txt"));

            if is_audio {
                let mut embedded_subs = Vec::new();
                if let Some(subs_arr) = v.get("subtitles").and_then(|s| s.as_array()) {
                    for item in subs_arr {
                        let sub_title = get_str(item, &["title", "name"]).unwrap_or_default();
                        let sub_ext = sub_title
                            .rsplit_once('.')
                            .map(|(_, e)| e.to_lowercase())
                            .unwrap_or_else(|| "vtt".to_string());
                        let sub_hash = get_str(item, &["hash", "id"]).unwrap_or_default();
                        let sub_url = ["mediaDownloadUrl", "mediaStreamUrl", "streamUrl"]
                            .iter()
                            .find_map(|k| get_str(item, &[k]).filter(|s| !s.trim().is_empty()))
                            .unwrap_or_else(|| {
                                if !sub_hash.is_empty() {
                                    format!("/api/asmr/file/{}", sub_hash)
                                } else {
                                    String::new()
                                }
                            });
                        if !sub_url.is_empty() {
                            embedded_subs.push(SubtitleInfo {
                                title: sub_title,
                                extension: sub_ext,
                                url: sub_url,
                            });
                        }
                    }
                }

                audio_items.push(RawAudioItem {
                    folder: folder_stack.clone(),
                    file: AsmrFile {
                        hash,
                        title,
                        media_extension: ext,
                        stream_url,
                        duration_sec: get_f64(v, &["duration"]).filter(|d| *d > 0.0),
                        size_bytes: get_i64(v, &["size"]).filter(|s| *s > 0),
                        subtitles: embedded_subs,
                    },
                });
                return;
            }

            if is_sub {
                let sub_url = if !stream_url.is_empty() {
                    stream_url
                } else if !hash.is_empty() {
                    format!("/api/asmr/file/{}", hash)
                } else {
                    String::new()
                };

                if !sub_url.is_empty() {
                    sub_items.push(RawSubItem {
                        folder: folder_stack.clone(),
                        sub: SubtitleInfo {
                            title,
                            extension: ext,
                            url: sub_url,
                        },
                    });
                }
                return;
            }

            // folder 或容器节点：遍历所有子节点
            let folder_name = if node_type == "folder" && !title.is_empty() {
                Some(title)
            } else {
                None
            };

            if let Some(ref fn_name) = folder_name {
                folder_stack.push(fn_name.clone());
            }

            if let Some(obj) = v.as_object() {
                for (k, child) in obj {
                    if k == "children" || child.is_array() || child.is_object() {
                        traverse_tree(child, folder_stack, audio_items, sub_items);
                    }
                }
            }

            if folder_name.is_some() {
                folder_stack.pop();
            }
        }
        _ => {}
    }
}

/// RJ 号转 asmr.one 的纯数字作品 ID（"RJ01014447" → "1014447"）。
/// asmr.one 的 API / 网页 URL 只接受数字 ID，带 RJ 前缀会返回 400。
fn asmr_numeric_id(rj: &str) -> Option<String> {
    let digits: String = rj
        .trim_start_matches(|c: char| !c.is_ascii_digit())
        .to_string();
    if digits.is_empty() || !digits.chars().all(|c| c.is_ascii_digit()) {
        None
    } else {
        Some(digits)
    }
}

/// 策略 1/2：JSON API（可选带 Bearer token）。
/// 成功返回 Some，失败/无数据返回 None。
fn fetch_from_api(
    client: &reqwest::blocking::Client,
    api_base: &str,
    rj: &str,
    token: Option<&str>,
) -> Option<ScrapedWork> {
    let work_id = asmr_numeric_id(rj)?;
    let url = format!("{api_base}/api/work/{work_id}");
    let mut req = client.get(&url);
    if let Some(t) = token {
        req = req.bearer_auth(t);
    }
    let resp = req.send().ok()?;
    if !resp.status().is_success() {
        return None; // 401/403/5xx 都跳过
    }
    let v: serde_json::Value = resp.json().ok()?;
    let data = get(&v, &["data"]).unwrap_or(&v);
    if get_str(data, &["title"]).is_none() {
        return None;
    }
    Some(map_work(data, rj))
}

/// 策略 3：网页版。
fn fetch_from_web(client: &reqwest::blocking::Client, rj: &str) -> Option<ScrapedWork> {
    let work_id = asmr_numeric_id(rj)?;
    let url = format!("{}/{}", WEB_BASE, work_id);
    let resp = client.get(&url).send().ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let html = resp.text().ok()?;

    // 优先 __NEXT_DATA__ JSON
    if let Some(w) = parse_next_data(&html, rj) {
        return Some(w);
    }
    // 兜底 og meta
    parse_html_meta(&html, rj)
}

// ==================== __NEXT_DATA__ JSON 提取 ====================

fn parse_next_data(html: &str, rj: &str) -> Option<ScrapedWork> {
    // 宽松匹配 <script id="__NEXT_DATA__" ...>，属性顺序可能不同
    let idx = html.find("__NEXT_DATA__")?;
    let start = html[idx..].find('>')? + idx + 1;
    let end = html[start..].find("</script>")? + start;
    let json: serde_json::Value = serde_json::from_str(&html[start..end]).ok()?;
    let work = find_work(&json, rj)?;
    Some(map_work(work, rj))
}

/// 递归在 JSON 里找一个「作品对象」。
fn find_work<'a>(v: &'a serde_json::Value, rj: &str) -> Option<&'a serde_json::Value> {
    match v {
        serde_json::Value::Object(map) => {
            let has_title = map.get("title").and_then(|t| t.as_str()).is_some();
            let has_rj = map.get("sourceId").and_then(|t| t.as_str())
                .or_else(|| map.get("rjCode").and_then(|t| t.as_str()))
                .map(|s| s.trim().eq_ignore_ascii_case(rj))
                .unwrap_or(false);
            let has_work_keys = has_rj
                || map.contains_key("circle")
                || map.contains_key("vas")
                || map.contains_key("tags")
                || map.contains_key("duration");
            if has_title && has_work_keys {
                return Some(v);
            }
            for (_, val) in map {
                if let Some(found) = find_work(val, rj) {
                    return Some(found);
                }
            }
            None
        }
        serde_json::Value::Array(arr) => {
            for val in arr {
                if let Some(found) = find_work(val, rj) {
                    return Some(found);
                }
            }
            None
        }
        _ => None,
    }
}

fn map_work(work: &serde_json::Value, rj: &str) -> ScrapedWork {
    let mut w = ScrapedWork {
        rj_code: rj.to_string(),
        work_type: guess_work_type(rj),
        dlsite_url: Some(dlsite_url(rj)),
        ..Default::default()
    };
    w.title_ja = get_str(work, &["title"]);
    w.sale_date = get_str(work, &["release", "release_date", "saleDate"]);
    w.price = get_i64(work, &["price"]);
    w.description = get_str(work, &["description"]);
    // asmr.one API 的封面字段为 mainCoverUrl（samCoverUrl 为缩略图）
    if let Some(c) = get(work, &["cover", "cover_url", "image", "mainCoverUrl"]) {
        w.cover_url = c
            .as_str()
            .map(|s| s.to_string())
            .or_else(|| get_str(c, &["url"]));
    }
    if let Some(circle) = get(work, &["circle"]) {
        w.circle_name = get_str(circle, &["name", "circleName"]);
        // API 的 circle.id 是数字（get_str 取不到），依次尝试字符串字段，最后数字转字符串
        w.circle_id = get_str(circle, &["source_id"])
            .or_else(|| get_str(circle, &["circleId"]))
            .or_else(|| {
                get(circle, &["id"]).and_then(|v| v.as_i64().map(|i| i.to_string()))
            });
    }
    w.actors = names_from_objects(work, &["vas", "voice_actors", "actors", "voiceActors"]);
    w.tags = names_from_objects(work, &["tags"]);
    if let Some(sec) = get_i64(work, &["duration"]) {
        if sec >= 60 {
            w.duration_min = Some(sec / 60);
        }
    }
    if w.duration_min.is_none() {
        w.duration_min = get_i64(work, &["duration_min", "durationMin"]);
    }
    if let Some(mb) = get_f64(work, &["file_size_mb", "fileSizeMb"]) {
        w.file_size_mb = Some(mb);
    } else if let Some(fs) = get_str(work, &["file_size", "fileSize"]) {
        w.file_size_mb = parse_file_size(&fs);
    }
    w.age_class = Some(if get_bool(work, &["is_r18", "nsfw"]) {
        "r18".to_string()
    } else {
        "general".to_string()
    });
    w
}

// ==================== HTML meta 兜底 ====================

fn parse_html_meta(html: &str, rj: &str) -> Option<ScrapedWork> {
    let doc = Html::parse_document(html);
    let meta_sel = Selector::parse("meta[property], meta[name]").ok()?;
    let meta: Vec<(String, String)> = doc
        .select(&meta_sel)
        .filter_map(|e| {
            let prop = e.value().attr("property").or_else(|| e.value().attr("name"))?;
            let content = e.value().attr("content")?.trim().to_string();
            Some((prop.to_lowercase(), content))
        })
        .collect();
    let get_meta = |key: &str| -> Option<String> {
        meta.iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.clone())
    };

    let title = get_meta("og:title")?;
    let mut w = ScrapedWork {
        rj_code: rj.to_string(),
        title_ja: Some(title.trim().to_string()),
        work_type: guess_work_type(rj),
        dlsite_url: Some(dlsite_url(rj)),
        cover_url: get_meta("og:image"),
        description: get_meta("og:description").or_else(|| get_meta("description")),
        ..Default::default()
    };
    if let Some(sel) = Selector::parse("a[href*='circle'], .circle, [class*='circle']").ok() {
        if let Some(e) = doc.select(&sel).next() {
            let t = e.text().collect::<String>().trim().to_string();
            if !t.is_empty() {
                w.circle_name = Some(t);
            }
        }
    }
    Some(w)
}

// ==================== 容错提取辅助 ====================

fn get<'a>(v: &'a serde_json::Value, keys: &[&str]) -> Option<&'a serde_json::Value> {
    keys.iter().find_map(|k| v.get(*k))
}

fn get_str(v: &serde_json::Value, keys: &[&str]) -> Option<String> {
    get(v, keys).and_then(|x| x.as_str().map(|s| s.trim().to_string()))
}

fn get_i64(v: &serde_json::Value, keys: &[&str]) -> Option<i64> {
    get(v, keys).and_then(|x| x.as_i64())
}

fn get_f64(v: &serde_json::Value, keys: &[&str]) -> Option<f64> {
    get(v, keys).and_then(|x| x.as_f64())
}

fn get_bool(v: &serde_json::Value, keys: &[&str]) -> bool {
    get(v, keys).and_then(|x| x.as_bool()).unwrap_or(false)
}

/// 从形如 `[{ "name": "声優名", ... }, ...]` 的数组提取 name 列表。
fn names_from_objects(v: &serde_json::Value, keys: &[&str]) -> Vec<String> {
    let Some(arr) = get(v, keys).and_then(|x| x.as_array()) else {
        return vec![];
    };
    arr.iter()
        .filter_map(|o| get_str(o, &["name", "name_en", "name_ja"]))
        .filter(|s| !s.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_api_base_variants() {
        assert_eq!(normalize_api_base(None), DEFAULT_API_BASE);
        assert_eq!(normalize_api_base(Some("")), DEFAULT_API_BASE);
        assert_eq!(normalize_api_base(Some("  ")), DEFAULT_API_BASE);
        // 站点地址自动换成 api. 前缀
        assert_eq!(
            normalize_api_base(Some("https://www.asmr.one")),
            "https://api.asmr.one"
        );
        assert_eq!(normalize_api_base(Some("asmr.one")), "https://api.asmr.one");
        assert_eq!(
            normalize_api_base(Some("www.asmr.one/work")),
            "https://api.asmr.one"
        );
        // api. 前缀（含镜像）原样保留，去掉路径
        assert_eq!(
            normalize_api_base(Some("https://api.asmr-200.com")),
            "https://api.asmr-200.com"
        );
        assert_eq!(
            normalize_api_base(Some("api.asmr-100.com/api")),
            "https://api.asmr-100.com"
        );
    }

    #[test]
    fn names_from_objects_works() {
        let v = serde_json::json!({
            "vas": [
                { "id": 1, "name": "佐倉綾音", "count": 5 },
                { "id": 2, "name": "日笠陽子", "count": 3 }
            ]
        });
        let names = names_from_objects(&v, &["vas"]);
        assert_eq!(names.len(), 2);
        assert_eq!(names[0], "佐倉綾音");
    }

    #[test]
    fn find_work_recursively() {
        let json = serde_json::json!({
            "props": { "pageProps": { "work": {
                "id": 123,
                "title": "テスト作品",
                "circle": { "name": "サークルA", "id": "RG1" },
                "vas": [{ "name": "声優1" }],
                "release": "2024-01-01"
            } } }
        });
        let work = find_work(&json, "RJ01014447").expect("should find work");
        let w = map_work(work, "RJ01014447");
        assert_eq!(w.title_ja.as_deref(), Some("テスト作品"));
        assert_eq!(w.circle_name.as_deref(), Some("サークルA"));
        assert_eq!(w.sale_date.as_deref(), Some("2024-01-01"));
    }

    #[test]
    fn html_meta_fallback() {
        let html = r#"
        <html><head>
          <meta property="og:title" content="【耳かき】テスト" />
          <meta property="og:image" content="https://example.com/cover.jpg" />
        </head><body></body></html>
        "#;
        let w = parse_html_meta(html, "RJ01014447").expect("should parse meta");
        assert_eq!(w.title_ja.as_deref(), Some("【耳かき】テスト"));
    }

    #[test]
    fn collect_audio_files_filters_audio() {
        // 模拟 asmr.one /api/tracks/{id} 的响应结构（实测字段：type/title/hash/mediaStreamUrl/duration/size）
        let json = serde_json::json!([
            {
                "type": "folder", "title": "01 本編",
                "children": [
                    { "type": "audio", "hash": "1/10", "title": "01.mp3",
                      "mediaStreamUrl": "https://raw.kiko-play-niptan.one/media/stream/daily/2026-07-23/RJ00000000/01.mp3",
                      "duration": 123.5, "size": 45678 },
                    { "type": "image", "hash": "1/11", "title": "cover.jpg",
                      "mediaDownloadUrl": "https://example.com/cover.jpg" }
                ]
            },
            { "type": "audio", "hash": "2/20", "title": "02.wav" }
        ]);
        let out = collect_tracks_and_subtitles(&json);
        assert_eq!(out.len(), 2, "只收集音频节点（图片即使有 mediaDownloadUrl 也不收）");
        assert_eq!(out[0].hash, "1/10");
        assert_eq!(out[0].media_extension, "mp3");
        assert_eq!(
            out[0].stream_url,
            "https://raw.kiko-play-niptan.one/media/stream/daily/2026-07-23/RJ00000000/01.mp3"
        );
        assert_eq!(out[0].duration_sec, Some(123.5));
        assert_eq!(out[0].size_bytes, Some(45678));
        assert_eq!(out[1].hash, "2/20");
        assert_eq!(out[1].title, "02.wav");
    }

    #[test]
    fn collect_audio_files_no_extension() {
        let json = serde_json::json!([
            { "type": "audio", "hash": "1/10", "title": "无扩展名" }
        ]);
        let out = collect_tracks_and_subtitles(&json);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].media_extension, "");
    }

    #[test]
    fn collect_audio_files_nested_folders() {
        let json = serde_json::json!([
            {
                "type": "folder", "title": "root",
                "children": [
                    {
                        "type": "folder", "title": "sub",
                        "children": [
                            { "type": "audio", "hash": "99/999", "title": "深嵌.m4a" }
                        ]
                    }
                ]
            }
        ]);
        let out = collect_tracks_and_subtitles(&json);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].hash, "99/999");
        assert_eq!(out[0].media_extension, "m4a");
    }

    #[test]
    fn collect_subtitles_matching() {
        let json = serde_json::json!([
            {
                "type": "folder", "title": "01_本篇",
                "children": [
                    {
                        "type": "audio",
                        "hash": "100/1",
                        "title": "01_prologue.mp3",
                        "mediaStreamUrl": "https://example.com/01.mp3"
                    },
                    {
                        "type": "text",
                        "hash": "100/2",
                        "title": "01_prologue_zh.vtt",
                        "mediaDownloadUrl": "https://example.com/01_zh.vtt"
                    },
                    {
                        "type": "other",
                        "hash": "100/3",
                        "title": "01_prologue.lrc"
                    }
                ]
            }
        ]);
        let out = collect_tracks_and_subtitles(&json);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].subtitles.len(), 2);
        assert_eq!(out[0].subtitles[0].title, "01_prologue_zh.vtt");
        assert_eq!(out[0].subtitles[0].url, "https://example.com/01_zh.vtt");
        assert_eq!(out[0].subtitles[1].title, "01_prologue.lrc");
        assert_eq!(out[0].subtitles[1].url, "/api/asmr/file/100/3");
    }
}
