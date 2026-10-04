//! 登录请求里带的设备：服务端建会话时记下，设备列表里显示、可注销。

use serde::{Deserialize, Serialize};

use crate::Platform;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Device {
    /// 设备名（iPhone 的名字、Mac 的电脑名）。
    pub name: String,

    pub platform: Platform,
}
