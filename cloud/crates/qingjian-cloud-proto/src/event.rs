//! 一条事件：服务端分配的序号、来源设备、时间与内容。

use serde::{Deserialize, Serialize};

use crate::EventKind;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Event {
    /// 全局递增的序号，从 1 开始。
    pub seq: u64,

    /// 产生这条事件的设备名。
    pub device: String,

    /// 服务端收到的时间，Unix 毫秒。
    pub at: i64,

    #[serde(flatten)]
    pub kind: EventKind,
}
