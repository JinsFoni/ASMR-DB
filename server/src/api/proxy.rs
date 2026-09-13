//! 出站请求代理配置（存于 settings 表）。
//!
//! DLsite 与 asmr.one 的抓取、文件下载可分别开关是否走代理；
//! 代理地址仅支持 HTTP(S) 代理（如 Clash/V2Ray 的混合端口）。
//! 未启用或地址无效时一律回退直连，不阻断请求。

use rusqlite::Connection;

/// 代理生效的目标站点。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ProxyTarget {
    Dlsite,
    AsmrOne,
}

/// 代理配置：地址 + 每个站点是否启用。
#[derive(Debug, Clone, Default)]
pub struct ProxyConfig {
    /// 代理地址，如 http://127.0.0.1:8502；None / 空 = 未配置
    pub url: Option<String>,
    pub dlsite: bool,
    pub asmrone: bool,
}

impl ProxyConfig {
    /// 从 settings 表读取代理配置。
    pub fn from_conn(conn: &Connection) -> Self {
        let get = |k: &str| -> Option<String> {
            crate::db::queries::get_setting(conn, k)
                .ok()
                .flatten()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
        };
        let on = |k: &str| get(k).is_some_and(|v| v == "1");
        Self {
            url: get("proxy_url"),
            dlsite: on("proxy_dlsite"),
            asmrone: on("proxy_asmrone"),
        }
    }

    fn enabled(&self, target: ProxyTarget) -> bool {
        match target {
            ProxyTarget::Dlsite => self.dlsite,
            ProxyTarget::AsmrOne => self.asmrone,
        }
    }

    /// 规范化代理地址：无 scheme 时补 http://。
    fn normalized_url(&self) -> Option<String> {
        let raw = self.url.as_deref()?.trim();
        if raw.is_empty() {
            return None;
        }
        Some(if raw.contains("://") {
            raw.to_string()
        } else {
            format!("http://{raw}")
        })
    }

    /// 给 blocking 客户端构建器应用目标站点的代理（未启用或地址无效时原样返回）。
    pub fn apply_blocking(
        &self,
        builder: reqwest::blocking::ClientBuilder,
        target: ProxyTarget,
    ) -> reqwest::blocking::ClientBuilder {
        if !self.enabled(target) {
            return builder;
        }
        match self.normalized_url().and_then(|u| reqwest::Proxy::all(&u).ok()) {
            Some(p) => builder.proxy(p),
            None => {
                eprintln!("⚠️ 代理地址无效，{} 请求走直连", target_name(target));
                builder
            }
        }
    }

    /// 给 async 客户端构建器应用目标站点的代理。
    pub fn apply_async(
        &self,
        builder: reqwest::ClientBuilder,
        target: ProxyTarget,
    ) -> reqwest::ClientBuilder {
        if !self.enabled(target) {
            return builder;
        }
        match self.normalized_url().and_then(|u| reqwest::Proxy::all(&u).ok()) {
            Some(p) => builder.proxy(p),
            None => {
                eprintln!("⚠️ 代理地址无效，{} 请求走直连", target_name(target));
                builder
            }
        }
    }
}

fn target_name(t: ProxyTarget) -> &'static str {
    match t {
        ProxyTarget::Dlsite => "DLsite",
        ProxyTarget::AsmrOne => "asmr.one",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT);")
            .unwrap();
        conn
    }

    #[test]
    fn default_is_direct() {
        let conn = test_conn();
        let cfg = ProxyConfig::from_conn(&conn);
        assert_eq!(cfg.url, None);
        assert!(!cfg.dlsite);
        assert!(!cfg.asmrone);
    }

    #[test]
    fn reads_settings() {
        let conn = test_conn();
        for (k, v) in [
            ("proxy_url", "127.0.0.1:8502"),
            ("proxy_dlsite", "1"),
            ("proxy_asmrone", "0"),
        ] {
            conn.execute(
                "INSERT INTO settings (key, value) VALUES (?1, ?2)",
                [k, v],
            )
            .unwrap();
        }
        let cfg = ProxyConfig::from_conn(&conn);
        assert_eq!(cfg.url.as_deref(), Some("127.0.0.1:8502"));
        assert!(cfg.dlsite);
        assert!(!cfg.asmrone);

        // 开关关闭的站点不注入代理
        let b = reqwest::blocking::Client::builder();
        let b = cfg.apply_blocking(b, ProxyTarget::AsmrOne);
        let _ = b.build().unwrap();
    }

    #[test]
    fn invalid_url_falls_back_to_direct() {
        let cfg = ProxyConfig {
            url: Some(":::not a url".into()),
            dlsite: true,
            asmrone: false,
        };
        let client = cfg
            .apply_blocking(reqwest::blocking::Client::builder(), ProxyTarget::Dlsite)
            .build()
            .unwrap();
        let _ = client; // 构建成功即可：无效地址回退直连
    }
}
