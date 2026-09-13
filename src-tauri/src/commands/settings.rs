use crate::db::queries;
use crate::AppState;
use tauri::{Manager, State};

type Db<'a> = State<'a, AppState>;

/// 递归收集目录下的字幕文件（.lrc / .vtt / .srt）。
fn collect_subtitles(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>, depth: u32) {
    if depth > 4 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            collect_subtitles(&p, out, depth + 1);
        } else if let Some(ext) = p.extension().map(|e| e.to_string_lossy().to_lowercase()) {
            if matches!(ext.as_str(), "lrc" | "vtt" | "srt") {
                out.push(p);
            }
        }
    }
}

/// 解码文本：优先 UTF-8，其次 Shift-JIS，最后 UTF-8 lossy。
fn decode_text(bytes: &[u8]) -> String {
    // 去除 UTF-8 BOM
    let bytes = if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        &bytes[3..]
    } else {
        bytes
    };
    match std::str::from_utf8(bytes) {
        Ok(s) => s.to_string(),
        Err(_) => {
            let (cow, _, _) = encoding_rs::SHIFT_JIS.decode(bytes);
            let s = cow.into_owned();
            if s.chars().any(|c| c != '\u{FFFD}') {
                s
            } else {
                String::from_utf8_lossy(bytes).into_owned()
            }
        }
    }
}

/// 读取字幕文件并解码，返回 `{ name, content }`。
fn read_subtitle(path: &std::path::Path) -> Result<Option<serde_json::Value>, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("读取字幕失败: {}", e))?;
    let content = decode_text(&bytes);
    if content.trim().is_empty() {
        return Ok(None);
    }
    Ok(Some(serde_json::json!({
        "name": path.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default(),
        "content": content,
    })))
}

/// 查找音轨对应的字幕文件（.lrc / .vtt / .srt）并读取内容。
///
/// 查找顺序：
/// 1. 音轨同目录下同 basename 的 .lrc / .vtt / .srt
/// 2. 向上定位到作品根目录（目录名含 RJ 号），递归搜索其中的 .lrc / .vtt / .srt，
///    优先同名文件，其次文件名包含音轨 basename 的。
#[tauri::command]
pub fn find_subtitle_for_track(track_path: String) -> Result<Option<serde_json::Value>, String> {
    let tp = std::path::PathBuf::from(&track_path);
    let dir = tp.parent().map(|d| d.to_path_buf()).unwrap_or_else(|| tp.clone());
    let stem = tp
        .file_stem()
        .map(|s| s.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    // 1) 同目录同 basename（大小写不敏感）
    for ext in ["lrc", "vtt", "srt"] {
        for cand_name in [format!("{}.{}", stem, ext), format!("{}.{}", stem.to_uppercase(), ext)] {
            let cand = dir.join(&cand_name);
            if cand.is_file() {
                if let Some(f) = read_subtitle(&cand)? {
                    return Ok(Some(f));
                }
            }
        }
    }

    // 2) 向上找到作品根目录，递归搜索
    let mut root: Option<std::path::PathBuf> = None;
    let mut cur = dir.clone();
    for _ in 0..8 {
        let name = cur
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        if crate::commands::import::extract_rj_code(&name).is_some() {
            root = Some(cur.clone());
            break;
        }
        if !cur.pop() {
            break;
        }
    }
    let search_dir = root.unwrap_or(dir);

    let mut found: Vec<std::path::PathBuf> = Vec::new();
    collect_subtitles(&search_dir, &mut found, 0);
    // 优先同 basename，其次包含 basename，最后按路径排序
    found.sort_by_key(|p| {
        let p_stem = p
            .file_stem()
            .map(|s| s.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        let score = if p_stem == stem {
            0
        } else if p_stem.contains(&stem) {
            1
        } else {
            2
        };
        (score, p.to_string_lossy().to_lowercase())
    });
    for p in found {
        if let Some(f) = read_subtitle(&p)? {
            return Ok(Some(f));
        }
    }
    Ok(None)
}

/// 拉取在线字幕文件文本内容并自动解码。
#[tauri::command]
pub fn fetch_subtitle_content(db: Db, url: String) -> Result<String, String> {
    let url_trimmed = url.trim();
    if url_trimmed.is_empty() {
        return Ok(String::new());
    }

    // 1) 本地路径
    if !url_trimmed.starts_with("http://") && !url_trimmed.starts_with("https://") && !url_trimmed.starts_with("asmr://") {
        let clean_path = url_trimmed.strip_prefix("file://").unwrap_or(url_trimmed);
        let p = std::path::PathBuf::from(clean_path);
        if p.is_file() {
            let bytes = std::fs::read(&p).map_err(|e| format!("读取本地字幕失败: {e}"))?;
            return Ok(decode_text(&bytes));
        }
    }

    // 2) 在线 URL / asmr 协议
    let token = {
        let conn = db.db.lock().map_err(|e| e.to_string())?;
        queries::get_setting(&conn, "asmr_one_token")
            .ok()
            .flatten()
            .unwrap_or_default()
    };

    let target_url = if let Some(hash) = url_trimmed.strip_prefix("http://asmr.localhost/file/") {
        format!("https://api.asmr-100.com/api/file/{}", hash)
    } else if let Some(hash) = url_trimmed.strip_prefix("asmr://file/") {
        format!("https://api.asmr-100.com/api/file/{}", hash)
    } else if let Some(hash) = url_trimmed.strip_prefix("asmr://") {
        format!("https://api.asmr-100.com/api/file/{}", hash)
    } else {
        url_trimmed.to_string()
    };

    let client = reqwest::blocking::Client::builder()
        .user_agent(crate::api::asmrone::USER_AGENT)
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("构建请求客户端失败: {e}"))?;

    let mut req = client.get(&target_url);
    if target_url.contains("asmr-100.com") || target_url.contains("asmr-200.com") || target_url.contains("asmr.one") {
        if !token.trim().is_empty() {
            req = req.header("Authorization", format!("Bearer {}", token.trim()));
        }
    }

    let resp = req.send().map_err(|e| format!("拉取字幕失败: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("拉取字幕 HTTP 状态异常: {}", resp.status()));
    }

    let bytes = resp.bytes().map_err(|e| format!("读取字幕数据流失败: {e}"))?;
    Ok(decode_text(&bytes))
}


#[tauri::command]
pub fn get_settings(db: Db) -> Result<serde_json::Value, String> {
    let conn = db.db.lock().map_err(|e| e.to_string())?;
    let download_dir = queries::get_setting(&conn, "download_dir")
        .ok()
        .flatten();
    let theme = queries::get_setting(&conn, "theme")
        .ok()
        .flatten()
        .unwrap_or_else(|| "dark".to_string());
    let asmr_one_token = queries::get_setting(&conn, "asmr_one_token").ok().flatten();
    Ok(serde_json::json!({
        "download_dir": download_dir,
        "theme": theme,
        "asmr_one_token": asmr_one_token,
    }))
}

/// 保存 asmr.one 登录 token。
#[tauri::command]
pub fn set_asmr_token(db: Db, token: String) -> Result<(), String> {
    let conn = db.db.lock().map_err(|e| e.to_string())?;
    queries::set_setting(&conn, "asmr_one_token", &token).map_err(|e| e.to_string())
}

/// 清除 asmr.one 登录 token。
#[tauri::command]
pub fn clear_asmr_token(db: Db) -> Result<(), String> {
    let conn = db.db.lock().map_err(|e| e.to_string())?;
    queries::set_setting(&conn, "asmr_one_token", "").map_err(|e| e.to_string())
}

/// 在指定窗口里执行 JS（用于 asmr.one 登录窗口注入按钮、抓取 token）。
#[tauri::command]
pub fn eval_in_window(app: tauri::AppHandle, label: String, js: String) -> Result<(), String> {
    let win = app
        .get_webview_window(&label)
        .ok_or_else(|| format!("窗口不存在: {}", label))?;
    win.eval(&js).map_err(|e| e.to_string())
}

/// 获取指定窗口的当前 URL（用于从 asmr.one 登录窗口读取 Token 标记）。
#[tauri::command]
pub fn get_window_url(app: tauri::AppHandle, label: String) -> Result<Option<String>, String> {
    let win = app.get_webview_window(&label);
    match win {
        Some(w) => Ok(w.url().ok().map(|u| u.to_string())),
        None => Ok(None),
    }
}

// ==================== asmr.one 登录窗口（内嵌 Webview）====================

const ASMR_LOGIN_LABEL: &str = "asmr-login";
const ASMR_LOGIN_URL: &str = "https://www.asmr.one/works";

/// 注入到 asmr.one 登录窗口的脚本，作为 `initialization_script` 在**每次页面导航**
/// （document-start）时由 WebView2 原生执行，比主窗口事后 `eval` 注入可靠得多。
///
/// 行为：
/// 1. 自动扫描 localStorage 中的 JWT（带 token/jwt/auth 关键词或 JWT 三段式），
///    找到后自动用 `history.replaceState` 写进 URL 片段 `#ASMRTOKEN=<token>`，
///    无需点击按钮；页面跳转/刷新后脚本重跑，1s 内重写。
/// 2. 同时注入一个固定的悬浮「获取 Token」按钮作为手动兜底（点它也会写 URL）。
const ASMR_LOGIN_INIT_SCRIPT: &str = r#"(function(){
  var MARK = '#ASMRTOKEN=';
  function findToken(){
    try{
      for(var i=0;i<localStorage.length;i++){
        var k=localStorage.key(i);
        var v=localStorage.getItem(k);
        if(!v || v.length < 30) continue;
        if(v.split('.').length === 3 && /^eyJ/.test(v)) return v;
        if(/token|jwt|auth/i.test(k) && v.length > 30) return v;
      }
    }catch(e){}
    return '';
  }
  function writeToken(t){
    if(!t) return;
    if(location.href.indexOf(MARK) >= 0) return;
    try{ history.replaceState(null, '', location.pathname + location.search + MARK + encodeURIComponent(t)); }catch(e){}
  }
  function ensureBtn(){
    if(!document.body) return;
    if(document.getElementById('asmr-token-btn')) return;
    var b = document.createElement('button');
    b.id = 'asmr-token-btn';
    b.textContent = '✅ 已登录，获取 Token';
    b.style.cssText = 'position:fixed;top:12px;right:12px;z-index:2147483647;background:#597ef7;color:#fff;border:none;border-radius:8px;padding:10px 16px;font-size:14px;cursor:pointer;box-shadow:0 2px 10px rgba(0,0,0,.3);font-family:sans-serif';
    b.onclick = function(){
      var t = findToken();
      if(t){ writeToken(t); b.textContent = '✅ Token 已提取，稍后窗口将自动关闭'; }
      else { alert('未找到 Token，请确认已登录后重试'); }
    };
    document.body.appendChild(b);
  }
  writeToken(findToken());
  setInterval(function(){
    ensureBtn();
    writeToken(findToken());
  }, 1000);
  document.addEventListener('DOMContentLoaded', ensureBtn);
})();"#;

/// 打开 asmr.one 登录窗口。从 Rust 侧创建并带上初始化脚本（注入更可靠）。
/// 已存在则聚焦到该窗口。
///
/// 注意：先用本地 `about:blank` 创建窗口，创建完成后再**异步**导航到目标页。
/// 直接以远程 URL 创建会让 WebView2 在创建阶段就去加载页面，网络卡住时
/// `build()` 阻塞主线程 → 整个应用冻结、窗口关不掉。
#[tauri::command]
pub fn open_asmr_login_window(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(w) = app.get_webview_window(ASMR_LOGIN_LABEL) {
        let _ = w.show();
        let _ = w.set_focus();
        return Ok(());
    }
    let blank: tauri::Url = "about:blank"
        .parse()
        .map_err(|e: url::ParseError| e.to_string())?;
    let win = tauri::WebviewWindowBuilder::new(
        &app,
        ASMR_LOGIN_LABEL,
        tauri::WebviewUrl::External(blank),
    )
    .title("asmr.one 登录")
    .inner_size(480.0, 720.0)
    .center()
    .initialization_script(ASMR_LOGIN_INIT_SCRIPT)
    .build()
    .map_err(|e| e.to_string())?;
    // 异步导航到目标页，不阻塞主线程
    let target: tauri::Url = ASMR_LOGIN_URL
        .parse()
        .map_err(|e: url::ParseError| e.to_string())?;
    win.navigate(target).map_err(|e| e.to_string())?;
    Ok(())
}

/// 关闭 asmr.one 登录窗口。
#[tauri::command]
pub fn close_asmr_login_window(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(w) = app.get_webview_window(ASMR_LOGIN_LABEL) {
        w.close().map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// 在指定窗口执行 JS 并返回其求值结果（JSON 字符串化的返回值，去掉外层引号）。
/// 页面加载中/超时返回 None。相比 `eval_in_window` 能拿到返回值，用于读取
/// 登录窗口内的 `location.href`（页面侧 URL 一定反映 replaceState）或直接扫 localStorage。
#[tauri::command]
pub fn eval_in_window_result(
    app: tauri::AppHandle,
    label: String,
    js: String,
) -> Result<Option<String>, String> {
    let win = app
        .get_webview_window(&label)
        .ok_or_else(|| format!("窗口不存在: {}", label))?;
    let (tx, rx) = std::sync::mpsc::channel::<String>();
    win.eval_with_callback(js, move |res| {
        let _ = tx.send(res);
    })
    .map_err(|e| e.to_string())?;
    match rx.recv_timeout(std::time::Duration::from_secs(3)) {
        Ok(res) => {
            // ExecuteScript 返回的是 JSON 字符串化的 JS 值，字符串值带引号
            let val: Option<String> = serde_json::from_str(res.trim()).ok();
            Ok(val.filter(|s| !s.trim().is_empty()))
        }
        Err(_) => Ok(None),
    }
}

#[tauri::command]
pub fn set_settings(db: Db, theme: Option<String>, download_dir: Option<String>) -> Result<(), String> {
    let conn = db.db.lock().map_err(|e| e.to_string())?;
    if let Some(t) = theme {
        queries::set_setting(&conn, "theme", &t).map_err(|e| e.to_string())?;
    }
    if let Some(d) = download_dir {
        queries::set_setting(&conn, "download_dir", &d).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn get_db_path(app: tauri::AppHandle) -> Result<String, String> {
    let path = crate::db::get_db_path(&app).map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().to_string())
}

/// 导出（备份）数据库到用户指定的目标路径。
/// 使用 SQLite VACUUM INTO 保证导出时数据一致性（WAL 完整合并）。
#[tauri::command]
pub fn export_database(db: Db<'_>, dest: String) -> Result<String, String> {
    let dest_path = std::path::PathBuf::from(&dest);
    // 确保目标目录存在
    if let Some(parent) = dest_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("创建目录失败: {e}"))?;
    }
    let conn = db.db.lock().map_err(|e| e.to_string())?;
    // VACUUM INTO 会将数据库完整写入新文件（合并 WAL），不影响当前连接
    conn.execute_batch(&format!(
        "VACUUM INTO '{}';",
        dest_path.to_string_lossy().replace('\'', "''")
    ))
    .map_err(|e| format!("导出数据库失败: {e}"))?;
    // 返回导出文件的大小信息
    let size = std::fs::metadata(&dest_path)
        .map(|m| m.len())
        .unwrap_or(0);
    Ok(format!(
        "导出成功：{} ({:.1} MB)",
        dest_path.display(),
        size as f64 / 1_048_576.0
    ))
}

/// 从用户指定的 .db 文件导入（还原）数据库。
/// 会关闭当前连接，用新文件覆盖当前数据库，然后重新打开并执行迁移。
#[tauri::command]
pub fn import_database(db: Db<'_>, app: tauri::AppHandle, source: String) -> Result<String, String> {
    let src = std::path::PathBuf::from(&source);
    if !src.exists() {
        return Err(format!("源文件不存在: {}", src.display()));
    }
    // 验证源文件是否是合法的 SQLite 数据库
    {
        let test_conn = rusqlite::Connection::open(&src).map_err(|e| format!("无法打开源数据库: {e}"))?;
        // 检查是否包含 works 表（最基本的完整性验证）
        let has_works: bool = test_conn
            .prepare("SELECT 1 FROM sqlite_master WHERE type='table' AND name='works'")
            .and_then(|mut stmt| stmt.exists([]))
            .map_err(|e| format!("数据库格式异常: {e}"))?;
        if !has_works {
            return Err("该文件不是有效的 DLsite ASMR Manager 数据库（缺少 works 表）".to_string());
        }
    }

    let current_db_path = crate::db::get_db_path(&app).map_err(|e| e.to_string())?;

    // 先获取锁并关闭当前连接（通过 checkpoint + close）
    let mut conn_guard = db.db.lock().map_err(|e| e.to_string())?;

    // WAL checkpoint 确保当前数据落盘
    let _ = conn_guard.pragma_update(None, "wal_checkpoint", "TRUNCATE");

    // 备份当前数据库（以防万一）
    let backup_path = current_db_path.with_extension("db.bak");
    let _ = std::fs::copy(&current_db_path, &backup_path);

    // 关闭当前连接再覆盖文件
    // 用一个空内存数据库替换，释放文件锁
    let placeholder = rusqlite::Connection::open_in_memory().map_err(|e| e.to_string())?;
    *conn_guard = placeholder;

    // 复制源文件覆盖当前数据库
    std::fs::copy(&src, &current_db_path)
        .map_err(|e| format!("覆盖数据库失败: {e}"))?;
    // 清理可能残留的 WAL/SHM 文件
    let _ = std::fs::remove_file(current_db_path.with_extension("db-wal"));
    let _ = std::fs::remove_file(current_db_path.with_extension("db-shm"));

    // 重新打开新的数据库并运行迁移
    let new_conn = crate::db::init_database(&app).map_err(|e| format!("重新初始化数据库失败: {e}"))?;
    *conn_guard = new_conn;

    // 统计导入的作品数量
    let count: i64 = conn_guard
        .query_row("SELECT COUNT(*) FROM works", [], |r| r.get(0))
        .unwrap_or(0);

    Ok(format!(
        "导入成功！已加载 {} 部作品。旧数据已备份到 {}",
        count,
        backup_path.display()
    ))
}

#[tauri::command]
pub fn open_folder(path: String) -> Result<(), String> {
    let p = std::path::PathBuf::from(&path);
    let target = if p.is_file() {
        p.parent().map(|d| d.to_path_buf()).unwrap_or(p)
    } else {
        p
    };
    if !target.exists() {
        return Err(format!("路径不存在: {}", target.display()));
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(&target)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = std::process::Command::new("xdg-open")
            .arg(&target)
            .spawn();
    }
    Ok(())
}

/// 用系统默认播放器打开音频文件。
#[tauri::command]
pub fn open_audio_file(path: String) -> Result<(), String> {
    let p = std::path::PathBuf::from(&path);
    if !p.exists() {
        return Err(format!("文件不存在: {}", p.display()));
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/C", "start", "", &path])
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = std::process::Command::new("xdg-open")
            .arg(&p)
            .spawn();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_find_subtitle_same_dir() {
        let dir = std::env::temp_dir().join("asmr_sub_test_rj999999");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let audio = dir.join("01_耳かき.mp3");
        std::fs::File::create(&audio).unwrap();
        let lrc_path = dir.join("01_耳かき.lrc");
        let mut lrc = std::fs::File::create(&lrc_path).unwrap();
        lrc.write_all("[00:01.00]こんにちは\n".as_bytes()).unwrap();

        let res = find_subtitle_for_track(audio.to_string_lossy().to_string()).unwrap();
        let v = res.expect("should find subtitle");
        assert_eq!(v["name"], "01_耳かき.lrc");
        assert!(v["content"].as_str().unwrap().contains("こんにちは"));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_find_subtitle_recursive() {
        // 字幕在作品根目录下的 scenario 子目录，音轨在 audio 子目录
        let root = std::env::temp_dir().join("asmr_sub_test_rj888888");
        let _ = std::fs::remove_dir_all(&root);
        let audio_dir = root.join("audio");
        let sub_dir = root.join("scenario");
        std::fs::create_dir_all(&audio_dir).unwrap();
        std::fs::create_dir_all(&sub_dir).unwrap();
        let audio = audio_dir.join("track01.mp3");
        std::fs::File::create(&audio).unwrap();
        let mut lrc = std::fs::File::create(sub_dir.join("track01.lrc")).unwrap();
        lrc.write_all("[00:01.00]シフトJISテキスト\n".as_bytes()).unwrap();

        let res = find_subtitle_for_track(audio.to_string_lossy().to_string()).unwrap();
        let v = res.expect("should find subtitle recursively");
        assert_eq!(v["name"], "track01.lrc");
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn test_find_subtitle_srt() {
        let dir = std::env::temp_dir().join("asmr_sub_test_rj777777");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let audio = dir.join("01_dialogue.wav");
        std::fs::File::create(&audio).unwrap();
        let srt_path = dir.join("01_dialogue.srt");
        let mut srt = std::fs::File::create(&srt_path).unwrap();
        srt.write_all("1\n00:00:01,000 --> 00:00:04,000\nテスト字幕\n".as_bytes()).unwrap();

        let res = find_subtitle_for_track(audio.to_string_lossy().to_string()).unwrap();
        let v = res.expect("should find srt subtitle");
        assert_eq!(v["name"], "01_dialogue.srt");
        assert!(v["content"].as_str().unwrap().contains("テスト字幕"));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_find_subtitle_none() {
        let dir = std::env::temp_dir().join("asmr_sub_test_no_rj");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let audio = dir.join("01.mp3");
        std::fs::File::create(&audio).unwrap();
        let res = find_subtitle_for_track(audio.to_string_lossy().to_string()).unwrap();
        assert!(res.is_none());
        std::fs::remove_dir_all(&dir).ok();
    }
}
