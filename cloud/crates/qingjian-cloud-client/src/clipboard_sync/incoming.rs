//! 交给壳的一条事件。

use qingjian_cloud_proto::Event;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Incoming {
    pub event: Event,

    /// 本机自己发的（壳用来做历史列表，不写回剪贴板）。
    pub mine: bool,
}
