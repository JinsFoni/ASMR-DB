//! LLM 翻译客户端（OpenAI 兼容 /chat/completions 接口）。

use crate::api::proxy::{ProxyConfig, ProxyTarget};
use crate::db::queries;
use rusqlite::Connection;
use serde_json::Value;

/// 系统默认翻译 Prompt（未自定义时使用）。
pub const DEFAULT_PROMPT: &str = "你是专业的 ASMR 音声作品翻译。将用户提供的日文作品标题和简介准确翻译成简体中文：\
保留人名、社团名、作品系列名等专有名词的惯用译法；语气自然流畅，符合中文表达习惯；\
不要添加解释、注释、罗马音或原文，只输出翻译后的中文文本。";

/// LLM 翻译配置（存于 settings 表）。
#[derive(Debug, Clone, Default)]
pub struct LlmConfig {
    /// OpenAI 兼容端点，如 https://api.deepseek.com/v1
    pub endpoint: String,
    pub api_key: String,
    pub model: String,
    /// 单次任务最大自动重试次数
    pub max_retry: i64,
    /// 每秒请求上限
    pub rps: i64,
    /// 并发线程数（1-4）
    pub threads: i64,
    /// 入库自动翻译
    pub auto_translate: bool,
    /// 自定义翻译 Prompt（空 = 系统默认）
    pub prompt: String,
    pub proxy: ProxyConfig,
}

impl LlmConfig {
    pub fn from_conn(conn: &Connection) -> Self {
        let get = |k: &str| -> Option<String> {
            queries::get_setting(conn, k)
                .ok()
                .flatten()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
        };
        let num = |k: &str, default: i64, min: i64, max: i64| -> i64 {
            get(k)
                .and_then(|v| v.parse::<i64>().ok())
                .map(|v| v.clamp(min, max))
                .unwrap_or(default)
        };
        Self {
            endpoint: get("llm_endpoint").unwrap_or_default(),
            api_key: get("llm_api_key").unwrap_or_default(),
            model: get("llm_model").unwrap_or_default(),
            max_retry: num("llm_max_retry", 3, 0, 10),
            rps: num("llm_rps", 2, 1, 100),
            threads: num("llm_threads", 1, 1, 4),
            auto_translate: get("llm_auto").is_some_and(|v| v == "1"),
            prompt: get("llm_prompt").unwrap_or_else(|| DEFAULT_PROMPT.to_string()),
            proxy: ProxyConfig::from_conn(conn),
        }
    }

    /// 配置是否完整（可发起翻译）。
    pub fn ready(&self) -> bool {
        !self.endpoint.is_empty() && !self.api_key.is_empty() && !self.model.is_empty()
    }

    /// chat/completions 完整 URL（端点末尾已带则直接使用）。
    pub fn chat_url(&self) -> String {
        let e = self.endpoint.trim().trim_end_matches('/');
        if e.ends_with("/chat/completions") {
            e.to_string()
        } else {
            format!("{e}/chat/completions")
        }
    }
}

/// 调用 LLM 翻译一段文本，返回翻译结果。
pub fn translate(cfg: &LlmConfig, text: &str) -> Result<String, String> {
    if !cfg.ready() {
        return Err("未配置 LLM API（请到设置页填写端点、密钥和模型）".to_string());
    }
    let builder = reqwest::blocking::Client::builder()
        .user_agent(crate::api::asmrone::USER_AGENT)
        .timeout(std::time::Duration::from_secs(60));
    let client = cfg
        .proxy
        .apply_blocking(builder, ProxyTarget::Llm)
        .build()
        .map_err(|e| format!("构建请求客户端失败: {e}"))?;

    let body = serde_json::json!({
        "model": cfg.model,
        "temperature": 0.3,
        "messages": [
            { "role": "system", "content": cfg.prompt },
            { "role": "user", "content": text }
        ]
    });

    let resp = client
        .post(cfg.chat_url())
        .header("Authorization", format!("Bearer {}", cfg.api_key))
        .json(&body)
        .send()
        .map_err(|e| format!("连接 LLM 服务失败: {e}"))?;

    let status = resp.status();
    let raw = resp.text().map_err(|e| format!("读取响应失败: {e}"))?;
    if !status.is_success() {
        let snippet: String = raw.chars().take(200).collect();
        return Err(format!("LLM HTTP {}：{snippet}", status.as_u16()));
    }
    let v: Value = serde_json::from_str(&raw).map_err(|e| format!("解析 LLM 响应失败: {e}"))?;
    let content = v
        .get("choices")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("message"))
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_str())
        .map(|s| s.trim().trim_matches(|q| q == '"' || q == '「' || q == '」').trim())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| "LLM 响应中没有翻译结果".to_string())?;
    Ok(content.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chat_url_variants() {
        let mut cfg = LlmConfig::default();
        cfg.endpoint = "https://api.deepseek.com/v1".into();
        assert_eq!(cfg.chat_url(), "https://api.deepseek.com/v1/chat/completions");
        cfg.endpoint = "https://api.deepseek.com/v1/".into();
        assert_eq!(cfg.chat_url(), "https://api.deepseek.com/v1/chat/completions");
        cfg.endpoint = "https://x.com/v1/chat/completions".into();
        assert_eq!(cfg.chat_url(), "https://x.com/v1/chat/completions");
    }

    #[test]
    fn config_defaults_and_clamps() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT);")
            .unwrap();
        let cfg = LlmConfig::from_conn(&conn);
        assert_eq!(cfg.threads, 1);
        assert_eq!(cfg.max_retry, 3);
        assert_eq!(cfg.rps, 2);
        assert!(!cfg.auto_translate);
        assert_eq!(cfg.prompt, DEFAULT_PROMPT);
        assert!(!cfg.ready());

        // 越界值收敛到 1-4
        for (v, expect) in [("0", 1), ("3", 3), ("99", 4)] {
            conn.execute(
                "INSERT INTO settings (key, value) VALUES ('llm_threads', ?1)",
                [v],
            )
            .unwrap();
            assert_eq!(LlmConfig::from_conn(&conn).threads, expect);
            conn.execute("DELETE FROM settings WHERE key='llm_threads'", []).unwrap();
        }
    }
}
