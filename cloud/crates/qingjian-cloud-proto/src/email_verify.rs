//! `POST /v1/auth/email/verify` 的请求体：邮箱加验证码换会话（首次验证即注册）。

use serde::{Deserialize, Serialize};

use crate::Device;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmailVerify {
    pub email: String,

    /// 邮件里的 6 位数字。
    pub code: String,

    pub device: Device,

    /// 只有网页登录带：S256(verifier)，有它时服务端返回 HandoffGrant。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub challenge: Option<String>,
}
