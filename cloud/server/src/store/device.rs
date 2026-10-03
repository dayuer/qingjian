//! 一台已登记的设备。

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Device {
    pub id: i64,

    pub name: String,

    /// 登记时间，Unix 毫秒。
    pub created_at: i64,

    /// 最近一次带令牌请求的时间，Unix 毫秒；从没用过为 `None`。
    pub last_seen: Option<i64>,
}
