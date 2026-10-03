//! `POST /v1/clipboard` 的请求体。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PushClip {
    /// 客户端生成的唯一 id（UUID）；同一设备重发同一个 id 只记一次。
    pub client_id: String,

    pub text: String,
}
