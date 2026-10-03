//! 客户端错误。按处理方式分：连不上（重试）、令牌无效（停下等用户改配置）、请求被拒（丢掉这一条）。

#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    /// 网络不通、超时、TLS 失败、服务端 5xx：过一会儿重试。
    #[error("server unreachable: {0}")]
    Unreachable(String),

    /// 令牌无效或设备已被移除。
    #[error("device token rejected")]
    Unauthorized,

    /// 服务端拒绝了这个请求（4xx），重试也没用。
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
            ureq::Error::StatusCode(status) if (400..500).contains(&status) => Self::Rejected {
                status,
                message: String::new(),
            },
            other => Self::Unreachable(other.to_string()),
        }
    }
}
