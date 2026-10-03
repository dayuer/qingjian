//! 后台线程的看护：线程体 panic 或报告要重来时，记日志、等一会儿重跑，不让同步悄悄停掉
//! （2026-10-03 真机上同步线程因 TLS 配置 panic 后静默退出，菜单停在「连接中」，日志里什么都没有）。

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::time::{Duration, Instant};

/// 第一次重跑前等多久，之后每次翻倍。
const FIRST_RETRY: Duration = Duration::from_secs(5);

/// 最长等多久。
const MAX_RETRY: Duration = Duration::from_secs(300);

/// 线程体跑了这么久才出错，就当是偶发的，退避从头算。
const HEALTHY_RUN: Duration = Duration::from_secs(600);

/// 等待时每隔这么久看一次是否该退出。
const STOP_POLL: Duration = Duration::from_millis(200);

/// 线程体跑完一轮的结果。
pub(crate) enum Exit {
    /// 收到停止信号，线程该结束了。
    Stopped,

    /// 出了不能在线程体里自己恢复的错（比如启动时读不了状态文件），等一会儿从头再来。
    Retry(String),
}

/// 反复跑 `body` 直到它返回 [`Exit::Stopped`] 或 `stopped()` 为真；panic 与 [`Exit::Retry`] 都按退避重跑。
pub(crate) fn supervise(name: &str, stopped: impl Fn() -> bool, mut body: impl FnMut() -> Exit) {
    let mut retry = FIRST_RETRY;
    while !stopped() {
        let started = Instant::now();
        let reason = match catch_unwind(AssertUnwindSafe(&mut body)) {
            Ok(Exit::Stopped) => return,
            Ok(Exit::Retry(reason)) => reason,
            Err(payload) => panic_message(payload.as_ref()),
        };
        if started.elapsed() >= HEALTHY_RUN {
            retry = FIRST_RETRY;
        }
        tracing::error!(thread = name, %reason, retry_secs = retry.as_secs(), "同步线程出错，稍后重启");
        let until = Instant::now() + retry;
        while Instant::now() < until && !stopped() {
            std::thread::sleep(STOP_POLL);
        }
        retry = (retry * 2).min(MAX_RETRY);
    }
}

fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
    payload
        .downcast_ref::<&str>()
        .map(|text| (*text).to_owned())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "panic".to_owned())
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU32, Ordering};

    use super::*;

    #[test]
    fn restarts_after_panic_and_stops_when_asked() {
        let runs = AtomicU32::new(0);
        // 第一次 panic，第二次正常结束：应当跑两次（中间等 FIRST_RETRY，这里用 stopped 不打断）
        let started = Instant::now();
        supervise(
            "test",
            || false,
            || {
                if runs.fetch_add(1, Ordering::SeqCst) == 0 {
                    panic!("boom");
                }
                Exit::Stopped
            },
        );
        assert_eq!(runs.load(Ordering::SeqCst), 2);
        assert!(started.elapsed() >= FIRST_RETRY);
    }

    #[test]
    fn stop_signal_cuts_the_wait_short() {
        let runs = AtomicU32::new(0);
        let started = Instant::now();
        supervise(
            "test",
            || runs.load(Ordering::SeqCst) > 0,
            || {
                runs.fetch_add(1, Ordering::SeqCst);
                Exit::Retry("again".to_owned())
            },
        );
        assert_eq!(runs.load(Ordering::SeqCst), 1);
        assert!(started.elapsed() < FIRST_RETRY);
    }
}
