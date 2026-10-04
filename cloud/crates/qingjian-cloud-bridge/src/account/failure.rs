//! 操作失败的样子：`{"code": "…", "message": "…"}`，code 给 Swift 区分种类，message 是给用户看的中文。
//! `ClientError` 到 code 与文案的映射是纯函数，不联网就能测。

use qingjian_cloud_client::ClientError;
use serde::Serialize;

/// 当天验证码输错太多次：email/start 与 email/verify 都被锁到明天。
pub const LOCKED_TODAY: &str = "今天验证失败次数过多，请明天再试，或改用 Apple 登录";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Failure {
    /// `auth_failed` `locked_today` `unauthorized` `not_configured` `rate_limited` `forbidden`
    /// `unreachable` `invalid_argument` `not_signed_in` `other`。
    pub code: &'static str,

    pub message: String,
}

impl Failure {
    pub fn new(code: &'static str, message: String) -> Self {
        Self { code, message }
    }

    /// 服务器返回的错误：code 按种类，文案由调用方按场景选。
    pub fn from_client(error: &ClientError, message: String) -> Self {
        Self::new(code_of(error), message)
    }

    pub fn invalid_argument() -> Self {
        Self::new("invalid_argument", "参数无效".to_owned())
    }

    pub fn not_signed_in() -> Self {
        Self::new("not_signed_in", "还没有登录".to_owned())
    }

    pub fn other(message: &str) -> Self {
        Self::new("other", message.to_owned())
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self)
            .unwrap_or_else(|_| r#"{"code":"other","message":""}"#.to_owned())
    }
}

pub fn code_of(error: &ClientError) -> &'static str {
    match error {
        ClientError::AuthFailed(_) => "auth_failed",
        ClientError::LockedToday(_) => "locked_today",
        ClientError::Unauthorized => "unauthorized",
        ClientError::NotConfigured(_) => "not_configured",
        ClientError::RateLimited => "rate_limited",
        ClientError::Forbidden(_) => "forbidden",
        ClientError::Unreachable(_) | ClientError::Io(_) => "unreachable",
        ClientError::Rejected { .. } | ClientError::BadResponse(_) => "other",
    }
}

pub fn apple_message(error: &ClientError) -> String {
    match error {
        ClientError::AuthFailed(_) => "Apple 登录没有通过验证，请重试".to_owned(),
        ClientError::NotConfigured(_) => "服务器还没配好 Apple 登录".to_owned(),
        other => message(other),
    }
}

pub fn email_start_message(error: &ClientError) -> String {
    match error {
        ClientError::Rejected {
            status: 400 | 422, ..
        } => "邮箱地址不对，检查后再试".to_owned(),
        ClientError::NotConfigured(_) => "服务器还没配好邮件发送".to_owned(),
        other => message(other),
    }
}

/// 验证码输错不显示剩余次数。
pub fn email_verify_message(error: &ClientError) -> String {
    match error {
        ClientError::AuthFailed(_) => "验证码不对，请重新输入".to_owned(),
        ClientError::NotConfigured(_) => "服务器还没配好邮件发送".to_owned(),
        other => message(other),
    }
}

/// 给用户看的失败原因。403 的服务端原文是英文，只记日志不显示。
pub fn message(error: &ClientError) -> String {
    match error {
        ClientError::Unreachable(_) | ClientError::Io(_) => {
            "连不上服务器，检查网络后再试".to_owned()
        }
        ClientError::Unauthorized => "登录已失效，请重新登录".to_owned(),
        ClientError::AuthFailed(_) => "验证没有通过，请重试".to_owned(),
        ClientError::NotConfigured(_) => "服务器暂时不支持这种登录方式".to_owned(),
        ClientError::Forbidden(reason) => {
            tracing::info!(%reason, "服务器拒绝了这项功能");
            "这项功能还没打开".to_owned()
        }
        ClientError::LockedToday(_) => LOCKED_TODAY.to_owned(),
        ClientError::RateLimited => "操作太频繁，请稍后再试".to_owned(),
        ClientError::Rejected { status, .. } => format!("服务器拒绝了请求（{status}）"),
        ClientError::BadResponse(_) => "服务器的回应看不懂，请升级 App".to_owned(),
    }
}
