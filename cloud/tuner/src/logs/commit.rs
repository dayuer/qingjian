//! 日志里的一次上屏（只取纠错闭环用得到的字段）。

use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Commit {
    pub device: String,

    /// 查询时整段作用域的原始键。
    pub scope: String,

    /// 这次上屏消耗掉的键（`scope` 的前缀）。
    pub keys: String,

    pub text: String,

    /// `word` / `sentence` / `cloud` / `raw` …
    pub source: String,

    /// 双拼方案；全拼为空。
    pub scheme: String,

    pub english: bool,
}

impl Commit {
    pub fn from_entry(device: &str, entry: &Value) -> Option<Self> {
        if entry.get("event")?.as_str()? != "commit" {
            return None;
        }
        let field = |name: &str| {
            entry
                .get(name)
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned()
        };
        let keys = field("keys");
        let scope = match field("scope") {
            scope if scope.is_empty() => keys.clone(),
            scope => scope,
        };
        Some(Self {
            device: device.to_owned(),
            scope,
            keys,
            text: field("text"),
            source: field("source"),
            scheme: field("scheme"),
            english: entry
                .get("english")
                .and_then(Value::as_bool)
                .unwrap_or(false),
        })
    }
}
