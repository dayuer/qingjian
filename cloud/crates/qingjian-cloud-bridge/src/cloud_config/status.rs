//! 设置页显示的青简 Cloud 状态：连的哪台服务器、配没配好、各开关；令牌不出桥。

use serde::Serialize;

use super::{CloudConfig, CloudSwitches};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CloudStatus {
    /// 服务器地址，只读显示。
    pub server: String,

    /// 地址与令牌都有，键盘会连。
    pub connected: bool,

    #[serde(flatten)]
    pub switches: CloudSwitches,
}

impl CloudStatus {
    pub fn of(config: &CloudConfig) -> Self {
        Self {
            server: config.server.trim().to_owned(),
            connected: !config.server.trim().is_empty() && !config.token.trim().is_empty(),
            switches: CloudSwitches::of(config),
        }
    }
}
