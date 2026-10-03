//! 合成：一段拼音分几次选完（`qingjian` 先选「青」再选「简」），合起来就是用户想要、却没有作为一个候选给出的词。
//! 判断依据是上屏的 `keys` 只是 `scope` 的前缀，而下一次上屏的 `scope` 正好是剩下的那段。

use std::collections::HashMap;

use super::{Commit, LogLine};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Composition {
    pub device: String,

    /// 整段键（去掉 `'`）。
    pub keys: String,

    /// 几次上屏的文字连起来。
    pub text: String,

    /// 分了几次。
    pub steps: usize,

    /// 前面最多 20 个字，给大模型判断语境。
    pub context: String,
}

/// 只看全拼、中文模式的合成；双拼的键不是拼音，跳过。各设备的行在日志里是交错的，按设备分别跟踪。
pub fn compositions(lines: &[LogLine]) -> Vec<Composition> {
    let mut found = Vec::new();
    let mut context: HashMap<String, String> = HashMap::new();
    let mut open: HashMap<String, (Composition, String)> = HashMap::new();
    for line in lines {
        let Some(commit) = Commit::from_entry(&line.device, &line.entry) else {
            if line.entry.get("event").and_then(|e| e.as_str()) == Some("break") {
                open.remove(&line.device);
                context.remove(&line.device);
            }
            continue;
        };
        let pending = open.remove(&commit.device);
        if commit.english || !commit.scheme.is_empty() {
            continue;
        }
        let rest = commit.scope.strip_prefix(&commit.keys).map(str::to_owned);
        let next = match pending {
            // 接着上一次没选完的那段
            Some((mut current, expected)) if expected == commit.scope => {
                current.text.push_str(&commit.text);
                current.steps += 1;
                finish(current, rest, &mut found)
            }
            _ if commit.keys.len() < commit.scope.len() => {
                let current = Composition {
                    device: commit.device.clone(),
                    keys: commit.scope.replace('\'', ""),
                    text: commit.text.clone(),
                    steps: 1,
                    context: tail(context.get(&commit.device).map_or("", String::as_str), 20),
                };
                finish(current, rest, &mut found)
            }
            _ => None,
        };
        if let Some(next) = next {
            open.insert(commit.device.clone(), next);
        }
        context
            .entry(commit.device.clone())
            .or_default()
            .push_str(&commit.text);
    }
    found
}

/// 选完了就收下（至少两步），否则继续等剩下的那段。
fn finish(
    current: Composition,
    rest: Option<String>,
    found: &mut Vec<Composition>,
) -> Option<(Composition, String)> {
    match rest.filter(|r| !r.trim_matches('\'').is_empty()) {
        Some(rest) => Some((current, rest.trim_start_matches('\'').to_owned())),
        None => {
            if current.steps >= 2 {
                found.push(current);
            }
            None
        }
    }
}

fn tail(text: &str, chars: usize) -> String {
    let skip = text.chars().count().saturating_sub(chars);
    text.chars().skip(skip).collect()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn line(entry: serde_json::Value) -> LogLine {
        LogLine {
            device: "mac".to_owned(),
            raw: entry.to_string(),
            entry,
        }
    }

    fn commit(scope: &str, keys: &str, text: &str) -> LogLine {
        line(
            json!({"event": "commit", "scope": scope, "keys": keys, "text": text, "source": "word", "scheme": "", "english": false}),
        )
    }

    #[test]
    fn joins_step_by_step_picks() {
        let lines = vec![
            commit("woyao", "woyao", "我要"),
            commit("qingjianyun", "qing", "青"),
            commit("jianyun", "jian", "简"),
            commit("yun", "yun", "云"),
            commit("shiguo", "shiguo", "试过"),
        ];
        let found = compositions(&lines);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].keys, "qingjianyun");
        assert_eq!(found[0].text, "青简云");
        assert_eq!(found[0].steps, 3);
        assert_eq!(found[0].context, "我要");
    }

    #[test]
    fn devices_interleave_without_mixing() {
        let mut phone = commit("jianyun", "jian", "简");
        phone.device = "iphone".to_owned();
        let lines = vec![
            commit("qingjian", "qing", "青"),
            phone,
            commit("jian", "jian", "简"),
        ];
        let found = compositions(&lines);
        assert_eq!(found.len(), 1);
        assert_eq!(
            (found[0].device.as_str(), found[0].text.as_str()),
            ("mac", "青简")
        );
    }

    #[test]
    fn handles_separators_and_breaks() {
        let lines = vec![
            commit("qing'jian", "qing'", "青"),
            commit("jian", "jian", "简"),
            commit("abc", "a", "啊"),
            line(json!({"event": "break"})),
            commit("bc", "bc", "不错"),
        ];
        let found = compositions(&lines);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].keys, "qingjian");
    }
}
