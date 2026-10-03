//! 候选栏里给用户的那条「别的设备刚复制的」文字。

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClipOffer {
    /// 服务器上这条 `ClipAdded` 的序号，用户插入或关掉后记下，不再给。
    pub seq: u64,

    /// 复制它的设备名（`macbook`）。
    pub device: String,

    pub text: String,

    /// 服务器收到它的时间，Unix 毫秒；提示放久了就不再给。
    pub at: i64,
}
