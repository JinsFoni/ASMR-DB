//! DLsite「最新作品」列表：抓取 FS 搜索页（fsr 路径）并解析。
//!
//! 搜索条件固定为用户偏好：音声作品（work_type_category=audio）+
//! 日语/中文任一语言选项，排序由 order 参数决定（最新/人气/销量/评分/赏析）；
//! 分页用路径段 /page/{n}。条目结构与在线页共用（AsmrOnlineWork）：
//! 评分/收藏数/声优等详情字段列表页没有，由前端 enrichMeta 走统一预览接口后台补齐。

use serde_json::Value;
use scraper::{Html, Selector};

use crate::api::asmr_online::AsmrOnlineWork;
use crate::api::proxy::{ProxyConfig, ProxyTarget};

/// 合法的排序参数（对应 DLsite fsr 的 order 路径段）。
pub const ORDERS: [&str; 5] = ["release_d", "trend", "dl_d", "rate_d", "review_d"];

/// 归一化前端传入的排序参数，非法值回落到最新（release_d）。
pub fn normalize_order(order: Option<&str>) -> &'static str {
    match order.unwrap_or("") {
        "trend" => "trend",
        "dl_d" => "dl_d",
        "rate_d" => "rate_d",
        "review_d" => "review_d",
        _ => "release_d",
    }
}

/// 用户给定的 DLsite 音声搜索路径（语言选项 JP/中文/无语言限制），
/// `{order}` 为排序路径段。
const FSR_PATH: &str = "fsr/=/language/jp/sex_category[0]/male/work_category[0]/doujin/\
order/{order}/work_type_category[0]/audio/options_and_or/and/\
options[0]/JPN/options[1]/CHI/options[2]/CHI_HANS/options[3]/CHI_HANT/options[4]/NM/\
lang_options[0]/日语/lang_options[1]/中文/lang_options[2]/中文(简体字)/\
lang_options[3]/中文(繁体字)/lang_options[4]/无语言限制";

fn page_url(order: &str, page: i64) -> String {
    let path = if page <= 1 {
        FSR_PATH.replace("{order}", order)
    } else {
        FSR_PATH.replace("{order}", order) + &format!("/page/{page}")
    };
    // 路径段直接拼 URL，非 ASCII 段做百分号编码
    format!("https://www.dlsite.com/maniax/{}", urlencode_path(&path))
}

/// 仅编码路径段中 URL 不允许的字符（保留 =() 等，[] 与官方链接一致做编码）。
fn urlencode_path(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 16);
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b'/' | b'='
            | b'(' | b')' => out.push(b as char),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

fn build_client(proxy: &ProxyConfig) -> Result<reqwest::blocking::Client, String> {
    crate::api::http_client::shared_blocking_client("dlsite_latest", proxy, || {
        let builder = reqwest::blocking::Client::builder()
            .user_agent(crate::api::asmr_online::USER_AGENT)
            .timeout(std::time::Duration::from_secs(30));
        proxy
            .apply_blocking(builder, ProxyTarget::Dlsite)
            .build()
            .map_err(|e| format!("构建请求客户端失败: {e}"))
    })
}

/// 从属性值中取出第一张 img.dlsite.jp 封面（兼容单值与 Vue 数组形态），补全为 https 绝对地址。
fn extract_img_url(raw: &str) -> Option<String> {
    // 值可能是 '//'img...' 单串、Vue 数组 "['a','b']" 或三元表达式 "is_show ? '//a' : 'b'"
    let u = raw
        .split('\'')
        .find(|seg| seg.contains("img.dlsite.jp"))?;
    Some(format!("https://{}", u.trim_start_matches("//")))
}

/// 抓取一页最新作品列表（每页 30 条）。order 为 DLsite fsr 的排序路径段
/// （如 release_d/trend/dl_d/rate_d/review_d）。
pub fn fetch_latest_page(
    proxy: &ProxyConfig,
    order: &str,
    page: i64,
) -> Result<crate::api::asmr_online::AsmrWorksPage, String> {
    let client = build_client(proxy)?;
    let url = page_url(order, page);
    let resp = client
        .get(&url)
        .header(reqwest::header::REFERER, "https://www.dlsite.com/maniax/")
        .send()
        .map_err(|e| format!("连接 DLsite 失败: {e}"))?;
    let status = resp.status();
    if !status.is_success() {
        return Err(format!("HTTP {} DLsite 响应异常", status.as_u16()));
    }
    let html = resp
        .text()
        .map_err(|e| format!("读取 DLsite 响应失败: {e}"))?;
    let doc = Html::parse_document(&html);
    Ok(parse_latest_page(&doc, page))
}

/// 解析 fsr 搜索页：每个条目是 `li[data-list_item_product_id=RJxxxxx]`，
/// 分页信息取自 `div[data-search-result-count]`（总数）。
fn parse_latest_page(doc: &Html, page: i64) -> crate::api::asmr_online::AsmrWorksPage {
    let li_sel = Selector::parse("li[data-list_item_product_id]").unwrap();
    let name_sel = Selector::parse("dd.work_name a").unwrap();
    let maker_sel = Selector::parse("dd.maker_name > a").unwrap();
    let author_sel = Selector::parse("dd.maker_name span.author a").unwrap();
    let cover_sel = Selector::parse("dt.work_thumb img, .work_thumb_inner img").unwrap();
    let dl_sel = Selector::parse("dd.work_dl span[class^=_dl_count]").unwrap();

    let items: Vec<AsmrOnlineWork> = doc
        .select(&li_sel)
        .filter_map(|li| {
            let rj = li.value().attr("data-list_item_product_id")?.to_string();
            let title = li
                .select(&name_sel)
                .next()
                .and_then(|a| a.value().attr("title").or_else(|| a.text().next()))
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .unwrap_or(&rj)
                .to_string();
            let circle_name = li.select(&maker_sel).next().and_then(|a| a.text().next()).map(str::trim).map(String::from);
            // 声优：maker_name 内 span.author 的链接列表
            let vas: Vec<String> = li
                .select(&author_sel)
                .filter_map(|a| a.text().next())
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(String::from)
                .collect();
            // 封面：列表页 img 是 Vue 懒加载占位，真实地址在 :src / :thumb-candidates 属性里
            let cover_url = li
                .select(&cover_sel)
                .find_map(|img| {
                    for key in [":src", "data-src", "src"] {
                        if let Some(v) = img.value().attr(key) {
                            if let Some(u) = extract_img_url(v) {
                                return Some(u);
                            }
                        }
                    }
                    None
                })
                .or_else(|| {
                    // 兜底：thumb-with-ng-filter 组件的 :thumb-candidates="['//img...','//img...']"
                    li.select(&cover_sel).find_map(|img| {
                        img.value()
                            .attr(":thumb-candidates")
                            .and_then(extract_img_url)
                    })
                });
            let dl_count = li
                .select(&dl_sel)
                .next()
                .and_then(|s| s.text().next())
                .and_then(|t| t.trim().replace(',', "").parse::<i64>().ok());
            Some(AsmrOnlineWork {
                id: 0,
                rj_code: Some(rj),
                title,
                circle_name,
                cover_url,
                release: None,
                dl_count,
                rating: None,
                rate_count: None,
                duration_sec: None,
                age_class: None,
                vas,
                tags: Vec::new(),
                has_subtitle: false,
                description: None,
                local_work_id: None,
            })
        })
        .collect();

    // 总条目数（如 90059）→ has_next = page * 30 < total
    let count_sel = Selector::parse("[data-search-result-count]").unwrap();
    let total: i64 = doc
        .select(&count_sel)
        .next()
        .and_then(|d| d.value().attr("data-search-result-count"))
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    let has_next = page * 30 < total && !items.is_empty();

    crate::api::asmr_online::AsmrWorksPage {
        items,
        page,
        has_next,
    }
}

/// Value 辅助（保持与其他模块一致的取值风格）。
#[allow(dead_code)]
fn get_str(v: &Value, key: &str) -> Option<String> {
    v.get(key).and_then(|x| x.as_str()).map(String::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_url_encodes_non_ascii_and_keeps_brackets() {
        let u1 = page_url("release_d", 1);
        assert!(u1.starts_with("https://www.dlsite.com/maniax/fsr/=/language/jp/"));
        assert!(u1.contains("order/release_d/"));
        assert!(u1.contains("sex_category%5B0%5D"));
        assert!(u1.contains("lang_options%5B0%5D/%E6%97%A5%E8%AF%AD"));
        assert!(!u1.contains("/page/"));
        let u2 = page_url("trend", 2);
        assert!(u2.contains("order/trend/"));
        assert!(u2.ends_with("/page/2"));
    }

    #[test]
    fn normalize_order_falls_back_to_release() {
        assert_eq!(normalize_order(None), "release_d");
        assert_eq!(normalize_order(Some("release")), "release_d");
        assert_eq!(normalize_order(Some("bogus")), "release_d");
        for o in ORDERS {
            assert_eq!(normalize_order(Some(o)), o);
        }
    }

    #[test]
    fn urlencode_path_keeps_safe_chars() {
        assert_eq!(urlencode_path("fsr/=/a[0]/中文(简体字)"), "fsr/=/a%5B0%5D/%E4%B8%AD%E6%96%87(%E7%AE%80%E4%BD%93%E5%AD%97)");
    }
}
