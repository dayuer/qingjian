//! `POST /v1/auth/handoff` 的请求体：Mac 拿网页登录回跳的一次性码与本进程里的 verifier 换会话。

use serde::{Deserialize, Serialize};

use crate::Device;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HandoffExchange {
    /// 回跳地址 `sujian://auth?handoff=…` 里的一次性码。
    pub handoff: String,

    /// PKCE 的 verifier，服务端核对 S256(verifier) 等于登录时的 challenge。
    pub verifier: String,

    pub device: Device,
}
