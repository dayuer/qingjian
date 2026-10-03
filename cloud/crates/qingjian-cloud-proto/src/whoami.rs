//! `GET /v1/whoami` 的响应。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Whoami {
    /// 令牌对应的设备名。
    pub device: String,

    /// 服务端当前最新的 `seq`。
    pub latest: u64,
}
