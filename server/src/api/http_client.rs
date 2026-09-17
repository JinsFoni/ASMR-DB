//! blocking reqwest Client 的进程内复用。
//!
//! `reqwest::blocking::Client` 本身是连接池设计，但此前每次外网请求都重新构建，
//! 等于每次丢弃整个连接池、重付 TLS 握手（约 100–300ms）。这里按「站点 + 代理配置」
//! 持有 Client 实例，代理设置变化时 key 变化，自然隔离旧连接。

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use crate::api::proxy::ProxyConfig;

fn client_pool() -> &'static Mutex<HashMap<String, reqwest::blocking::Client>> {
    static P: OnceLock<Mutex<HashMap<String, reqwest::blocking::Client>>> = OnceLock::new();
    P.get_or_init(|| Mutex::new(HashMap::new()))
}

/// 取指定站点的共享 blocking Client；缓存未命中才调用 `build` 构建。
/// `site` 是调用方模块标识——不同模块即使目标站点相同，默认头/重定向策略也可能不同，
/// 必须各自占一个 key。
pub fn shared_blocking_client(
    site: &'static str,
    proxy: &ProxyConfig,
    build: impl FnOnce() -> Result<reqwest::blocking::Client, String>,
) -> Result<reqwest::blocking::Client, String> {
    let key = format!(
        "{site}|{}|{}|{}|{}",
        proxy.url.as_deref().unwrap_or(""),
        proxy.dlsite,
        proxy.asmrone,
        proxy.llm
    );
    if let Some(c) = client_pool().lock().unwrap().get(&key) {
        // Client 内部是 Arc，clone 零成本
        return Ok(c.clone());
    }
    let client = build()?;
    client_pool()
        .lock()
        .unwrap()
        .insert(key, client.clone());
    Ok(client)
}
