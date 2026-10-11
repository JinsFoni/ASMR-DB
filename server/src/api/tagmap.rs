//! 日文标签 → 简体中文标签映射表。
//!
//! 数据来源：asmr.one 标签 API `https://api.asmr.one/api/tags/{id}` 的 i18n(zh-cn) 字段
//! （枚举全部标签 ID 抓取生成，见 assets/tag-map.json，键为日文标签名，值为简中名）。
//! 榜单（DLsite）与作品库（asmr.one）的日文标签在展示/入库时经此表翻译，
//! 不在表内的标签原样保留。

use std::collections::HashMap;
use std::sync::OnceLock;

const TAG_MAP_JSON: &str = include_str!("../../assets/tag-map.json");

fn map() -> &'static HashMap<&'static str, &'static str> {
    static MAP: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();
    MAP.get_or_init(|| serde_json::from_str(TAG_MAP_JSON).expect("tag-map.json 格式错误"))
}

/// 日文标签翻译为简体中文；不在映射表内的原样返回。
pub fn zh(name: &str) -> &str {
    map().get(name).copied().unwrap_or(name)
}

/// 翻译一组标签，保留顺序。
pub fn zh_all(names: &[String]) -> Vec<String> {
    names.iter().map(|n| zh(n).to_string()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn translates_known_ja_tag() {
        assert_eq!(zh("ラブラブ/あまあま"), "亲热/甜蜜");
        assert_eq!(zh("オフィス/職場"), "办公室/职场");
    }

    #[test]
    fn passthrough_unknown_and_chinese() {
        assert_eq!(zh("自定义标签"), "自定义标签");
        assert_eq!(zh("连续高潮"), "连续高潮");
    }

    #[test]
    fn zh_all_keeps_order() {
        let v = vec!["連続絶頂".to_string(), "不存在的标签".to_string()];
        assert_eq!(zh_all(&v), vec!["连续高潮", "不存在的标签"]);
    }
}
