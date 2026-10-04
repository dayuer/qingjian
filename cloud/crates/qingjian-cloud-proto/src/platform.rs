//! 设备的平台：建会话时报给服务端，设备列表里显示。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    Ios,
    Macos,
    /// 服务端的网页登录页。
    Web,
}
