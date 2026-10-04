//! 账号类请求的响应检查：关掉 ureq 的「状态码即错误」后自己判，读出服务端 `{"error": "…"}` 里的文案，
//! 再按调用场景映射成 [`ClientError`]（登录类与账号类对 401 / 503 的含义不同）。

use ureq::Body;
use ureq::http::Response;

use crate::ClientError;

/// 错误响应体最多读这么多。
const MAX_ERROR_BODY: u64 = 4096;

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
    let message = response
        .body_mut()
        .with_config()
        .limit(MAX_ERROR_BODY)
        .read_to_string()
        .ok()
        .map(|body| error_message(&body))
        .unwrap_or_default();
    Err(map_status(status.as_u16(), message, context))
}

/// 从 `{"error": "…"}` 取文案，解析不出来就是空串。
fn error_message(body: &str) -> String {
    serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|value| value.get("error")?.as_str().map(str::to_owned))
        .unwrap_or_default()
}

fn map_status(status: u16, message: String, context: Context) -> ClientError {
    match (context, status) {
        (Context::Login, 401) => ClientError::AuthFailed(message),
        (Context::Login, 503) => ClientError::NotConfigured(message),
        (Context::Account, 401) => ClientError::Unauthorized,
        (Context::Account, 403) => ClientError::Forbidden(message),
        (_, 429) => ClientError::RateLimited,
        (_, 400..=499) => ClientError::Rejected { status, message },
        _ => ClientError::Unreachable(format!("http {status}: {message}")),
    }
}

#[cfg(test)]
mod tests {
    use super::{Context, error_message, map_status};
    use crate::ClientError;

    #[test]
    fn error_message_reads_error_field_or_empty() {
        assert_eq!(error_message(r#"{"error":"nope"}"#), "nope");
        assert_eq!(error_message("<html>"), "");
        assert_eq!(error_message(r#"{"error":3}"#), "");
    }

    #[test]
    fn login_and_account_differ_on_401_and_503() {
        assert!(matches!(
            map_status(401, String::new(), Context::Login),
            ClientError::AuthFailed(_)
        ));
        assert!(matches!(
            map_status(401, String::new(), Context::Account),
            ClientError::Unauthorized
        ));
        assert!(matches!(
            map_status(503, String::new(), Context::Login),
            ClientError::NotConfigured(_)
        ));
        assert!(matches!(
            map_status(503, String::new(), Context::Account),
            ClientError::Unreachable(_)
        ));
        assert!(matches!(
            map_status(502, String::new(), Context::Login),
            ClientError::Unreachable(_)
        ));
    }
}
