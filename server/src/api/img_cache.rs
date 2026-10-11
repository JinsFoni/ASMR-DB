//! 磁盘图片缓存：远端封面图经 /api/img 代理时旁路落盘。
//!
//! 文件名为 URL 的 SHA-256，文件头自包含元数据（LRU 不依赖文件系统时间戳）：
//! ```text
//! [0..8)   stored_ms   u64 LE  首次存入的 unix 毫秒
//! [8..16)  access_ms   u64 LE  最近一次命中的 unix 毫秒（淘汰依据）
//! [16..18) mime_len    u16 LE  Content-Type 长度
//! [18..)   mime + 原始字节
//! ```
//! 容量上限与 TTL 来自设置表（image_cache_max_mb / image_cache_ttl_hours，
//! 0 = 关闭缓存），每次请求从 DB 读取，改动即时生效。超限时先删过期文件，
//! 仍超则按 access_ms 从旧到新淘汰。缓存目录固定为数据库同级的 image_cache/。

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use sha2::{Digest, Sha256};

const HEADER_MIN: usize = 18;
const DEFAULT_MAX_MB: u64 = 512;
const DEFAULT_TTL_HOURS: u64 = 168; // 7 天

pub fn cache_dir(db_path: &Path) -> PathBuf {
    db_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("image_cache")
}

/// 从设置表读 (max_mb, ttl)。max_mb = 0 表示关闭。
pub fn config_from(conn: &rusqlite::Connection) -> (u64, Duration) {
    let max_mb = crate::db::queries::get_setting(conn, "image_cache_max_mb")
        .ok()
        .flatten()
        .and_then(|v| v.parse::<u64>().ok())
        .filter(|v| *v <= 64 * 1024)
        .unwrap_or(DEFAULT_MAX_MB);
    let ttl_hours = crate::db::queries::get_setting(conn, "image_cache_ttl_hours")
        .ok()
        .flatten()
        .and_then(|v| v.parse::<u64>().ok())
        .filter(|v| *v <= 24 * 365)
        .unwrap_or(DEFAULT_TTL_HOURS);
    (max_mb, Duration::from_secs(ttl_hours.saturating_mul(3600)))
}

fn key_of(url: &str) -> String {
    let mut h = Sha256::new();
    h.update(url.as_bytes());
    h.finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>()
}

fn path_for(dir: &Path, url: &str) -> PathBuf {
    dir.join(format!("{}.img", key_of(url)))
}

/// 当前 unix 毫秒：LRU 与 TTL 都用它，避免同秒写入的时间戳并列。
fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// 查缓存：命中返回 (字节, Content-Type) 并刷新访问时间；过期视为未命中并删除。
pub fn get(dir: &Path, url: &str, ttl: Duration) -> Option<(Vec<u8>, String)> {
    let p = path_for(dir, url);
    let mut data = Vec::new();
    std::fs::File::open(&p).ok()?.read_to_end(&mut data).ok()?;
    if data.len() < HEADER_MIN {
        let _ = std::fs::remove_file(&p);
        return None;
    }
    let stored_ms = u64::from_le_bytes(data[0..8].try_into().ok()?);
    let mime_len = u16::from_le_bytes(data[16..18].try_into().ok()?) as usize;
    if data.len() < HEADER_MIN + mime_len {
        let _ = std::fs::remove_file(&p);
        return None;
    }
    let now = unix_now();
    let age = Duration::from_millis(now.saturating_sub(stored_ms));
    if age >= ttl {
        let _ = std::fs::remove_file(&p);
        return None;
    }
    let mime = String::from_utf8_lossy(&data[HEADER_MIN..HEADER_MIN + mime_len]).to_string();
    let mime = if mime.is_empty() { guess_mime(url) } else { mime };
    // 刷新 LRU 时间戳（失败只影响淘汰顺序）
    data[8..16].copy_from_slice(&now.to_le_bytes());
    let _ = std::fs::write(&p, &data);
    Some((data[HEADER_MIN + mime_len..].to_vec(), mime))
}

/// 写缓存；max_mb = 0 或空 body 时跳过。失败静默——缓存是尽力而为。
pub fn put(dir: &Path, url: &str, body: &[u8], mime: &str, max_mb: u64, ttl: Duration) {
    if max_mb == 0 || body.is_empty() {
        return;
    }
    if std::fs::create_dir_all(dir).is_err() {
        return;
    }
    let now = unix_now();
    let mime_bytes = mime.as_bytes();
    if mime_bytes.len() > u16::MAX as usize {
        return;
    }
    let mut buf = Vec::with_capacity(HEADER_MIN + mime_bytes.len() + body.len());
    buf.extend_from_slice(&now.to_le_bytes());
    buf.extend_from_slice(&now.to_le_bytes());
    buf.extend_from_slice(&(mime_bytes.len() as u16).to_le_bytes());
    buf.extend_from_slice(mime_bytes);
    buf.extend_from_slice(body);
    if std::fs::File::create(path_for(dir, url))
        .and_then(|mut f| f.write_all(&buf))
        .is_err()
    {
        return;
    }
    enforce_limit(dir, max_mb, ttl);
}

/// 清理：删过期文件；总体积仍超 max_mb 时按最近访问从旧到新淘汰。
pub fn enforce_limit(dir: &Path, max_mb: u64, ttl: Duration) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let max_bytes = max_mb.saturating_mul(1024 * 1024);
    let now = unix_now();
    let mut files: Vec<(PathBuf, u64, u64)> = Vec::new(); // (path, len, access_ms)
    let mut total: u64 = 0;
    for e in entries.flatten() {
        let p = e.path();
        if p.extension().and_then(|x| x.to_str()) != Some("img") {
            continue;
        }
        let Ok(meta) = e.metadata() else { continue };
        if meta.len() < HEADER_MIN as u64 {
            let _ = std::fs::remove_file(&p);
            continue;
        }
        let mut head = [0u8; 16];
        let ok = std::fs::File::open(&p)
            .and_then(|mut f| f.read_exact(&mut head))
            .is_ok();
        if !ok {
            continue;
        }
        let stored_ms = u64::from_le_bytes(head[0..8].try_into().unwrap());
        let access_ms = u64::from_le_bytes(head[8..16].try_into().unwrap());
        if Duration::from_millis(now.saturating_sub(stored_ms)) >= ttl {
            let _ = std::fs::remove_file(&p);
            continue;
        }
        total += meta.len();
        files.push((p, meta.len(), access_ms));
    }
    if total <= max_bytes {
        return;
    }
    files.sort_by(|(pa, _, a), (pb, _, b)| a.cmp(b).then_with(|| pa.cmp(pb)));
    for (p, len, _) in files {
        if total <= max_bytes {
            break;
        }
        if std::fs::remove_file(&p).is_ok() {
            total = total.saturating_sub(len);
        }
    }
}

/// 删除全部缓存文件（设置页「清空缓存」用）。
pub fn clear(dir: &Path) -> Result<u32, String> {
    let entries = std::fs::read_dir(dir).map_err(|e| format!("读取缓存目录失败: {e}"))?;
    let mut n = 0u32;
    for e in entries.flatten() {
        if e.path().extension().and_then(|x| x.to_str()) == Some("img") {
            if std::fs::remove_file(e.path()).is_ok() {
                n += 1;
            }
        }
    }
    Ok(n)
}

/// 缓存统计：(文件数, 总字节数)。
pub fn stats(dir: &Path) -> (u32, u64) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return (0, 0);
    };
    let mut count = 0u32;
    let mut total = 0u64;
    for e in entries.flatten() {
        if e.path().extension().and_then(|x| x.to_str()) == Some("img") {
            if let Ok(m) = e.metadata() {
                count += 1;
                total += m.len();
            }
        }
    }
    (count, total)
}

/// 内容嗅探不到时按 URL 扩展名兜底。
fn guess_mime(url: &str) -> String {
    let lower = url.split(['?', '#']).next().unwrap_or("").to_lowercase();
    if lower.ends_with(".png") {
        "image/png".into()
    } else if lower.ends_with(".webp") {
        "image/webp".into()
    } else if lower.ends_with(".gif") {
        "image/gif".into()
    } else {
        "image/jpeg".into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmpdir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("asmr_img_cache_test_{name}"));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn put_get_roundtrip() {
        let d = tmpdir("roundtrip");
        put(&d, "https://x/a.jpg", b"hello-image", "image/jpeg", 10, Duration::from_secs(60));
        let (bytes, mime) = get(&d, "https://x/a.jpg", Duration::from_secs(60)).unwrap();
        assert_eq!(bytes, b"hello-image");
        assert_eq!(mime, "image/jpeg");
        assert_eq!(stats(&d).0, 1);
    }

    #[test]
    fn expired_entry_is_miss() {
        let d = tmpdir("expired");
        put(&d, "https://x/b.jpg", b"data", "image/jpeg", 10, Duration::from_secs(1));
        // 手动把 stored 时间改旧：直接读改头
        let p = path_for(&d, "https://x/b.jpg");
        let mut data = std::fs::read(&p).unwrap();
        data[0..8].copy_from_slice(&1u64.to_le_bytes()); // 1970 年 → 必过期
        std::fs::write(&p, &data).unwrap();
        assert!(get(&d, "https://x/b.jpg", Duration::from_secs(60)).is_none());
    }

    #[test]
    fn zero_limit_disables() {
        let d = tmpdir("disabled");
        put(&d, "https://x/c.jpg", b"data", "image/jpeg", 0, Duration::from_secs(60));
        assert!(get(&d, "https://x/c.jpg", Duration::from_secs(60)).is_none());
        assert_eq!(stats(&d).0, 0);
    }

    #[test]
    fn enforce_limit_evicts_oldest_access() {
        let d = tmpdir("evict");
        // 每张 400KB、上限 1MB：最多同时容两张，放第三张时淘汰最旧
        put(&d, "https://x/1.jpg", &[7u8; 400_000], "image/jpeg", 1, Duration::from_secs(3600));
        put(&d, "https://x/2.jpg", &[8u8; 400_000], "image/jpeg", 1, Duration::from_secs(3600));
        // 命中 1 号，刷新其访问时间 → 2 号成为最旧
        let _ = get(&d, "https://x/1.jpg", Duration::from_secs(3600));
        put(&d, "https://x/3.jpg", &[9u8; 400_000], "image/jpeg", 1, Duration::from_secs(3600));
        assert!(get(&d, "https://x/1.jpg", Duration::from_secs(3600)).is_some());
        assert!(get(&d, "https://x/2.jpg", Duration::from_secs(3600)).is_none());
    }

    #[test]
    fn clear_removes_all() {
        let d = tmpdir("clear");
        put(&d, "https://x/1.jpg", b"a", "image/jpeg", 10, Duration::from_secs(60));
        put(&d, "https://x/2.jpg", b"b", "image/jpeg", 10, Duration::from_secs(60));
        assert_eq!(clear(&d).unwrap(), 2);
        assert_eq!(stats(&d), (0, 0));
    }
}
