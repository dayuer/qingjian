use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

mod available;
mod package;
mod state;

pub use available::Available;
pub use package::Package;
pub use state::UpdateState;

use qingjian_platform::{UpdateChannel, UpdateConfig};

use crate::download::fetch_package;
use crate::index::fetch_index;
use crate::{Target, UpdateError, Version};

/// 查成功后隔多久再查。
const CHECK_INTERVAL_SECS: u64 = 24 * 60 * 60;

/// 下载的安装包放在 `update.json` 旁边的这个目录。
const UPDATES_DIR: &str = "updates";

/// 查失败（断网、验签不过）后隔多久再试。
const RETRY_INTERVAL_SECS: u64 = 60 * 60;

/// 检查更新的调度：壳在自己的定时器里调 [`poll`](Self::poll)，到点了起一个一次性线程去查，不占调用线程。
pub struct Checker {
    state_path: PathBuf,

    /// 当前版本原文（进 User-Agent）与解析结果；解析不了或是 `-dev` 本地包就什么都不查。
    current: String,
    version: Option<Version>,

    target: Target,
    state: Arc<Mutex<UpdateState>>,
    in_flight: Arc<AtomicBool>,
    attempted_at: Arc<AtomicU64>,
}

impl Checker {
    /// `current` 是壳的版本号，经 [`Self::effective_version`] 可被环境变量顶替。
    pub fn new(state_path: PathBuf, current: &str) -> Self {
        let current = Self::effective_version(current);
        let version = Version::parse(&current).filter(|version| !version.is_dev());
        Self {
            state: Arc::new(Mutex::new(UpdateState::load(&state_path))),
            state_path,
            current,
            version,
            target: Target::current(),
            in_flight: Arc::new(AtomicBool::new(false)),
            attempted_at: Arc::new(AtomicU64::new(0)),
        }
    }

    /// 到点就查：开关开着、上次成功超过一天（或换了渠道）、上次尝试超过一小时、没有正在查的。
    pub fn poll(&self, config: &UpdateConfig) {
        if !config.check {
            return;
        }
        let now = UpdateState::now();
        let due = {
            let state = self.lock();
            state.channel != Some(config.channel)
                || now.saturating_sub(state.checked_at) >= CHECK_INTERVAL_SECS
        };
        let retry_ok =
            now.saturating_sub(self.attempted_at.load(Ordering::Relaxed)) >= RETRY_INTERVAL_SECS;
        if due && retry_ok {
            self.check_now(config);
        }
    }

    /// 立刻查一次（「检查更新」按钮），不看间隔；已有正在查的就不重复起。
    pub fn check_now(&self, config: &UpdateConfig) {
        let Some(version) = self.version.clone() else {
            return;
        };
        if self.in_flight.swap(true, Ordering::AcqRel) {
            return;
        }
        self.attempted_at
            .store(UpdateState::now(), Ordering::Relaxed);
        let channel = config.channel;
        let current = self.current.clone();
        let target = self.target;
        let state = Arc::clone(&self.state);
        let in_flight = Arc::clone(&self.in_flight);
        let state_path = self.state_path.clone();
        std::thread::spawn(move || {
            match check_once(&state_path, &current, &version, target, channel) {
                Ok(next) => {
                    *state
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner()) = next
                }
                Err(error) => tracing::warn!(%error, "检查更新失败"),
            }
            in_flight.store(false, Ordering::Release);
        });
    }

    /// 要提示的新版本：开关开着、渠道里看得到、比当前新（装完新版后旧结果自然失效）。
    pub fn available(&self, config: &UpdateConfig) -> Option<Available> {
        let version = self.version.as_ref()?;
        let state = self.lock();
        Self::relevant(&state, version, config).cloned()
    }

    /// 要提示的新版本已经下载并校验过的安装包；还没下完或索引没给下载地址时为 `None`。
    pub fn downloaded(&self, config: &UpdateConfig) -> Option<PathBuf> {
        let version = self.version.as_ref()?;
        let state = self.lock();
        let found = Self::relevant(&state, version, config)?;
        let file = &found.package.as_ref()?.file;
        state.downloaded.clone().filter(|path| {
            path.is_file() && path.file_name().and_then(|n| n.to_str()) == Some(file)
        })
    }

    /// 参与比较的版本号：`QINGJIAN_UPDATE_VERSION` 可以顶替壳的版本号（拿 `-dev` 包测提示用）。
    pub fn effective_version(current: &str) -> String {
        std::env::var("QINGJIAN_UPDATE_VERSION").unwrap_or_else(|_| current.to_owned())
    }

    /// `current` 是不是本地开发包（或版本号读不懂），这种不检查；已按 [`Self::effective_version`] 顶替。
    pub fn is_dev_version(current: &str) -> bool {
        Version::parse(&Self::effective_version(current)).is_none_or(|version| version.is_dev())
    }

    /// 本地开发包（或版本号读不懂）：什么都不查。
    pub fn is_dev_build(&self) -> bool {
        self.version.is_none()
    }

    /// 正在查。
    pub fn checking(&self) -> bool {
        self.in_flight.load(Ordering::Acquire)
    }

    /// 上次查成功的时间（Unix 秒），0 = 还没查过。
    pub fn checked_at(&self) -> u64 {
        self.lock().checked_at
    }

    /// 当场查一次并落盘，阻塞到查完（Windows 设置程序的「立即检查」在后台线程里调）；本地开发包返回 `None`。
    pub fn check_blocking(
        state_path: &Path,
        current: &str,
        config: &UpdateConfig,
    ) -> Option<Result<UpdateState, UpdateError>> {
        let current = Self::effective_version(current);
        let version = Version::parse(&current).filter(|version| !version.is_dev())?;
        Some(check_once(
            state_path,
            &current,
            &version,
            Target::current(),
            config.channel,
        ))
    }

    /// 不经调度、只看落盘结果（Windows 设置程序用）：`state` 里有没有该提示 `current` 的新版本。
    pub fn available_in(
        state: &UpdateState,
        current: &str,
        config: &UpdateConfig,
    ) -> Option<Available> {
        let version = Version::parse(&Self::effective_version(current))
            .filter(|version| !version.is_dev())?;
        Self::relevant(state, &version, config).cloned()
    }

    fn relevant<'a>(
        state: &'a UpdateState,
        version: &Version,
        config: &UpdateConfig,
    ) -> Option<&'a Available> {
        if !config.check {
            return None;
        }
        state.available.as_ref().filter(|found| {
            config.channel.includes(&found.channel)
                && Version::parse(&found.version).is_some_and(|found| &found > version)
        })
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, UpdateState> {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

/// 下载、验签、挑版本、写 `update.json`。
fn check_once(
    state_path: &Path,
    current: &str,
    version: &Version,
    target: Target,
    channel: UpdateChannel,
) -> Result<UpdateState, UpdateError> {
    let index = fetch_index(current)?;
    let available = index.newest(version, target, channel);
    if available.is_none()
        && let Some(dir) = state_path.parent().map(|dir| dir.join(UPDATES_DIR))
    {
        // 已装上（或没有新版）：上次下的安装包不再有用，别一直占着上百 MB
        let _ = std::fs::remove_dir_all(dir);
    }
    let downloaded = available
        .as_ref()
        .and_then(|found| found.package.as_ref())
        .and_then(|package| {
            let dir = state_path.parent()?.join(UPDATES_DIR);
            fetch_package(&dir, package, current)
                .inspect_err(|error| tracing::warn!(%error, "新版本安装包下载失败"))
                .ok()
        });
    let next = UpdateState {
        checked_at: UpdateState::now(),
        channel: Some(channel),
        available,
        downloaded,
    };
    tracing::info!(
        channel = channel.key(),
        available = next.available.as_ref().map(|found| found.version.as_str()),
        "检查更新完成"
    );
    if let Err(error) = next.save(state_path) {
        tracing::warn!(%error, "检查更新的结果没写进文件");
    }
    Ok(next)
}

#[cfg(test)]
mod tests {

    use super::*;

    fn state(version: &str, channel: &str) -> UpdateState {
        UpdateState {
            checked_at: 1,
            channel: Some(UpdateChannel::Beta),
            available: Some(Available {
                version: version.to_owned(),
                channel: channel.to_owned(),
                date: String::new(),
                notes: Vec::new(),
                package: None,
            }),
            downloaded: None,
        }
    }

    fn config(check: bool, channel: UpdateChannel) -> UpdateConfig {
        UpdateConfig { check, channel }
    }

    #[test]
    fn saved_result_expires_once_the_new_version_is_installed() {
        let state = state("0.1.4", "stable");
        let on = config(true, UpdateChannel::Stable);
        assert!(Checker::available_in(&state, "0.1.3", &on).is_some());
        assert!(Checker::available_in(&state, "0.1.4", &on).is_none());
    }

    #[test]
    fn saved_prerelease_is_hidden_on_the_stable_channel_and_when_off() {
        let state = state("0.1.4-beta.1", "beta");
        assert!(
            Checker::available_in(&state, "0.1.3", &config(true, UpdateChannel::Beta)).is_some()
        );
        assert!(
            Checker::available_in(&state, "0.1.3", &config(true, UpdateChannel::Stable)).is_none()
        );
        assert!(
            Checker::available_in(&state, "0.1.3", &config(false, UpdateChannel::Beta)).is_none()
        );
    }

    #[test]
    fn dev_builds_are_never_prompted() {
        let state = state("0.1.4", "stable");
        let on = config(true, UpdateChannel::Stable);
        assert!(Checker::available_in(&state, "0.1.3-dev-d893e1e", &on).is_none());
    }
}
