//! `POST /v1/pair/code` 的响应：旧设备出的匹配码，给新设备输入。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PairCode {
    /// 展示格式，如 `K7P2-9QXM`；新设备输入时先过 `normalize_pair_code`。
    pub code: String,

    /// 过期时间，Unix 毫秒。
    pub expires_at: i64,
}
