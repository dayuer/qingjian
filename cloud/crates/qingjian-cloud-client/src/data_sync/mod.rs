//! 学习数据与配置文件的后台同步线程：每 30 秒一轮；收件箱等输入法合并时 2 秒看一次；失败按退避重试。
//! 壳只要 [`DataSync::start`]，再定时读 [`DataSync::status`] 显示在菜单里。

mod data_status;
mod data_sync_config;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use crate::config_sync::{ConfigOutcome, ConfigSync};
use crate::{Client, ClientError, LearningOutcome, LearningSync};

pub use data_status::DataStatus;
pub use data_sync_config::DataSyncConfig;

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
    pub fn start(config: DataSyncConfig) -> Result<Self, ClientError> {
        let client = Client::new(&config.server, &config.token);
        let learning = LearningSync::open(client.clone(), &config.ime_dir, &config.state_dir)?;
        let settings = ConfigSync::open(client, &config.ime_dir, &config.state_dir)?;
        let shared = Arc::new(Shared {
            status: Mutex::new(DataStatus::default()),
            wake: (Mutex::new(false), Condvar::new()),
            stop: AtomicBool::new(false),
        });
        let thread_shared = shared.clone();
        std::thread::Builder::new()
            .name("cloud-data".to_owned())
            .spawn(move || {
                run(
                    &thread_shared,
                    learning,
                    settings,
                    config.sync_learning,
                    config.sync_config,
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

fn run(
    shared: &Shared,
    mut learning: LearningSync,
    mut settings: ConfigSync,
    sync_learning: bool,
    sync_config: bool,
) {
    let mut retry = Duration::from_secs(1);
    while !shared.stop.load(Ordering::Relaxed) {
        let learned = if sync_learning {
            learning.cycle()
        } else {
            Ok(LearningOutcome::default())
        };
        let result = learned.and_then(|outcome| {
            let config = if sync_config {
                settings.cycle()?
            } else {
                ConfigOutcome::Unchanged
            };
            Ok((outcome, config))
        });
        let delay = {
            let mut status = lock(&shared.status);
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
                    status.error = Some(error.to_string());
                    let delay = retry;
                    retry = (retry * 2).min(MAX_RETRY);
                    delay
                }
            }
        };
        wait(shared, delay);
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
