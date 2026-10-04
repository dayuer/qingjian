//! `POST /v1/auth/email/start` 的请求体：给这个邮箱发 6 位验证码，成功是 204。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmailStart {
    pub email: String,

    /// 同意把账号与数据发到境外服务器的文本版本号，缺省空串；服务端缺字段、空串、未知版本都返回 400 `consent_required`。
    #[serde(default)]
    pub cross_border_consent: String,
}
