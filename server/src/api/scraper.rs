use crate::db::models::ScrapedWork;
use crate::api::proxy::{ProxyConfig, ProxyTarget};
use scraper::{Html, Selector};
use std::error::Error;

const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0 Safari/537.36";

/// Build the DLsite product page URL for an RJ code.
pub fn product_url(rj_code: &str) -> String {
    // RJ* → maniax (R18), non-RJ and BJ* → maniax as well; home is all-ages
    let rj = rj_code.trim().to_uppercase();
    // locale=zh_CN：作品页返回简体中文标签（治愈/低语…），表格标题同为中文（发售日/分类…）
    format!(
        "https://www.dlsite.com/maniax/work/=/product_id/{}.html?locale=zh_CN",
        rj
    )
}

/// 仅抓取 DLsite 作品页的完整「作品内容」简介（HTML 转纯文本，保留换行）。
/// 供 ASMR ONE 在线详情页补充展示：asmr.one 的 API 不提供作品简介，
/// og:description 的 meta 又只有截断的一句话，只有作品页正文才是完整简介。
pub fn fetch_work_description(rj_code: &str, proxy: &ProxyConfig) -> Result<String, String> {
    let rj = rj_code.trim().to_uppercase();
    let client = crate::api::http_client::shared_blocking_client("scraper-dlsite", proxy, || {
        let builder = reqwest::blocking::Client::builder()
            .user_agent(USER_AGENT)
            .redirect(reqwest::redirect::Policy::limited(5))
            .timeout(std::time::Duration::from_secs(30));
        proxy
            .apply_blocking(builder, ProxyTarget::Dlsite)
            .build()
            .map_err(|e| format!("构建请求客户端失败: {e}"))
    })?;

    let url = product_url(&rj);
    let resp = client
        .get(&url)
        .send()
        .map_err(|e| format!("请求 DLsite 页面失败: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("DLsite 页面返回 {}", resp.status()));
    }
    let html = resp.text().map_err(|e| format!("读取响应失败: {e}"))?;
    let doc = Html::parse_document(&html);

    // 完整简介容器：itemprop="description"（class="work_parts_container"），
    // 依次尝试该容器内段落、容器全文、og:description 兜底
    let container_sel = Selector::parse("[itemprop='description']")
        .or_else(|_| Selector::parse(".work_parts_container"))
        .map_err(|e| e.to_string())?;
    let p_sel = Selector::parse("p").map_err(|e| e.to_string())?;
    let br_sel = Selector::parse("br").map_err(|e| e.to_string())?;

    let mut text = String::new();
    if let Some(container) = doc.select(&container_sel).next() {
        for p in container.select(&p_sel) {
            // zh_CN 页容器尾部会混入包着页面脚本/模板的 <p>（DataDog 注入、Vue 组件等），
            // 含 script/style 的段落整段跳过
            if Selector::parse("script, style")
                .map(|s| p.select(&s).next().is_some())
                .unwrap_or(false)
            {
                continue;
            }
            // 用 inner_html 按 <br> 拆行后剥离标签，保留原文换行结构
            let inner = p.inner_html();
            let html_line = inner
                .replace("<br>", "\n")
                .replace("<br/>", "\n")
                .replace("<br />", "\n");
            let line = strip_tags(&html_line);
            // 元素间换行数（<br> 可能连续出现多个），压掉段首尾空白
            let line = line.trim();
            // 正文段落不会出现的 JS 代码特征，或超长行（页面脚本文本）→ 丢弃
            let looks_like_code = line.contains("function(") || line.contains("$(function") || line.contains("var ");
            if looks_like_code || line.chars().count() > 2000 {
                continue;
            }
            let br_count = p.select(&br_sel).count();
            if !line.is_empty() {
                if !text.is_empty() {
                    text.push('\n');
                }
                text.push_str(line);
                // 连续多个 <br> 保留为空行
                for _ in 1..br_count.min(2) {
                    text.push('\n');
                }
            }
        }
        if text.is_empty() {
            // 容器内没有 <p> 时直接取容器文本
            text = container.text().collect::<String>().trim().to_string();
        }
    }
    if text.is_empty() {
        return Err("DLsite 页面未找到作品简介".to_string());
    }
    Ok(text)
}

/// 剥离 HTML 标签，只留文本与实体解码后的内容（简介 <p> 内已按 <br> 拆行）。
fn strip_tags(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut in_tag = false;
    let mut chars = html.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            '&' if !in_tag => {
                // 解码常见实体
                let rest: String = chars.clone().collect();
                let entity = rest.split('&').next().unwrap_or("");
                let decoded = match entity.split(';').next() {
                    Some("amp") => Some('&'),
                    Some("lt") => Some('<'),
                    Some("gt") => Some('>'),
                    Some("quot") => Some('"'),
                    Some("nbsp") => Some(' '),
                    _ => None,
                };
                if let Some(d) = decoded {
                    out.push(d);
                    let skip = entity.find(';').map(|i| i + 1).unwrap_or(0);
                    for _ in 0..skip {
                        chars.next();
                    }
                } else {
                    out.push('&');
                }
            }
            c if !in_tag => out.push(c),
            _ => {}
        }
    }
    out
}

/// Fetch and parse metadata for a work given its RJ code.
pub fn fetch_work_metadata(rj_code: &str, proxy: &ProxyConfig) -> Result<ScrapedWork, Box<dyn Error>> {
    let rj = rj_code.trim().to_uppercase();
    let client = crate::api::http_client::shared_blocking_client("scraper-dlsite", proxy, || {
        let builder = reqwest::blocking::Client::builder()
            .user_agent(USER_AGENT)
            .redirect(reqwest::redirect::Policy::limited(5))
            .timeout(std::time::Duration::from_secs(30));
        proxy
            .apply_blocking(builder, ProxyTarget::Dlsite)
            .build()
            .map_err(|e| format!("构建请求客户端失败: {e}"))
    })?;

    let url = product_url(&rj);
    let resp = client.get(&url).send()?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {} while fetching {}", resp.status(), url).into());
    }
    let html = resp.text()?;
    let doc = Html::parse_document(&html);

    let mut w = ScrapedWork {
        rj_code: rj.clone(),
        dlsite_url: Some(url),
        work_type: guess_work_type(&rj),
        ..Default::default()
    };

    // --- Title (from og:title or <title>) ---
    w.title_ja = meta_content(&doc, "og:title")
        .or_else(|| page_title(&doc))
        .map(|t| clean_title(&t));

    // --- Cover image ---
    w.cover_url = meta_content(&doc, "og:image");

    // --- Description ---
    // og:description 是「简介前半句 + 站点广告」拼接（…立即欣赏！每天都有新作品上架…），
    // 把广告段剥掉，只留作品自身的简介
    w.description = meta_content(&doc, "og:description")
        .or_else(|| meta_content(&doc, "description"))
        .map(|d| strip_site_ad(&d));

    // --- Circle name / id ---
    if let Some(circle) = find_circle(&doc) {
        w.circle_name = Some(circle.name);
        w.circle_id = circle.id;
    }

    // --- Table-driven fields: 販売日 / 年齢指定 / 声優 / ジャンル / ファイル容量 / 作品形式 / ファイルサイズ ---
    parse_info_table(&doc, &mut w);

    // --- 售出 / 收藏 / 评分：作品页的 DL 数和评分是 Vue 异步渲染抓不到，走作品信息接口 ---
    fetch_work_counts(&client, &rj, &mut w);

    // --- 评分兜底：翻译作品（?translation=）的评分聚合在原版作品上，信息接口对翻译版返回空，
    //     但作品页 HTML 里的 schema.org aggregateRating 微数据始终带聚合后的评分 ---
    if w.rating.is_none() {
        w.rating = meta_itemprop_content(&doc, "ratingValue").and_then(|s| s.parse::<f64>().ok());
    }

    // --- Multi-language titles ---
    parse_localized_titles(&doc, &mut w);

    Ok(w)
}

/// 从 product/info/ajax 拉取售出数（dl_count）、收藏数（wishlist_count）与精确评分（rate_average_2dp）。
/// 接口失败时保留 None，不影响主流程。
fn fetch_work_counts(client: &reqwest::blocking::Client, rj: &str, w: &mut ScrapedWork) {
    const PRODUCT_INFO_URL: &str = "https://www.dlsite.com/maniax/product/info/ajax";
    let Ok(resp) = client.get(PRODUCT_INFO_URL).query(&[("product_id", rj)]).send() else {
        return;
    };
    if !resp.status().is_success() {
        return;
    }
    let Ok(json) = resp.json::<serde_json::Value>() else {
        return;
    };
    if let Some(v) = json.get(rj) {
        w.dl_count = v.get("dl_count").and_then(|x| x.as_i64());
        w.wishlist_count = v.get("wishlist_count").and_then(|x| x.as_i64());
        w.rating = v.get("rate_average_2dp").and_then(|x| x.as_f64());
    }
}

/// Parse `<meta property="og:xx" content="...">` or `<meta name="xx">`.
fn meta_content(doc: &Html, key: &str) -> Option<String> {
    let sel = Selector::parse(&format!(
        "meta[property=\"{}\"], meta[name=\"{}\"]",
        key, key
    ))
    .ok()?;
    doc.select(&sel)
        .filter_map(|e| e.value().attr("content"))
        .next()
        .map(|s| s.trim().to_string())
}

/// Parse `<meta itemprop="xx" content="...">`（schema.org 微数据，如 aggregateRating/ratingValue）。
fn meta_itemprop_content(doc: &Html, key: &str) -> Option<String> {
    let sel = Selector::parse(&format!("meta[itemprop=\"{}\"]", key)).ok()?;
    doc.select(&sel)
        .filter_map(|e| e.value().attr("content"))
        .next()
        .map(|s| s.trim().to_string())
}

fn page_title(doc: &Html) -> Option<String> {
    let sel = Selector::parse("title").ok()?;
    doc.select(&sel).next().map(|e| e.text().collect::<String>())
}

/// 剥离 og:description 尾部拼接的 DLsite 站点广告段。
/// 简中版形如「…委ねよう！「DLsite 同人 - R18」是同人志・…下载商城「DLsite」！」，
/// 日文版形如「…ダウンロードショップ。お気に入りの作品を…」。
fn strip_site_ad(d: &str) -> String {
    let markers = [
        // 简中版：广告以「DLsite 同人」起头
        "「DLsite 同人 - R18」是同人志",
        "「DLsite 同人」是同人志",
        // 日文版：广告以「同人誌・同人ゲーム…」起头，前面是句号或引号闭合
        "同人誌・同人ゲーム・同人ボイス・ASMRのダウンロードショップ",
    ];
    let mut out = d.trim().to_string();
    for m in markers {
        if let Some(i) = out.find(m) {
            out.truncate(i);
            break;
        }
    }
    out.trim_end_matches(['、', '。', '！', '~', '」', ' ', '\n'])
        .to_string()
}

/// Strip trailing " - DLsite ..." / " - DLsiteR18..." from a title.
fn clean_title(t: &str) -> String {
    let mut t = t.trim();
    // 末尾站点尾缀：「… | DLsite」「… - DLsiteR18」等（og:title 新旧格式）
    for marker in [" | DLsite", " - DLsiteR18", " - DLsite R18", " - DLsite", " -DLsite", " | DLsite FANZA"] {
        if let Some(idx) = t.rfind(marker) {
            // 只在末尾才算尾缀，避免误删正文中出现的同形文本
            if idx + marker.len() == t.len() {
                t = t[..idx].trim_end();
            }
        }
    }
    // 标题自带的品牌方括号后缀：「… [社团名/ DLsite]」（og:title 新格式）
    if let Some(idx) = t.rfind(" [") {
        if t.ends_with(']') && idx + 2 < t.len() {
            t = t[..idx].trim_end();
        }
    }
    // 旧格式波浪线后缀
    for marker in ["〜DLsite", "～DLsite"] {
        if let Some(idx) = t.find(marker) {
            t = t[..idx].trim_end();
        }
    }
    t.to_string()
}

struct Circle {
    name: String,
    id: Option<String>,
}

/// DLsite 页面里社团链接形如 `https://www.dlsite.com/maniax/circle/profile/=/maker_id/RG12345.html`
/// 或 `<a class="maker_name">サークル名</a>`。
fn find_circle(doc: &Html) -> Option<Circle> {
    let anchor = Selector::parse(
        "a.maker_name, .work_maker a, a[href*='circle/profile'], a[href*='maker_id']",
    )
    .ok()?;
    for e in doc.select(&anchor) {
        let name = e.text().collect::<String>().trim().to_string();
        if name.is_empty() {
            continue;
        }
        let href = e.value().attr("href").unwrap_or("");
        let id = href
            .split("maker_id/")
            .nth(1)
            .map(|s| s.trim_end_matches(".html").to_string())
            .filter(|s| !s.is_empty());
        return Some(Circle { name, id });
    }
    None
}

/// Scan the product info table for labelled rows.
fn parse_info_table(doc: &Html, w: &mut ScrapedWork) {
    // 注意：现代页面 `<table id="work_outline">` 本身即表格（tr 直接在其下，
    // 中间没有再嵌套 table），因此 th 直接在 tr 内查找即可。
    let Some(th_sel) = Selector::parse("th").ok() else {
        return;
    };
    let Some(tr_sel) = Selector::parse("#work_outline tr, .work_outline tr").ok() else {
        return;
    };

    for tr in doc.select(&tr_sel) {
        let Some(th_el) = tr.select(&th_sel).next() else {
            continue;
        };
        let label = th_el.text().collect::<String>().trim().to_string();
        let td_sel = match Selector::parse("td") {
            Ok(s) => s,
            Err(_) => continue,
        };
        let value: String = tr
            .select(&td_sel)
            .next()
            .map(|e| e.text().collect::<Vec<_>>().join(" ").trim().to_string())
            .unwrap_or_default();
        if value.is_empty() {
            continue;
        }

        // 表格标题随 locale 变化：日文（販売日/年齢指定…）与简体中文（发售日/年龄指定…）都接受
        match label {
            l if l.contains("販売日") || l.contains("配信日") || l.contains("发售日") => {
                w.sale_date = extract_date(&value);
            }
            l if l.contains("年齢指定") || l.contains("年龄指定") => {
                // 中文页年龄值是「全年龄 / R18 / R-18G」，日文页含「18」判定即可
                w.age_class = Some(if value.contains("R-18G") || value.contains("R‐18G") {
                    "r18g".to_string()
                } else if value.contains("18") {
                    "r18".to_string()
                } else {
                    "general".to_string()
                });
            }
            l if l.contains("声優") || l.contains("声优") => {
                w.actors = split_list(&value);
            }
            l if l.contains("ジャンル")
                || l.contains("作品内容")
                || l.contains("分类")
                || l.contains("分類") =>
            {
                // 全库作品即 ASMR，单标的「ASMR」无信息量，屏蔽
                w.tags = split_list(&value)
                    .into_iter()
                    .filter(|t| t != "ASMR")
                    .collect();
            }
            l if l.contains("ファイル容量")
                || l.contains("ファイルサイズ")
                || l.contains("文件容量") =>
            {
                w.file_size_mb = parse_file_size(&value);
            }
            l if l.contains("作品形式") || l.contains("ファイル形式") || l.contains("文件形式") => {
                if w.duration_min.is_none() {
                    // 形式字段可能是 MP3/FLAC/WAV + 収録時間
                    w.duration_min = parse_duration_from_text(&value);
                }
            }
            l if l.contains("収録時間") || l.contains("收录时间") || l.contains("收錄時間") => {
                w.duration_min = parse_duration_from_text(&value);
            }
            _ => {}
        }
    }
}

pub(crate) fn extract_date(s: &str) -> Option<String> {
    // YYYY年MM月DD日 (注意 年/月/日 是多字节字符，用字符边界偏移)
    if let Some(i) = s.find('年') {
        if let Some(j) = s.find('月') {
            if let Some(k) = s.find('日') {
                let m_start = i + '年'.len_utf8();
                let d_start = j + '月'.len_utf8();
                if i >= 4 && j > m_start && k > d_start {
                    let y = s[i - 4..i].trim();
                    let m = s[m_start..j].trim();
                    let d = s[d_start..k].trim();
                    if y.bytes().all(|c| c.is_ascii_digit())
                        && m.bytes().all(|c| c.is_ascii_digit())
                        && d.bytes().all(|c| c.is_ascii_digit())
                    {
                        return Some(format!("{}-{:0>2}-{:0>2}", y, m, d));
                    }
                }
            }
        }
    }
    // YYYY/MM/DD or YYYY-MM-DD
    let bytes = s.as_bytes();
    let finder = |sep: u8| -> Option<String> {
        let idxs: Vec<usize> = bytes
            .iter()
            .enumerate()
            .filter(|(_, &c)| c == sep)
            .map(|(i, _)| i)
            .collect();
        if idxs.len() >= 2 {
            let (i1, i2) = (idxs[0], idxs[1]);
            if i1 >= 4 && i2 > i1 + 1 && i2 + 1 < bytes.len() {
                let y = &s[i1 - 4..i1];
                let m = &s[i1 + 1..i2];
                let d = &s[i2 + 1..bytes.len().min(i2 + 3)];
                if y.bytes().all(|c| c.is_ascii_digit())
                    && m.bytes().all(|c| c.is_ascii_digit())
                    && d.bytes().all(|c| c.is_ascii_digit())
                {
                    return Some(format!("{}-{:0>2}-{:0>2}", y, m, d));
                }
            }
        }
        None
    };
    if let Some(v) = finder(b'/') {
        return Some(v);
    }
    finder(b'-')
}

fn split_list(s: &str) -> Vec<String> {
    s.split([' ', '、', '，', '/', '\n'])
        .map(|x| x.trim().to_string())
        .filter(|x| !x.is_empty() && x.len() > 1)
        .collect()
}

/// Parse a value like "10.00MB" / "1.2GB" / "約 350MB" into MB.
pub(crate) fn parse_file_size(s: &str) -> Option<f64> {
    let s = s.replace([' ', '約'], "");
    let up = s.to_uppercase();
    for (unit, mult) in [("GB", 1024.0), ("MB", 1.0), ("KB", 1.0 / 1024.0)] {
        if let Some(i) = up.find(unit) {
            let num = up[..i].trim();
            if let Ok(v) = num.parse::<f64>() {
                return Some(v * mult);
            }
        }
    }
    None
}

/// Parse a value like "120分" / "約120分" / "02:00:00" into minutes.
fn parse_duration_from_text(s: &str) -> Option<i64> {
    let s = s.replace('約', "");
    if let Some(i) = s.find('分') {
        let num = s[..i].trim();
        if let Ok(v) = num.parse::<i64>() {
            return Some(v);
        }
    }
    // hh:mm:ss
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() == 3 {
        if let (Ok(h), Ok(m), Ok(sec)) = (
            parts[0].trim().parse::<i64>(),
            parts[1].trim().parse::<i64>(),
            parts[2].trim().parse::<i64>(),
        ) {
            return Some(h * 60 + m + if sec > 0 { 1 } else { 0 });
        }
    }
    None
}

/// Localized titles live in hidden language-switch spans like `<span class="work-title-zh">` or
/// `<p class="title_en">`. Best-effort extraction.
fn parse_localized_titles(doc: &Html, w: &mut ScrapedWork) {
    // 注意不能裸用 [lang='zh-CN']：zh_CN 页的 <html lang="zh-CN"> 会整体命中，
    // 其 text() 是全文档文本（含 <script>），数万字符垃圾会被写进 title_zh。
    // 只匹配行内元素，html/body 这类容器级 lang 排除在外。
    let zh_sel = Selector::parse(
        "span[lang='zh-CN'], p[lang='zh-CN'], div[lang='zh-CN'], .work-title-zh, .work_title_zh, #work_name_zh",
    )
    .ok();
    if let Some(sel) = zh_sel {
        if let Some(e) = doc.select(&sel).next() {
            let t = e.text().collect::<String>().trim().to_string();
            // 标题长度保险：正常作品标题不会超过 200 字符，超长必是误抓的容器文本
            if !t.is_empty() && t.chars().count() <= 200 {
                w.title_zh = Some(t);
            }
        }
    }
    let en_sel = Selector::parse("span[lang='en'], p[lang='en'], .work-title-en, #work_name_en").ok();
    if let Some(sel) = en_sel {
        if let Some(e) = doc.select(&sel).next() {
            let t = e.text().collect::<String>().trim().to_string();
            if !t.is_empty() && t.chars().count() <= 200 {
                w.title_en = Some(t);
            }
        }
    }
}

pub(crate) fn guess_work_type(rj: &str) -> String {
    let prefix: String = rj.chars().take(2).collect::<String>().to_uppercase();
    match prefix.as_str() {
        "RE" => "game".to_string(),
        "BJ" => "comic".to_string(),
        "VJ" => "video".to_string(),
        _ => "voice".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_HTML: &str = r#"
    <html>
    <head>
      <meta property="og:title" content="RJ01014447 【耳かき】悪魔娘が癒す〜DLsiteR18" />
      <meta property="og:image" content="https://img.dlsite.jp/modpub/images2/work/doujin/RJ01010000/RJ01014447_img_main.jpg" />
      <meta name="description" content="悪魔娘が疲れたあなたを癒してくれる耳かきASMR。集中力と癒しを届けます。" />
      <title>RJ01014447 【耳かき】悪魔娘が癒す - DLsiteR18</title>
    </head>
    <body>
      <div id="work_maker" class="work_maker">
        <a class="maker_name" href="/maniax/circle/profile/=/maker_id/RG12345.html">サークル・魔王城</a>
      </div>
      <div id="work_outline">
        <table class="work_table">
          <tr><th>販売日</th><td>2024年12月31日</td></tr>
          <tr><th>年齢指定</th><td>18禁</td></tr>
          <tr><th>作品形式</th><td>音声・ASMR</td></tr>
          <tr><th>ファイル容量</th><td>約 1.20GB</td></tr>
          <tr><th>ジャンル</th><td>耳かき 添い寝 囁き 安眠</td></tr>
          <tr><th>収録時間</th><td>約120分</td></tr>
          <tr><th>声優</th><td>佐倉綾音 日笠陽子</td></tr>
        </table>
      </div>
    </body>
    </html>
    "#;

    /// 真实页面结构：`<table id="work_outline">` 本身即表格（tr 直接在其下）。
    const SAMPLE_HTML_V2: &str = r##"
    <html>
    <head><title>【15%OFF】テスト作品 [サークルX] | DLsite 同人 - R18</title></head>
    <body>
      <table cellspacing="0" id="work_outline">
        <tr><th>販売日</th><td><a href="#">2026年09月11日 0時</a></td></tr>
        <tr><th>声優</th><td><a href="#">都みみち</a> / <a href="#">ありのりあ</a></td></tr>
        <tr><th>年齢指定</th><td><div class="work_genre"><span class="icon_ADL" title="R18">R18</span></div></td></tr>
        <tr><th>作品形式</th><td><div class="work_genre"><span title="ボイス・ASMR">ボイス・ASMR</span></div></td></tr>
        <tr><th>ジャンル</th><td><div class="main_genre"><a href="#">主観</a> <a href="#">耳かき</a></div></td></tr>
      </table>
    </body>
    </html>
    "##;

    #[test]
    fn parse_real_page_structure() {
        let doc = Html::parse_document(SAMPLE_HTML_V2);
        let mut w = ScrapedWork {
            rj_code: "RJ01690344".into(),
            ..Default::default()
        };
        parse_info_table(&doc, &mut w);
        assert_eq!(w.sale_date.as_deref(), Some("2026-09-11"));
        assert_eq!(w.age_class.as_deref(), Some("r18"));
        assert!(w.actors.iter().any(|a| a.contains("都みみち")));
        assert!(w.tags.iter().any(|t| t.contains("耳かき")));
    }

    #[test]
    fn parse_sample_page() {
        let doc = Html::parse_document(SAMPLE_HTML);

        let title = meta_content(&doc, "og:title").unwrap();
        assert_eq!(clean_title(&title), "RJ01014447 【耳かき】悪魔娘が癒す");

        let cover = meta_content(&doc, "og:image").unwrap();
        assert!(cover.contains("RJ01014447_img_main.jpg"));

        let circle = find_circle(&doc).unwrap();
        assert_eq!(circle.name, "サークル・魔王城");
        assert_eq!(circle.id, Some("RG12345".to_string()));

        let mut w = ScrapedWork {
            rj_code: "RJ01014447".into(),
            ..Default::default()
        };
        parse_info_table(&doc, &mut w);

        assert_eq!(w.sale_date.as_deref(), Some("2024-12-31"));
        assert_eq!(w.age_class.as_deref(), Some("r18"));
        assert_eq!(w.duration_min, Some(120));
        assert!(w.file_size_mb.is_some());
        assert!(w.file_size_mb.unwrap() > 1000.0);
        assert!(w.actors.iter().any(|a| a.contains("佐倉綾音")));
        assert!(w.tags.iter().any(|t| t.contains("耳かき")));
    }

    #[test]
    fn date_and_size_parsing() {
        assert_eq!(extract_date("2024年12月31日").as_deref(), Some("2024-12-31"));
        assert_eq!(extract_date("2024/12/5").as_deref(), Some("2024-12-05"));
        assert_eq!(extract_date("2024-1-1").as_deref(), Some("2024-01-01"));
        assert_eq!(parse_file_size("約 1.20GB"), Some(1228.8));
        assert_eq!(parse_duration_from_text("約120分"), Some(120));
        assert_eq!(parse_duration_from_text("02:00:00"), Some(120));
    }

    #[test]
    fn rj_type_guessing() {
        assert_eq!(guess_work_type("RJ01014447"), "voice");
        assert_eq!(guess_work_type("RE123456"), "game");
        assert_eq!(guess_work_type("BJ123456"), "comic");
        assert_eq!(guess_work_type("VJ123456"), "video");
    }
}

