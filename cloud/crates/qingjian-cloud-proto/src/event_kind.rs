//! 事件的内容。按 `type` 字段区分，以后的数据流（输入历史、用户词）在这里加新变体。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EventKind {
    /// 某台设备复制了一段文本。
    ClipAdded {
        /// 客户端生成的唯一 id；离线队列重发时服务端据此去重。
        client_id: String,

        text: String,
    },

    /// 删掉一条剪贴板记录。
    ClipDeleted {
        /// 被删的那条 `ClipAdded` 的 `seq`。
        target: u64,
    },
}
