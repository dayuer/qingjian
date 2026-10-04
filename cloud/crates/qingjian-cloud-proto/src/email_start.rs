//! `POST /v1/auth/email/start` 的请求体：给这个邮箱发 6 位验证码，成功是 204。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmailStart {
    pub email: String,
}
