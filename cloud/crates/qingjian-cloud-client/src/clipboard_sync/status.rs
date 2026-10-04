//! 连接状态，壳用来改菜单栏图标与提示。

use crate::ClientError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Status {
    /// 刚启动，还没连上过。
    Connecting,

    /// SSE 连着或刚刚请求成功。
    Online,

    /// 连不上，正在按退避重试；带最近一次的错误。
    Offline(String),

    /// 令牌被拒（401）：要用户重新登录。
    Unauthorized,

    /// 服务器上没开跨设备剪贴板（403）：停下，等用户在设置里打开。
    Disabled,
}

impl Status {
    /// 请求失败后该显示的状态。
    pub fn from_error(error: &ClientError) -> Self {
        match error {
            ClientError::Unauthorized => Self::Unauthorized,
            ClientError::Forbidden(_) => Self::Disabled,
            other => Self::Offline(other.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Status;
    use crate::ClientError;

    #[test]
    fn error_maps_to_status() {
        assert_eq!(
            Status::from_error(&ClientError::Unauthorized),
            Status::Unauthorized
        );
        assert_eq!(
            Status::from_error(&ClientError::Forbidden(String::new())),
            Status::Disabled
        );
        assert!(matches!(
            Status::from_error(&ClientError::RateLimited),
            Status::Offline(_)
        ));
        assert!(matches!(
            Status::from_error(&ClientError::Unreachable("x".to_owned())),
            Status::Offline(_)
        ));
    }
}
