//! 带 `challenge` 的登录（网页登录页）的响应：60 秒有效的一次性码，页面拿它回跳给 Mac。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HandoffGrant {
    pub handoff: String,
}
