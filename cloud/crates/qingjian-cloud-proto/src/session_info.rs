//! 账号下的一台已登录设备（会话）。

use serde::{Deserialize, Serialize};

use crate::Platform;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionInfo {
    pub id: i64,

    pub name: String,

    pub platform: Platform,

    /// Unix 毫秒。
    pub created_at: i64,

    /// 最近一次请求的时间，Unix 毫秒；登录后还没请求过为空。
    pub last_seen: Option<i64>,

    /// 是不是发这个请求的设备。
    pub current: bool,
}
