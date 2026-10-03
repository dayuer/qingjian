//! 跨次运行记住的东西：审过的用户词不再审，加过的词不再加，省大模型的钱；上一轮时的日志行数，没怎么打字就跳过。

use std::collections::BTreeSet;
use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TunerState {
    /// 体检过的用户词。
    pub audited: BTreeSet<String>,

    /// 已经提议并推送过、或被大模型否掉的词。
    pub seen: BTreeSet<String>,

    /// 上一轮跑完时服务器上的输入日志行数，用来判断这段时间打了多少字。旧的状态文件没有这一项。
    #[serde(default)]
    pub log_lines: usize,
}

impl TunerState {
    /// 距上一轮新增的日志行数。日志被清空过（变短了）就从头算。
    pub fn new_lines(&self, now: usize) -> usize {
        if now < self.log_lines {
            now
        } else {
            now - self.log_lines
        }
    }
}

impl TunerState {
    pub fn load(path: &Path) -> Self {
        std::fs::read(path)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        let temp = path.with_extension("json.tmp");
        std::fs::write(&temp, serde_json::to_vec(self).unwrap_or_default())?;
        std::fs::rename(temp, path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_new_lines_and_restarts_after_clear() {
        let state = TunerState {
            log_lines: 400,
            ..TunerState::default()
        };
        assert_eq!(state.new_lines(430), 30);
        assert_eq!(state.new_lines(400), 0);
        // 日志被清空后又写了 20 行
        assert_eq!(state.new_lines(20), 20);
    }

    #[test]
    fn old_state_file_without_line_count_still_loads() {
        let old: TunerState = serde_json::from_str(r#"{"audited":["曜行"],"seen":[]}"#).unwrap();
        assert_eq!(old.log_lines, 0);
        assert!(old.audited.contains("曜行"));
    }
}
