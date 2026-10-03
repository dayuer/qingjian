//! 失败重试的指数退避：1 秒起翻倍，封顶 5 分钟，成功一次归零。

use std::time::Duration;

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
