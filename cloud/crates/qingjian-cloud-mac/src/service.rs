//! 同步本体：主线程上 0.5 秒一拍，看本机剪贴板有没有变、取收到的事件、刷新菜单行。
//! 网络都在 `ClipboardSync` / `DataSync` 的后台线程里，主线程从不等网络。
//! 跑在输入法进程里：入口都包 `catch_unwind`，这里出错只停同步，不能把输入法带崩（跨 ObjC 边界的 panic 会直接终止进程）。

use std::cell::{Cell, RefCell};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::time::{Duration, Instant};

use objc2::rc::Retained;
use objc2::{MainThreadMarker, sel};
use objc2_foundation::NSTimer;
use qingjian_cloud_client::{ClipboardSync, DataSync, DataSyncConfig, SyncConfig};
use qingjian_cloud_proto::EventKind;

use crate::config::AgentConfig;
use crate::history::History;
use crate::menu::{
    Display, Line, TAG_OPEN_CONFIG, TAG_PAUSE, TAG_RELOAD, TAG_SYNC_NOW, TAG_USE_LLM, build_lines,
    status_line,
};
use crate::timer::TimerTarget;
use crate::watcher::ClipboardWatcher;
use crate::{ime_config, input_source, pasteboard, paths};

/// 主循环间隔（秒）。
const TICK: f64 = 0.5;

/// 每隔这么多拍看一次当前输入法（2 秒）。
const INPUT_SOURCE_EVERY: u32 = 4;

/// 连续这么久不是青简才暂停同步：密码框里系统会临时切到英文键盘，不能一进密码框就停。
const OTHER_INPUT_GRACE: Duration = Duration::from_secs(30);

/// 刚上传过或刚收到的同一段文字，这么久之内不再上传：和苹果通用剪贴板之间的最后一道防回灌。
const ECHO_WINDOW: Duration = Duration::from_secs(10 * 60);

/// 状态行没变也隔这么久重算一次菜单行：「N 分钟前同步」要跟着走。
const MENU_REFRESH: Duration = Duration::from_secs(30);

/// 别的设备复制的条目，在这么久以内收到才自动写进本机剪贴板（毫秒）。
/// 更早的（离线很久后补拉到的）只进菜单里的历史，免得突然覆盖用户正在用的剪贴板。
const AUTO_PASTE_WINDOW_MS: i64 = 120_000;

/// 主线程这边出错后，第一次重启前等多久，之后每次翻倍。
const FIRST_RESTART: Duration = Duration::from_secs(10);

/// 重启最长等多久。
const MAX_RESTART: Duration = Duration::from_secs(600);

thread_local! {
    static SERVICE: RefCell<Option<Service>> = const { RefCell::new(None) };

    /// 定时器单独放：服务出错被丢掉后它还在走，到点重建服务。
    static TIMER: RefCell<Option<Retained<NSTimer>>> = const { RefCell::new(None) };

    /// 服务出错停了：什么时候重建、这次等了多久（下次翻倍）。
    static RESTART: Cell<Option<(Instant, Duration)>> = const { Cell::new(None) };
}

/// 输入法启动时调一次：挂上定时器、读配置、起同步。重复调用不做事。
pub fn start(mtm: MainThreadMarker) {
    if TIMER.with(|timer| timer.borrow().is_some()) {
        return;
    }
    let target = TimerTarget::new(mtm);
    // NSTimer 持有 target，定时器活着它就活着
    let timer = unsafe {
        NSTimer::scheduledTimerWithTimeInterval_target_selector_userInfo_repeats(
            TICK,
            &target,
            sel!(tick:),
            None,
            true,
        )
    };
    TIMER.with(|cell| *cell.borrow_mut() = Some(timer));
    build(FIRST_RESTART);
}

/// 「青简 Cloud ›」子菜单的内容；没启动或出错停了时为空（输入法据此收起子菜单）。
pub fn menu_lines() -> Vec<Line> {
    with(|service| service.lines.clone()).unwrap_or_default()
}

/// 菜单行每变一次加一，输入法据此判断要不要重画子菜单。
pub fn menu_revision() -> u64 {
    with(|service| service.revision).unwrap_or(0)
}

/// 子菜单里点了一项。
pub fn perform(tag: isize) {
    with(|service| service.perform(tag));
}

pub(crate) fn tick() {
    if let Some((at, waited)) = RESTART.with(Cell::get) {
        if Instant::now() >= at {
            tracing::info!("青简 Cloud 重启");
            build((waited * 2).min(MAX_RESTART));
        }
        return;
    }
    with(Service::tick);
}

/// 建服务；出错就按 `next_wait` 排下一次重建。
fn build(next_wait: Duration) {
    match catch_unwind(AssertUnwindSafe(|| {
        let mut service = Service::new();
        service.refresh_menu(true);
        service
    })) {
        Ok(service) => {
            SERVICE.with(|cell| *cell.borrow_mut() = Some(service));
            RESTART.with(|cell| cell.set(None));
            tracing::info!("青简 Cloud 已启动");
        }
        Err(_) => schedule_restart(next_wait),
    }
}

fn schedule_restart(wait: Duration) {
    tracing::error!(
        wait_secs = wait.as_secs(),
        "青简 Cloud 出错，已停止同步，稍后自动重启（见上面的日志）"
    );
    RESTART.with(|cell| cell.set(Some((Instant::now() + wait, wait))));
}

/// 在主线程上取服务；重入时（正在处理上一个回调）跳过。出错就丢掉服务、排定重启，输入法照常打字。
fn with<T>(f: impl FnOnce(&mut Service) -> T) -> Option<T> {
    let (result, crashed) = SERVICE.with(|cell| {
        let Ok(mut guard) = cell.try_borrow_mut() else {
            return (None, false);
        };
        let Some(service) = guard.as_mut() else {
            return (None, false);
        };
        match catch_unwind(AssertUnwindSafe(|| f(service))) {
            Ok(value) => (Some(value), false),
            Err(_) => {
                // 丢掉服务时它的同步线程收到停止信号，各自退出
                *guard = None;
                (None, true)
            }
        }
    });
    if crashed {
        schedule_restart(FIRST_RESTART);
    }
    result
}

struct Service {
    /// 剪贴板同步；配置好了且没暂停才有。
    sync: Option<ClipboardSync>,

    /// 学习数据、设置与输入日志的同步；配置里关掉、没配置或暂停时为 `None`。
    data: Option<DataSync>,

    /// 服务器地址，「使用 Cloud 的大模型」时写进输入法配置。
    server: Option<String>,

    /// 没配置好的原因，菜单里显示。
    unconfigured: Option<String>,

    /// 用户在菜单里点了「暂停同步」。
    paused: bool,

    /// 当前输入法不是青简已超过 [`OTHER_INPUT_GRACE`]，同步线程都停了。
    suspended: bool,

    watcher: ClipboardWatcher,

    history: History,

    /// 给输入法画子菜单的行。
    lines: Vec<Line>,

    /// [`Self::lines`] 的版本号，变一次加一。
    revision: u64,

    /// 上次重算菜单行的时间。
    menu_built: Instant,

    /// 拍数，按 [`INPUT_SOURCE_EVERY`] 看当前输入法。
    ticks: u32,

    /// 最近一次上传或收到的文字的哈希与时间，防回灌用。
    last_synced: Option<(u64, Instant)>,

    /// 从什么时候起当前输入法不是青简。
    other_input_since: Option<Instant>,
}

impl Service {
    fn new() -> Self {
        let mut service = Self {
            sync: None,
            data: None,
            server: None,
            unconfigured: None,
            paused: false,
            suspended: false,
            watcher: ClipboardWatcher::new(),
            history: History::default(),
            lines: Vec::new(),
            revision: 0,
            menu_built: Instant::now(),
            ticks: 0,
            last_synced: None,
            other_input_since: None,
        };
        service.load_config();
        service
    }

    fn load_config(&mut self) {
        // 先停旧的，再按新配置起；进度文件按服务器地址区分，换服务器会从头同步
        self.sync = None;
        self.data = None;
        self.server = None;
        self.history = History::default();
        let (Some(config_path), Some(state_dir)) = (paths::config_path(), paths::support_dir())
        else {
            self.unconfigured = Some("找不到用户目录".to_owned());
            return;
        };
        let config = match AgentConfig::load(&config_path) {
            Ok(config) => config,
            Err(reason) => {
                tracing::info!(%reason, "青简 Cloud 未启用同步");
                self.unconfigured = Some(reason);
                return;
            }
        };
        self.server = Some(config.server.clone());
        if let Some(ime_dir) = paths::ime_dir() {
            // 本机令牌放进输入法的 .env（不同步），config.toml 里只引用变量名
            match ime_config::ensure_token(&ime_dir.join(".env"), &config.token) {
                Ok(true) => tracing::info!("设备令牌已写入输入法的 .env"),
                Ok(false) => {}
                Err(error) => tracing::warn!(%error, "设备令牌写入 .env 失败"),
            }
        }
        if (config.learning || config.settings || config.logs)
            && let Some(ime_dir) = paths::ime_dir()
        {
            match DataSync::start(DataSyncConfig {
                server: config.server.clone(),
                token: config.token.clone(),
                ime_dir,
                state_dir: state_dir.join("data"),
                sync_learning: config.learning,
                sync_logs: config.logs,
                log_download_dir: config.download_logs.then(|| state_dir.join("input-log")),
                sync_config: config.settings,
            }) {
                Ok(data) => self.data = Some(data),
                Err(error) => tracing::warn!(%error, "学习数据同步启动失败"),
            }
        }
        match ClipboardSync::start(SyncConfig {
            server: config.server,
            token: config.token,
            state_dir,
        }) {
            Ok(sync) => {
                self.sync = Some(sync);
                self.unconfigured = None;
            }
            Err(error) => {
                tracing::warn!(%error, "剪贴板同步启动失败");
                self.unconfigured = Some(format!("同步启动失败：{error}"));
            }
        }
    }

    /// 每拍：看当前输入法决定暂停 / 恢复，上传本机新复制的、写入别的设备刚复制的、刷新菜单行。
    fn tick(&mut self) {
        self.ticks = self.ticks.wrapping_add(1);
        if self.ticks.is_multiple_of(INPUT_SOURCE_EVERY) {
            self.follow_input_source();
        }
        if self.suspended {
            return;
        }
        let copied = self.watcher.poll();
        let Some(sync) = &self.sync else {
            self.refresh_menu(false);
            return;
        };
        if let Some(text) = copied
            && !self.paused
        {
            let hash = text_hash(&text);
            if self.recently_synced(hash) {
                tracing::debug!("与刚同步过的内容相同，不再上传");
            } else {
                self.last_synced = Some((hash, Instant::now()));
                sync.copy(text);
            }
        }
        let mut changed = false;
        let now = now_ms();
        while let Some(incoming) = sync.try_recv() {
            changed |= self.history.apply(&incoming);
            if let EventKind::ClipAdded { text, .. } = &incoming.event.kind
                && !incoming.mine
                && !self.paused
                && (now - incoming.event.at).abs() < AUTO_PASTE_WINDOW_MS
            {
                self.last_synced = Some((text_hash(text), Instant::now()));
                // 通用剪贴板正管着剪贴板（同一 Apple ID 的设备刚复制过）：交给它，不写也不读（读会跨设备取数据卡主线程）
                if pasteboard::is_remote() {
                    tracing::debug!(seq = incoming.event.seq, "剪贴板是通用剪贴板送来的，不写");
                    continue;
                }
                // 已经一样就不写，写了会再被通用剪贴板广播回去
                if pasteboard::current_text().as_deref() == Some(text.as_str()) {
                    tracing::debug!(seq = incoming.event.seq, "剪贴板里已是这段文字，不写");
                    continue;
                }
                let count = pasteboard::write_text(text);
                self.watcher.note_own_write(count);
                tracing::info!(seq = incoming.event.seq, device = %incoming.event.device, "写入剪贴板");
            }
        }
        self.refresh_menu(changed);
    }

    /// 只在用青简时同步：切走超过 [`OTHER_INPUT_GRACE`] 就停掉同步线程，切回来按配置重新起。
    fn follow_input_source(&mut self) {
        match input_source::qingjian_selected() {
            Some(false) => {
                let since = *self.other_input_since.get_or_insert_with(Instant::now);
                if !self.suspended && since.elapsed() >= OTHER_INPUT_GRACE {
                    tracing::info!("切到了别的输入法，青简 Cloud 暂停同步");
                    self.suspended = true;
                    self.sync = None;
                    self.data = None;
                }
            }
            Some(true) => {
                self.other_input_since = None;
                if self.suspended {
                    tracing::info!("切回青简，青简 Cloud 恢复同步");
                    self.suspended = false;
                    // 暂停期间的复制不补传：从当前剪贴板重新开始看
                    self.watcher = ClipboardWatcher::new();
                    self.load_config();
                    self.refresh_menu(true);
                }
            }
            None => {}
        }
    }

    /// 子菜单的动作。
    fn perform(&mut self, tag: isize) {
        match tag {
            TAG_PAUSE => {
                self.paused = !self.paused;
                self.refresh_menu(true);
            }
            TAG_SYNC_NOW => {
                if let Some(data) = &self.data {
                    data.sync_now();
                }
            }
            TAG_USE_LLM => {
                if let (Some(server), Some(ime_dir)) = (&self.server, paths::ime_dir()) {
                    match ime_config::use_cloud_llm(&ime_dir.join("config.toml"), server) {
                        Ok(()) => tracing::info!("输入法的云联想已指向 Cloud"),
                        Err(error) => tracing::warn!(%error, "改输入法配置失败"),
                    }
                }
            }
            TAG_RELOAD => {
                self.load_config();
                self.refresh_menu(true);
            }
            TAG_OPEN_CONFIG => {
                if let Some(path) = paths::config_path() {
                    if !path.exists() {
                        let _ = AgentConfig::load(&path);
                    }
                    open(&["-t", &path.to_string_lossy()]);
                }
            }
            index if index >= 0 => {
                if let Some(entry) = self.history.get(index as usize) {
                    let count = pasteboard::write_text(&entry.text);
                    self.watcher.note_own_write(count);
                }
            }
            _ => {}
        }
    }

    fn recently_synced(&self, hash: u64) -> bool {
        self.last_synced
            .is_some_and(|(last, at)| last == hash && at.elapsed() < ECHO_WINDOW)
    }

    fn display(&self) -> Display {
        if let Some(reason) = &self.unconfigured {
            return Display::Unconfigured(reason.clone());
        }
        if self.paused {
            return Display::Paused;
        }
        match &self.sync {
            Some(sync) => Display::Sync {
                status: sync.status(),
                pending: sync.pending(),
            },
            None => Display::Unconfigured("未配置".to_owned()),
        }
    }

    /// 强制、状态行变了、或隔了 [`MENU_REFRESH`] 才重算；算出来和上次一样就不动版本号。
    fn refresh_menu(&mut self, force: bool) {
        let display = self.display();
        let status_changed = self.lines.first() != Some(&Line::Text(status_line(&display)));
        if !force && !status_changed && self.menu_built.elapsed() < MENU_REFRESH {
            return;
        }
        let data = self.data.as_ref().map(DataSync::status);
        let lines = build_lines(&display, data.as_ref(), &self.history);
        self.menu_built = Instant::now();
        if lines != self.lines {
            self.lines = lines;
            self.revision += 1;
        }
    }
}

fn text_hash(text: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    text.hash(&mut hasher);
    hasher.finish()
}

fn open(args: &[&str]) {
    if let Err(error) = std::process::Command::new("open").args(args).spawn() {
        tracing::warn!(%error, ?args, "open 失败");
    }
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or_default()
}
