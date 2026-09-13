use crate::db::models::ScrapedWork;
use crate::api::proxy::{ProxyConfig, ProxyTarget};
use scraper::{Html, Selector};
use std::error::Error;

const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0 Safari/537.36";

/// Build the DLsite product page URL for an RJ code.
pub fn product_url(rj_code: &str) -> String {
    // RJ* → maniax (R18), non-RJ and BJ* → maniax as well; home is all-ages
    let rj = rj_code.trim().to_uppercase();
    format!("https://www.dlsite.com/maniax/work/=/product_id/{}.html", rj)
}

/// Fetch and parse metadata for a work given its RJ code.
pub fn fetch_work_metadata(rj_code: &str, proxy: &ProxyConfig) -> Result<ScrapedWork, Box<dyn Error>> {
    let rj = rj_code.trim().to_uppercase();
    let builder = reqwest::blocking::Client::builder()
        .user_agent(USER_AGENT)
        .redirect(reqwest::redirect::Policy::limited(5))
        .timeout(std::time::Duration::from_secs(30));
    let client = proxy.apply_blocking(builder, ProxyTarget::Dlsite).build()?;

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
    w.description = meta_content(&doc, "og:description")
        .or_else(|| meta_content(&doc, "description"));

    // --- Circle name / id ---
    if let Some(circle) = find_circle(&doc) {
        w.circle_name = Some(circle.name);
        w.circle_id = circle.id;
    }

    // --- Table-driven fields: 販売日 / 年齢指定 / 声優 / ジャンル / ファイル容量 / 作品形式 / ファイルサイズ ---
    parse_info_table(&doc, &mut w);

    // --- Price ---
    if let Some(p) = find_price(&doc) {
        w.price = Some(p);
    }

    // --- Multi-language titles ---
    parse_localized_titles(&doc, &mut w);

    Ok(w)
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

fn page_title(doc: &Html) -> Option<String> {
    let sel = Selector::parse("title").ok()?;
    doc.select(&sel).next().map(|e| e.text().collect::<String>())
}

/// Strip trailing " - DLsite ..." / " - DLsiteR18..." from a title.
fn clean_title(t: &str) -> String {
    let t = t.trim();
    for marker in [
        "〜DLsite",
        "～DLsite",
        " - DLsiteR18",
        " - DLsite R18",
        " - DLsite",
        " -DLsite",
        " | DLsite",
        " - DLsite FANZA",
    ] {
        if let Some(idx) = t.find(marker) {
            return t[..idx].trim().to_string();
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

        match label {
            l if l.contains("販売日") || l.contains("配信日") => {
                w.sale_date = extract_date(&value);
            }
            l if l.contains("年齢指定") => {
                w.age_class = Some(if value.contains("18") {
                    "r18".to_string()
                } else if value.contains("R-18G") {
                    "r18g".to_string()
                } else {
                    "general".to_string()
                });
            }
            l if l.contains("声優") => {
                w.actors = split_list(&value);
            }
            l if l.contains("ジャンル") || l.contains("作品内容") => {
                w.tags = split_list(&value);
            }
            l if l.contains("ファイル容量") || l.contains("ファイルサイズ") => {
                w.file_size_mb = parse_file_size(&value);
            }
            l if l.contains("作品形式") || l.contains("ファイル形式") => {
                if w.duration_min.is_none() {
                    // 形式字段可能是 MP3/FLAC/WAV + 収録時間
                    w.duration_min = parse_duration_from_text(&value);
                }
            }
            l if l.contains("収録時間") => {
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

fn find_price(doc: &Html) -> Option<i64> {
    // 现代页面价格由 Vue 渲染（静态 HTML 无价格文本），
    // 优先取隐藏 GA4 埋点元素上的 data-price（当前折扣价）。
    if let Ok(sel) = Selector::parse("div[data-price]") {
        for e in doc.select(&sel) {
            if let Some(v) = e.value().attr("data-price") {
                if let Ok(p) = v.trim().parse::<i64>() {
                    return Some(p);
                }
            }
        }
    }
    let price_sel = Selector::parse(
        ".work_price, .buy_price, #price_box .price, span[itemprop='price'], .price-whole",
    )
    .ok()?;
    for e in doc.select(&price_sel) {
        let text = e.text().collect::<String>();
        let digits: String = text.chars().filter(|c| c.is_ascii_digit()).collect();
        if !digits.is_empty() {
            return digits.parse::<i64>().ok();
        }
    }
    None
}

/// Localized titles live in hidden language-switch spans like `<span class="work-title-zh">` or
/// `<p class="title_en">`. Best-effort extraction.
fn parse_localized_titles(doc: &Html, w: &mut ScrapedWork) {
    let zh_sel = Selector::parse(
        ".work-title-zh, [lang='zh-CN'], .work_title_zh, #work_name_zh",
    )
    .ok();
    if let Some(sel) = zh_sel {
        if let Some(e) = doc.select(&sel).next() {
            let t = e.text().collect::<String>().trim().to_string();
            if !t.is_empty() {
                w.title_zh = Some(t);
            }
        }
    }
    let en_sel = Selector::parse(".work-title-en, [lang='en'], #work_name_en").ok();
    if let Some(sel) = en_sel {
        if let Some(e) = doc.select(&sel).next() {
            let t = e.text().collect::<String>().trim().to_string();
            if !t.is_empty() {
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
      <div class="work_price"><span>1,320</span></div>
    </body>
    </html>
    "#;

    /// 真实页面结构：`<table id="work_outline">` 本身即表格（tr 直接在其下），
    /// 价格由 Vue 渲染，仅 GA4 埋点元素上有 data-price。
    const SAMPLE_HTML_V2: &str = r##"
    <html>
    <head><title>【15%OFF】テスト作品 [サークルX] | DLsite 同人 - R18</title></head>
    <body>
      <div data-vue-component="product-price" data-section_name="right_work_price">
        <div hidden class="ga4_event_item_RJ01690344" data-product_id="RJ01690344"
             data-price="1309" data-official_price="1540"></div>
        <div id="work_price"></div>
      </div>
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
        assert_eq!(find_price(&doc), Some(1309), "应从 GA4 data-price 取到折扣价");
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

