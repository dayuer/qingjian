//! `POST /v1/auth/apple` 的请求体。

use serde::{Deserialize, Serialize};

use crate::{AppleClient, Device};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppleSignIn {
    /// Apple 签的 JWT。
    pub identity_token: String,

    /// 换 refresh_token 用的一次性授权码。
    pub authorization_code: String,

    /// 原始 nonce；交给 Apple 的是它的 SHA-256 十六进制。
    pub nonce: String,

    pub client: AppleClient,

    pub device: Device,

    /// 只有网页登录带：S256(verifier)，有它时服务端返回 HandoffGrant 而不是 SessionGrant。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub challenge: Option<String>,
}
