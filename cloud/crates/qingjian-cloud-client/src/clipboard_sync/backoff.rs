//! 失败重试的指数退避：1 秒起翻倍，封顶 5 分钟，成功一次归零；401 / 403 要用户操作，改用固定间隔。

use std::time::Duration;

use crate::ClientError;

/// 令牌被拒（401）或服务器上没开剪贴板（403）后多久再试：这两种都要用户操作，不按退避空转。
pub const USER_ACTION_RETRY: Duration = Duration::from_secs(300);

const INITIAL: Duration = Duration::from_secs(1);

const MAX: Duration = Duration::from_secs(300);

#[derive(Debug)]
pub struct Backoff {
    next: Duration,
}

impl Backoff {
    pub fn new() -> Self {
        Self { next: INITIAL }
    }

    /// 这次该等多久，并把下次翻倍。
    pub fn next_delay(&mut self) -> Duration {
        let delay = self.next;
        self.next = (self.next * 2).min(MAX);
        delay
    }

    pub fn reset(&mut self) {
        self.next = INITIAL;
    }
}

/// 失败后该等多久：401 / 403 固定间隔且不推进退避，其余按退避。
pub fn retry_delay(error: &ClientError, backoff: &mut Backoff) -> Duration {
    match error {
        ClientError::Unauthorized | ClientError::Forbidden(_) => USER_ACTION_RETRY,
        _ => backoff.next_delay(),
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{Backoff, USER_ACTION_RETRY, retry_delay};
    use crate::ClientError;

    #[test]
    fn user_action_errors_use_fixed_delay_without_advancing_backoff() {
        let mut backoff = Backoff::new();
        assert_eq!(
            retry_delay(&ClientError::Unauthorized, &mut backoff),
            USER_ACTION_RETRY
        );
        assert_eq!(
            retry_delay(&ClientError::Forbidden(String::new()), &mut backoff),
            USER_ACTION_RETRY
        );
        assert_eq!(backoff.next_delay(), Duration::from_secs(1));
    }

    #[test]
    fn other_errors_advance_backoff() {
        let mut backoff = Backoff::new();
        let error = ClientError::Unreachable("x".to_owned());
        assert_eq!(retry_delay(&error, &mut backoff), Duration::from_secs(1));
        assert_eq!(retry_delay(&error, &mut backoff), Duration::from_secs(2));
    }
}
