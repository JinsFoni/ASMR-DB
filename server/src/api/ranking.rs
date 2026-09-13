use scraper::{ElementRef, Html, Selector};

use crate::api::proxy::{ProxyConfig, ProxyTarget};
use crate::db::models::RankingEntry;
use crate::handlers::import::extract_rj_code;

const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0 Safari/537.36";

/// 榜单周期（日/周/月/年度/累计），同时是 DLsite URL 的路径段。
pub const RANKING_TERMS: [&str; 5] = ["day", "week", "month", "year", "total"];

/// DLsite 音声作品（ボイス・ASMR）排行榜页面 URL，每个周期固定返回 Top 100。
pub fn ranking_url(term: &str) -> String {
    format!("https://www.dlsite.com/maniax/ranking/{term}?category=voice")
}

/// 抓取并解析指定周期的音声作品排行榜。
pub fn fetch_ranking(term: &str, proxy: &ProxyConfig) -> Result<Vec<RankingEntry>, String> {
    if !RANKING_TERMS.contains(&term) {
        return Err(format!("未知的榜单周期: {term}"));
    }
    let builder = reqwest::blocking::Client::builder()
        .user_agent(USER_AGENT)
        .redirect(reqwest::redirect::Policy::limited(5))
        .timeout(std::time::Duration::from_secs(30));
    let client = proxy.apply_blocking(builder, ProxyTarget::Dlsite).build().map_err(|e| e.to_string())?;

    let url = ranking_url(term);
    let resp = client.get(&url).send().map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {} while fetching {}", resp.status(), url));
    }
    let html = resp.text().map_err(|e| e.to_string())?;
    Ok(parse_ranking(&html))
}

/// 解析排行榜页面 HTML。
///
/// 页面结构：`<table id="ranking_table">` 内每个 `<tr>` 为一个作品——
/// 排名在 `td .rank_no`，作品链接 `dt.work_name a`，社团 `dd.maker_name > a`，
/// 价格 `dd.work_price_wrap .work_price`，封面完整大图藏在缩略图 Vue 组件的
/// `:src` 属性里（`is_show ? '//img.dlsite.jp/modpub/...' : ...`），
/// 发售日 / 贩売数 / 评分在 `td.work_1col_right ul.work_info_box`，标签在 `dd.search_tag`。
pub fn parse_ranking(html: &str) -> Vec<RankingEntry> {
    let doc = Html::parse_document(html);
    let Ok(row_sel) = Selector::parse("#ranking_table tr") else {
        return Vec::new();
    };

    let mut out = Vec::new();
    for tr in doc.select(&row_sel) {
        let Some(entry) = parse_row(&tr, out.len() as i64 + 1) else {
            continue;
        };
        out.push(entry);
    }
    out
}

fn parse_row(tr: &ElementRef, fallback_rank: i64) -> Option<RankingEntry> {
    // --- 排名（无排名的行视为表头/广告，跳过） ---
    let rank = text_of(tr, ".rank_no")
        .and_then(|t| digits_of(&t))
        .and_then(|d| d.parse::<i64>().ok())
        .unwrap_or(fallback_rank);

    // --- RJ 号 + 标题 ---
    let a_sel = Selector::parse("dt.work_name a[href*='product_id']").ok()?;
    let a = tr.select(&a_sel).next()?;
    let href = a.value().attr("href").unwrap_or("");
    let rj_code = extract_rj_code(href)?;
    let title = Some(a.text().collect::<String>().trim().to_string())
        .filter(|t| !t.is_empty());

    // --- 社团（dd.maker_name 的直接子 a；声优链接嵌在 span.author 里，需排除） ---
    let circle_name = Selector::parse("dd.maker_name > a")
        .ok()
        .and_then(|sel| tr.select(&sel).next())
        .map(|e| e.text().collect::<String>().trim().to_string())
        .filter(|t| !t.is_empty());

    // --- 价格（含折扣价的当前售价） ---
    let price = text_of(tr, "dd.work_price_wrap .work_price").and_then(|t| digits_of(&t));
    let price = price.and_then(|d| d.parse::<i64>().ok());

    // --- 发售日（去掉「販売日:」标签前缀，否则标签里的“日”字会干扰日期解析） ---
    let sale_date = text_of(tr, "li.sales_date").and_then(|t| {
        let tail = t.rsplit_once(':').map(|(_, tail)| tail).unwrap_or(&t);
        crate::api::scraper::extract_date(tail)
    });

    // --- 贩売数（累计下载量） ---
    let dl_count = text_of(tr, "li.work_dl").and_then(|t| digits_of(&t));
    let dl_count = dl_count.and_then(|d| d.parse::<i64>().ok());

    // --- 评分（star_rating 的 star_NN 类，NN/10 即评分） ---
    let rating = Selector::parse("li.work_rating .star_rating")
        .ok()
        .and_then(|sel| tr.select(&sel).next())
        .and_then(|e| {
            e.value()
                .classes()
                .find(|c| c.starts_with("star_"))
                .and_then(|c| c.trim_start_matches("star_").parse::<f64>().ok())
        })
        .map(|v| v / 10.0);

    // --- 标签 ---
    let tags = Selector::parse("dd.search_tag a")
        .map(|sel| {
            tr.select(&sel)
                .map(|e| e.text().collect::<String>().trim().to_string())
                .filter(|t| !t.is_empty())
                .collect()
        })
        .unwrap_or_default();

    // --- 封面大图（hover 弹出层里的完整主图） ---
    let cover_url = cover_of(tr);

    Some(RankingEntry {
        rank,
        rj_code,
        title,
        circle_name,
        cover_url,
        price,
        sale_date,
        dl_count,
        rating,
        tags,
    })
}

/// 从缩略图区域提取完整封面图 URL。
/// 优先取 `:src` 属性里的 modpub 主图（原始比例），回退到 `:thumb-candidates` 方形缩略图。
fn cover_of(tr: &ElementRef) -> Option<String> {
    if let Ok(img_sel) = Selector::parse("img") {
        for img in tr.select(&img_sel) {
            if let Some(v) = img.value().attr(":src") {
                let url = v
                    .split('\'')
                    .find(|s| s.contains("img.dlsite.jp") && s.contains("_img_main"));
                if let Some(url) = url {
                    return Some(absolute(url));
                }
            }
        }
    }
    // 回退：thumb-with-ng-filter 的候选缩略图（240x240 方图）
    if let Ok(thumb_sel) = Selector::parse("thumb-with-ng-filter") {
        for el in tr.select(&thumb_sel) {
            if let Some(v) = el.value().attr(":thumb-candidates") {
                let url = v
                    .split('\'')
                    .find(|s| s.contains("img.dlsite.jp") && s.ends_with(".jpg"));
                if let Some(url) = url {
                    return Some(absolute(url));
                }
            }
        }
    }
    None
}

fn absolute(url: &str) -> String {
    if url.starts_with("//") {
        format!("https:{url}")
    } else {
        url.to_string()
    }
}

fn text_of(tr: &ElementRef, sel_str: &str) -> Option<String> {
    let sel = Selector::parse(sel_str).ok()?;
    let text = tr
        .select(&sel)
        .next()?
        .text()
        .collect::<String>()
        .trim()
        .to_string();
    Some(text).filter(|t| !t.is_empty())
}

/// 提取字符串中的数字（跳过「販売数:」「円」「,」等修饰）。
fn digits_of(s: &str) -> Option<String> {
    let d: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
    if d.is_empty() { None } else { Some(d) }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 按真实页面结构裁剪的最小样例（一行含折扣/评分/标签，一行缺省字段）。
    const SAMPLE_HTML: &str = r##"
    <html><body>
    <table cellspacing="0" id="ranking_table" class="ranking_worklist n_worklist">
      <tbody>
        <tr class="bg_trans">
          <td class="ranking_count">
            <div class="ranking_count_inner">
              <div class="rank_no type_1">1</div>
              <div class="dl_count"><span class="dl_count_label">販売数</span>1,992</div>
            </div>
          </td>
          <td class="work_1col_thumb">
            <div class="work_thumb">
              <thumb-with-ng-filter
                link="https://www.dlsite.com/maniax/work/=/product_id/RJ01675713.html"
                :thumb-candidates="['//img.dlsite.jp/resize/images2/work/doujin/RJ01676000/RJ01675713_img_main_240x240.webp','//img.dlsite.jp/resize/images2/work/doujin/RJ01676000/RJ01675713_img_main_240x240.jpg']"
                alt="タイトルA [マヨタマ]">
                <div class="work_img_popover">
                  <img src="data:image/gif;base64,R0lGODlhAQABAGAAACH5BAEKAP8ALAAAAAABAAEAAAgEAP8FBAA7"
                    :src="is_show ? '//img.dlsite.jp/modpub/images2/work/doujin/RJ01676000/RJ01675713_img_main.jpg' : 'data:image/gif;base64,R0lGODlhAQABAGAAACH5BAEKAP8ALAAAAAABAAEAAAgEAP8FBAA7'" />
                </div>
              </thumb-with-ng-filter>
              <div class="work_category type_SOU">ボイス・ASMR</div>
            </div>
          </td>
          <td>
            <dl class="work_1col">
              <dt class="work_name">
                <a href="https://www.dlsite.com/maniax/work/=/product_id/RJ01675713.html">作品タイトルA</a>
              </dt>
              <dd class="maker_name">
                <a href="https://www.dlsite.com/maniax/circle/profile/=/maker_id/RG68316.html">マヨタマ</a>
                <span class="separator">/</span>
                <span class="author"><a href="#x">大山チロル</a>&nbsp;<a href="#y">陽向葵ゅか</a></span>
              </dd>
              <dd class="work_price_wrap">
                <span class="work_price discount">2,640<i>円</i></span>
                <span class="strike">3,300<i>円</i></span>
              </dd>
              <dd class="search_tag">
                <a href="#g46">ハーレム</a>
                <a href="#g04">ラブラブ/あまあま</a>
              </dd>
            </dl>
          </td>
          <td class="work_1col_right">
            <ul class="work_info_box">
              <li class="sales_date">販売日:&nbsp;2026年09月13日</li>
              <li class="work_dl clear">
                <div class="_work_dl_RJ01675713">販売数:&nbsp;<span class="_dl_count_RJ01675713">3,032</span></div>
              </li>
              <li class="work_rating"><div class="star_rating star_45">(67)</div></li>
            </ul>
          </td>
        </tr>
        <tr class="">
          <td class="ranking_count"><div class="ranking_count_inner"><div class="rank_no">2</div></div></td>
          <td class="work_1col_thumb"><div class="work_thumb"></div></td>
          <td>
            <dl class="work_1col">
              <dt class="work_name"><a href="https://www.dlsite.com/maniax/work/=/product_id/RJ01683949.html">作品B</a></dt>
              <dd class="maker_name"><a href="#m">サークルB</a></dd>
            </dl>
          </td>
          <td class="work_1col_right"><ul class="work_info_box"></ul></td>
        </tr>
        <tr class="">
          <td class="ranking_count"><div class="ranking_count_inner"><div class="rank_no">2</div></div></td>
          <td class="work_1col_thumb"><div class="work_thumb"></div></td>
          <td>
            <dl class="work_1col">
              <dt class="work_name"><a href="https://www.dlsite.com/maniax/work/=/product_id/RJ01999999.html">作品C</a></dt>
              <dd class="maker_name"><a href="#m">サークルC</a></dd>
            </dl>
          </td>
          <td class="work_1col_right"><ul class="work_info_box"></ul></td>
        </tr>
      </tbody>
    </table>
    </body></html>
    "##;

    #[test]
    fn parse_sample_ranking() {
        let items = parse_ranking(SAMPLE_HTML);
        assert_eq!(items.len(), 3);

        let first = &items[0];
        assert_eq!(first.rank, 1);
        assert_eq!(first.rj_code, "RJ01675713");
        assert_eq!(first.title.as_deref(), Some("作品タイトルA"));
        assert_eq!(first.circle_name.as_deref(), Some("マヨタマ"));
        assert_eq!(first.price, Some(2640));
        assert_eq!(first.sale_date.as_deref(), Some("2026-09-13"));
        assert_eq!(first.dl_count, Some(3032));
        assert_eq!(first.rating, Some(4.5));
        assert_eq!(
            first.cover_url.as_deref(),
            Some("https://img.dlsite.jp/modpub/images2/work/doujin/RJ01676000/RJ01675713_img_main.jpg")
        );
        assert_eq!(first.tags, vec!["ハーレム", "ラブラブ/あまあま"]);

        // 缺省字段的行：字段为 None，不 panic
        let second = &items[1];
        assert_eq!(second.rank, 2);
        assert_eq!(second.rj_code, "RJ01683949");
        assert_eq!(second.circle_name.as_deref(), Some("サークルB"));
        assert_eq!(second.price, None);
        assert_eq!(second.rating, None);
        assert_eq!(second.cover_url, None);
        assert!(second.tags.is_empty());

        // DLsite 榜单存在并列名次（如三作品并列第 N），rank 可重复
        let third = &items[2];
        assert_eq!(third.rank, 2);
        assert_eq!(third.rj_code, "RJ01999999");
    }

    #[test]
    fn rejects_unknown_term() {
        assert!(fetch_ranking("hourly", &ProxyConfig::default()).is_err());
    }
}
