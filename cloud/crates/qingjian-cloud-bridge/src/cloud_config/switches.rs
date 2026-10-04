//! 设置页能改的那部分 `cloud.toml`：各项功能开关。服务器地址与令牌随安装配好，设置页不给改。

use serde::{Deserialize, Serialize};

use super::CloudConfig;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CloudSwitches {
    pub llm: bool,

    pub logs: bool,

    pub sync: bool,

    pub clipboard: bool,
}

impl CloudSwitches {
    pub fn of(config: &CloudConfig) -> Self {
        Self {
            llm: config.llm,
            logs: config.logs,
            sync: config.sync,
            clipboard: config.clipboard,
        }
    }

    /// 只改开关，服务器地址与令牌保持原样。
    pub fn apply_to(self, config: &mut CloudConfig) {
        config.llm = self.llm;
        config.logs = self.logs;
        config.sync = self.sync;
        config.clipboard = self.clipboard;
    }
}
