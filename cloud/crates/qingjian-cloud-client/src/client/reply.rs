//! 账号类请求的响应检查：关掉 ureq 的「状态码即错误」后自己判，读出服务端 `{"error": "…", "code": "…"}` 里的文案与代号，
//! 再按调用场景映射成 [`ClientError`]（登录类与账号类对 401 / 503 的含义不同）。

use ureq::Body;
use ureq::http::Response;

use crate::ClientError;

/// 错误响应体最多读这么多。
const MAX_ERROR_BODY: u64 = 4096;

/// 同一邮箱当天验证码输错太多次后，登录类 429 带的 `code`。
const CODE_LOCKED_TODAY: &str = "locked_today";

/// 请求属于哪一类，决定状态码的含义。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Context {
    /// 登录类（不带令牌）：401 是登录没通过，503 是服务器没配这种登录方式。
    Login,

    /// 带令牌的账号类：401 是令牌无效，403 是功能没开。
    Account,
}

/// 2xx 原样返回；其余读出错误文案后映射成错误。网络层的失败一律是 `Unreachable`。
pub fn check(
    result: Result<Response<Body>, ureq::Error>,
    context: Context,
) -> Result<Response<Body>, ClientError> {
    let mut response = result?;
    let status = response.status();
    if status.is_success() {
        return Ok(response);
    }
    let (message, code) = response
        .body_mut()
        .with_config()
        .limit(MAX_ERROR_BODY)
        .read_to_string()
        .ok()
        .map(|body| error_body(&body))
        .unwrap_or_default();
    Err(map_status(
        status.as_u16(),
        message,
        code.as_deref(),
        context,
    ))
}

/// 从 `{"error": "…", "code": "…"}` 取文案与可选的机器可读代号，解析不出来文案是空串、代号是 `None`。
fn error_body(body: &str) -> (String, Option<String>) {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(body) else {
        return (String::new(), None);
    };
    let field = |name: &str| value.get(name)?.as_str().map(str::to_owned);
    (field("error").unwrap_or_default(), field("code"))
}

fn map_status(status: u16, message: String, code: Option<&str>, context: Context) -> ClientError {
    match (context, status) {
        (Context::Login, 401) => ClientError::AuthFailed(message),
        (Context::Login, 503) => ClientError::NotConfigured(message),
        (Context::Account, 401) => ClientError::Unauthorized,
        (Context::Account, 403) => ClientError::Forbidden(message),
        (Context::Login, 429) if code == Some(CODE_LOCKED_TODAY) => {
            ClientError::LockedToday(message)
        }
        (_, 429) => ClientError::RateLimited,
        (_, 400..=499) => ClientError::Rejected { status, message },
        _ => ClientError::Unreachable(format!("http {status}: {message}")),
    }
}

#[cfg(test)]
mod tests {
    use super::{Context, error_body, map_status};
    use crate::ClientError;

    #[test]
    fn login_and_account_differ_on_401_and_503() {
        assert!(matches!(
            map_status(401, String::new(), None, Context::Login),
            ClientError::AuthFailed(_)
        ));
        assert!(matches!(
            map_status(401, String::new(), None, Context::Account),
            ClientError::Unauthorized
        ));
        assert!(matches!(
            map_status(503, String::new(), None, Context::Login),
            ClientError::NotConfigured(_)
        ));
        assert!(matches!(
            map_status(503, String::new(), None, Context::Account),
            ClientError::Unreachable(_)
        ));
        assert!(matches!(
            map_status(502, String::new(), None, Context::Login),
            ClientError::Unreachable(_)
        ));
    }

    #[test]
    fn error_body_reads_message_and_code() {
        assert_eq!(
            error_body(r#"{"error":"too many failed attempts today","code":"locked_today"}"#),
            (
                "too many failed attempts today".to_owned(),
                Some("locked_today".to_owned())
            )
        );
        assert_eq!(error_body(r#"{"error":"slow"}"#), ("slow".to_owned(), None));
        assert_eq!(error_body("<html>"), (String::new(), None));
        assert_eq!(error_body(r#"{"code":3}"#), (String::new(), None));
    }

    #[test]
    fn login_429_locked_today_only_with_that_code() {
        assert!(matches!(
            map_status(429, "m".into(), Some("locked_today"), Context::Login),
            ClientError::LockedToday(m) if m == "m"
        ));
        assert!(matches!(
            map_status(429, "m".into(), Some("other"), Context::Login),
            ClientError::RateLimited
        ));
        assert!(matches!(
            map_status(429, "m".into(), None, Context::Login),
            ClientError::RateLimited
        ));
        assert!(matches!(
            map_status(429, "m".into(), Some("locked_today"), Context::Account),
            ClientError::RateLimited
        ));
    }
}
