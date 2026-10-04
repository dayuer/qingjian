//! 学习数据、配置文件与输入日志的后台同步线程：每 30 秒一轮；收件箱等输入法合并时 2 秒看一次；
//! 连不上等失败按退避重试（1 秒起翻倍，封顶 5 分钟）；收到 401 按最长间隔等，收到 403 就停掉那一项、不再请求。
//! 登录换了令牌必须重建 [`DataSync`]，线程里的令牌不会更新。
//! 壳只要 [`DataSync::start`]，再定时读 [`DataSync::status`] 显示在菜单里。

mod data_status;
mod data_sync_config;
mod jobs;
mod reset;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use crate::config_sync::{ConfigOutcome, ConfigSync};

use crate::supervise::{Exit, supervise};
use crate::{Client, ClientError, InputLogSync, LearningSync};
use jobs::Jobs;

pub use data_status::DataStatus;
pub use data_sync_config::DataSyncConfig;
pub use reset::reset_sync_progress;

/// 正常间隔。
const INTERVAL: Duration = Duration::from_secs(30);

/// 收件箱还没被输入法合并时的检查间隔。
const WAITING_INTERVAL: Duration = Duration::from_secs(2);

/// 出错后的最长重试间隔。
const MAX_RETRY: Duration = Duration::from_secs(300);

struct Shared {
    status: Mutex<DataStatus>,

    wake: (Mutex<bool>, Condvar),

    stop: AtomicBool,
}

pub struct DataSync {
    shared: Arc<Shared>,
}

impl DataSync {
    /// 起后台线程就返回：读基线与进度文件也在线程里做，不占调用方（输入法主线程）的时间。
    /// 线程出错（panic 或读不了状态文件）按退避重启，每次都从磁盘重新读状态，不沿用出错时内存里的半截数据。
    pub fn start(config: DataSyncConfig) -> Result<Self, ClientError> {
        let shared = Arc::new(Shared {
            status: Mutex::new(DataStatus::default()),
            wake: (Mutex::new(false), Condvar::new()),
            stop: AtomicBool::new(false),
        });
        let thread_shared = shared.clone();
        std::thread::Builder::new()
            .name("cloud-data".to_owned())
            .spawn(move || {
                supervise(
                    "cloud-data",
                    || thread_shared.stop.load(Ordering::Relaxed),
                    || match open_jobs(&config) {
                        Ok(jobs) => {
                            run(&thread_shared, jobs);
                            Exit::Stopped
                        }
                        Err(error) => {
                            lock(&thread_shared.status).error = Some(error.to_string());
                            Exit::Retry(error.to_string())
                        }
                    },
                )
            })?;
        Ok(Self { shared })
    }

    pub fn status(&self) -> DataStatus {
        lock(&self.shared.status).clone()
    }

    /// 马上同步一轮（菜单里的「立即同步」）。
    pub fn sync_now(&self) {
        let (flag, condvar) = &self.shared.wake;
        *lock(flag) = true;
        condvar.notify_all();
    }
}

impl Drop for DataSync {
    fn drop(&mut self) {
        self.shared.stop.store(true, Ordering::Relaxed);
        self.sync_now();
    }
}

/// 按配置打开各项同步：读基线、进度与配置文件。
fn open_jobs(config: &DataSyncConfig) -> Result<Jobs, ClientError> {
    let client = Client::new(&config.server, &config.token);
    let learning = LearningSync::open(client.clone(), &config.ime_dir, &config.state_dir)?;
    let settings = ConfigSync::open(client.clone(), &config.ime_dir, &config.state_dir)?;
    let logs = if config.sync_logs {
        Some(InputLogSync::open(
            client,
            &config.ime_dir,
            &config.state_dir,
            config.log_download_dir.clone(),
        )?)
    } else {
        None
    };
    Ok(Jobs {
        learning: config.sync_learning.then_some(learning),
        settings: config.sync_config.then_some(settings),
        logs,
        disabled: Vec::new(),
    })
}

fn run(shared: &Shared, mut jobs: Jobs) {
    let mut retry = Duration::from_secs(1);
    while !shared.stop.load(Ordering::Relaxed) {
        let result = jobs.cycle();
        let delay = {
            let mut status = lock(&shared.status);
            status.disabled.clone_from(&jobs.disabled);
            match result {
                Ok((outcome, config)) => {
                    if outcome.pushed > 0
                        || outcome.delivered > 0
                        || config != ConfigOutcome::Unchanged
                    {
                        tracing::info!(?outcome, ?config, "学习数据同步");
                    }
                    status.last_ok_ms = Some(now_ms());
                    status.waiting_for_ime = outcome.waiting;
                    status.error = None;
                    status.unauthorized = false;
                    status.config_conflict |= config == ConfigOutcome::Conflict;
                    retry = Duration::from_secs(1);
                    if outcome.waiting {
                        WAITING_INTERVAL
                    } else {
                        INTERVAL
                    }
                }
                Err(error) => {
                    tracing::warn!(%error, "学习数据同步失败");
                    status.unauthorized = matches!(error, ClientError::Unauthorized);
                    status.error = Some(error.to_string());
                    let (delay, next) = after_error(&error, retry);
                    retry = next;
                    delay
                }
            }
        };
        wait(shared, delay);
    }
}

/// 出错后等多久、下一次退避多久。401 要用户重新登录，退避没有意义，按最长间隔等；其余翻倍退避。
fn after_error(error: &ClientError, retry: Duration) -> (Duration, Duration) {
    match error {
        ClientError::Unauthorized => (MAX_RETRY, retry),
        _ => (retry, (retry * 2).min(MAX_RETRY)),
    }
}

fn wait(shared: &Shared, timeout: Duration) {
    let (flag, condvar) = &shared.wake;
    let mut pending = lock(flag);
    if !*pending {
        pending = condvar
            .wait_timeout(pending, timeout)
            .map(|(guard, _)| guard)
            .unwrap_or_else(|poisoned| poisoned.into_inner().0);
    }
    *pending = false;
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{MAX_RETRY, after_error};
    use crate::ClientError;

    #[test]
    fn unauthorized_waits_the_longest_without_growing_backoff() {
        let retry = Duration::from_secs(4);
        assert_eq!(
            after_error(&ClientError::Unauthorized, retry),
            (MAX_RETRY, retry)
        );
    }

    #[test]
    fn other_errors_double_the_backoff_up_to_the_cap() {
        let error = ClientError::Unreachable("x".to_owned());
        assert_eq!(
            after_error(&error, Duration::from_secs(1)),
            (Duration::from_secs(1), Duration::from_secs(2))
        );
        assert_eq!(
            after_error(&error, Duration::from_secs(200)),
            (Duration::from_secs(200), MAX_RETRY)
        );
    }
}
