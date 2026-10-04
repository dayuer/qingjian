//! 事件的内容。按 `type` 字段区分，以后的数据流（输入历史、用户词）在这里加新变体。
//!
//! `PairRequest` / `DeviceJoined` 是临时事件：`seq` 恒为 0、不落库、不补发，离线设备靠 `GET /v1/pair/requests` 补看。

use serde::{Deserialize, Serialize};

use crate::Platform;

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

    /// 有新设备拿匹配码申请加入，等这台设备允许（临时事件）。
    PairRequest {
        /// 允许 / 拒绝时接在 `/v1/pair/requests/` 后面。
        request_id: String,

        /// 新设备报的设备名。
        name: String,

        platform: Platform,
    },

    /// 有设备通过找回方式加入了空间（临时事件）。
    DeviceJoined {
        name: String,

        /// 怎么加入的：`apple` / `email` / `wechat` / `web`。
        via: String,
    },
}
