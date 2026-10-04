//! 客户端错误。按处理方式分：连不上（重试）、令牌无效或功能没开（停下等用户登录或打开）、请求被拒（丢掉这一条）。

#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    /// 网络不通、超时、TLS 失败、服务端 5xx：过一会儿重试。
    #[error("server unreachable: {0}")]
    Unreachable(String),

    /// 令牌无效：退出登录了、设备被注销或账号已删（401）。
    #[error("session token rejected")]
    Unauthorized,

    /// 这项功能在服务器上没开，或访问了别人的东西（403）：停下，等用户在设置里打开，不重试。
    #[error("forbidden: {0}")]
    Forbidden(String),

    /// 请求太频繁（429，验证码发得太勤、大模型超了每日上限）：这次放弃，交给用户稍后再试。
    #[error("rate limited")]
    RateLimited,

    /// 服务端拒绝了这个请求（其余 4xx），重试也没用。
    #[error("request rejected ({status}): {message}")]
    Rejected { status: u16, message: String },

    /// 响应不是预期的 JSON。
    #[error("bad response: {0}")]
    BadResponse(String),

    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

impl ClientError {
    /// 稍后重试有没有意义。
    pub fn is_retryable(&self) -> bool {
        matches!(self, Self::Unreachable(_) | Self::Io(_))
    }
}

impl From<ureq::Error> for ClientError {
    fn from(error: ureq::Error) -> Self {
        match error {
            ureq::Error::StatusCode(401) => Self::Unauthorized,
            // ureq 开了 http_status_as_error，响应体拿不到，原因留空
            ureq::Error::StatusCode(403) => Self::Forbidden(String::new()),
            ureq::Error::StatusCode(429) => Self::RateLimited,
            ureq::Error::StatusCode(status) if (400..500).contains(&status) => Self::Rejected {
                status,
                message: String::new(),
            },
            other => Self::Unreachable(other.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ClientError;

    #[test]
    fn status_codes_map_to_variants() {
        assert!(matches!(
            ClientError::from(ureq::Error::StatusCode(401)),
            ClientError::Unauthorized
        ));
        assert!(matches!(
            ClientError::from(ureq::Error::StatusCode(403)),
            ClientError::Forbidden(message) if message.is_empty()
        ));
        assert!(matches!(
            ClientError::from(ureq::Error::StatusCode(429)),
            ClientError::RateLimited
        ));
        assert!(matches!(
            ClientError::from(ureq::Error::StatusCode(409)),
            ClientError::Rejected { status: 409, .. }
        ));
        assert!(matches!(
            ClientError::from(ureq::Error::StatusCode(503)),
            ClientError::Unreachable(_)
        ));
    }

    #[test]
    fn forbidden_and_rate_limited_are_not_retryable() {
        assert!(!ClientError::Forbidden(String::new()).is_retryable());
        assert!(!ClientError::RateLimited.is_retryable());
        assert!(!ClientError::Unauthorized.is_retryable());
        assert!(ClientError::Unreachable("x".to_owned()).is_retryable());
    }
}
