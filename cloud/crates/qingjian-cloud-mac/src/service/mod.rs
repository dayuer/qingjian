//! 同步本体：主线程上 0.5 秒一拍，看本机剪贴板有没有变、取收到的事件、处理账号操作的结果、刷新菜单行。
//! 网络都在 `ClipboardSync` / `DataSync` 与账号操作的后台线程里，主线程从不等网络。
//! 跑在输入法进程里：入口都包 `catch_unwind`，这里出错只停同步，不能把输入法带崩（跨 ObjC 边界的 panic 会直接终止进程）。
//! 登录、退出登录、功能开关与跟随服务器状态在 `account.rs`。

mod account;

use std::cell::{Cell, RefCell};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::{Duration, Instant};

use objc2::rc::Retained;
use objc2::{MainThreadMarker, sel};
use objc2_foundation::NSTimer;
use qingjian_cloud_client::{ClipboardSync, DataSync, DataSyncConfig, SyncConfig};
use qingjian_cloud_proto::EventKind;

use crate::account::{AccountEvent, AccountFlow};
use crate::config::AgentConfig;
use crate::history::History;
use crate::llm_endpoint::LlmEndpoint;
use crate::menu::{
    AccountMenu, Display, Line, TAG_CANCEL_JOIN, TAG_OPEN_CONFIG, TAG_PAUSE, TAG_RELOAD,
    TAG_SIGN_OUT, TAG_SYNC_NOW, build_lines, status_line, toggled_feature,
};
use crate::timer::TimerTarget;
use crate::watcher::ClipboardWatcher;
use crate::{input_source, pasteboard, paths};

/// 主循环间隔（秒）。
const TICK: f64 = 0.5;

/// 每隔这么多拍看一次当前输入法（2 秒）。
const INPUT_SOURCE_EVERY: u32 = 4;

/// 连续这么久不是素笺才暂停同步：密码框里系统会临时切到英文键盘，不能一进密码框就停。
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

/// 「素笺云 ›」子菜单的内容；没启动或出错停了时为空（输入法据此收起子菜单）。
pub fn menu_lines() -> Vec<Line> {
    with(|service| service.lines.clone()).unwrap_or_default()
}

/// 菜单行每变一次加一，输入法据此判断要不要重画子菜单。
pub fn menu_revision() -> u64 {
    with(|service| service.revision).unwrap_or(0)
}

/// 云联想选素笺云时用的大模型代理端点；没登录、没开大模型（或服务没起来）为 `None`。
pub fn llm_endpoint() -> Option<LlmEndpoint> {
    with(|service| service.endpoint.clone()).flatten()
}

/// 子菜单里点了一项（建空间 / 输码 / 清空这三项由壳先弹窗，再调下面三个函数）。
pub fn perform(tag: isize) {
    with(|service| service.perform(tag));
}

/// 「开通素笺云」：输入法壳弹过出境同意之后调，`consented` 是用户点过「同意并继续」。
pub fn create_space(consented: bool) {
    with(|service| service.create_space(consented));
}

/// 「输入匹配码加入」：输入法壳弹过输入框、拿到码之后调。
pub fn join_with_code(code: &str) {
    with(|service| service.join_with_code(code));
}

/// 「清空云端输入记录」：输入法壳弹过确认之后调。
pub fn clear_input_log() {
    with(|service| service.clear_input_log());
}

pub(crate) fn tick() {
    if let Some((at, waited)) = RESTART.with(Cell::get) {
        if Instant::now() >= at {
            tracing::info!("素笺云重启");
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
            tracing::info!("素笺云已启动");
        }
        Err(_) => schedule_restart(next_wait),
    }
}

fn schedule_restart(wait: Duration) {
    tracing::error!(
        wait_secs = wait.as_secs(),
        "素笺云出错，已停止同步，稍后自动重启（见上面的日志）"
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
    /// 剪贴板同步；登录了、开了剪贴板且没暂停才有。
    sync: Option<ClipboardSync>,

    /// 学习数据、设置与输入日志的同步；没登录、两项都没开或暂停时为 `None`。
    data: Option<DataSync>,

    /// 大模型代理的地址与令牌，输入法的云联想选素笺云时用；没登录或没开大模型为 `None`。
    endpoint: Option<LlmEndpoint>,

    /// 读到的配置；读不了时为 `None`（原因在 `unconfigured`）。
    config: Option<AgentConfig>,

    /// 配置读不了或同步起不来的原因，菜单里显示。
    unconfigured: Option<String>,

    /// 用户在菜单里点了「暂停同步」。
    paused: bool,

    /// 当前输入法不是素笺已超过 [`OTHER_INPUT_GRACE`]，同步线程都停了。
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

    /// 从什么时候起当前输入法不是素笺。
    other_input_since: Option<Instant>,

    /// 加入阶段与事件代数：晚到的旧结果靠它丢掉。
    flow: AccountFlow,

    /// 让正在等的配对轮询收手（用户取消或退出时置位）。
    join_stop: Arc<AtomicBool>,

    /// 账号操作的后台线程与登录窗口回调把（发出时的代数，结果）发到这里，主线程每拍取。
    events: Receiver<(u64, AccountEvent)>,

    sender: Sender<(u64, AccountEvent)>,

    /// 最近一次登录或切换开关失败的原因，菜单里显示。
    note: Option<String>,
}

impl Service {
    fn new() -> Self {
        let (sender, events) = mpsc::channel();
        let mut service = Self {
            sync: None,
            data: None,
            endpoint: None,
            config: None,
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
            flow: AccountFlow::default(),
            join_stop: Arc::new(AtomicBool::new(false)),
            events,
            sender,
            note: None,
        };
        service.load_config();
        service.refresh_consents();
        service
    }

    fn load_config(&mut self) {
        // 先停旧的，再按新配置起；进度文件按服务器地址区分，换服务器会从头同步
        self.sync = None;
        self.data = None;
        self.endpoint = None;
        self.history = History::default();
        self.config = None;
        let (Some(config_path), Some(state_dir)) = (paths::config_path(), paths::support_dir())
        else {
            self.unconfigured = Some("找不到用户目录".to_owned());
            return;
        };
        let config = match AgentConfig::load(&config_path) {
            Ok(config) => config,
            Err(reason) => {
                tracing::info!(%reason, "素笺云配置读不了");
                self.unconfigured = Some(reason);
                return;
            }
        };
        self.unconfigured = None;
        // 没登录或暂停中（切走了输入法）只记下配置，不起同步
        if !config.signed_in() || self.suspended {
            self.config = Some(config);
            return;
        }
        let server = config.server();
        if config.llm {
            self.endpoint = Some(LlmEndpoint::new(&server, &config.token));
        }
        if (config.sync || config.logs)
            && let Some(ime_dir) = paths::ime_dir()
        {
            match DataSync::start(DataSyncConfig {
                server: server.clone(),
                token: config.token.clone(),
                ime_dir,
                state_dir: state_dir.join("data"),
                sync_learning: config.sync,
                sync_logs: config.logs,
                log_download_dir: (config.logs && config.download_logs)
                    .then(|| state_dir.join("input-log")),
                sync_config: config.sync,
            }) {
                Ok(data) => self.data = Some(data),
                Err(error) => tracing::warn!(%error, "学习数据同步启动失败"),
            }
        }
        if config.clipboard {
            match ClipboardSync::start(SyncConfig {
                server,
                token: config.token.clone(),
                state_dir,
            }) {
                Ok(sync) => self.sync = Some(sync),
                Err(error) => {
                    tracing::warn!(%error, "剪贴板同步启动失败");
                    self.unconfigured = Some(format!("同步启动失败：{error}"));
                }
            }
        }
        self.config = Some(config);
    }

    /// 每拍：处理账号操作的结果，看当前输入法决定暂停 / 恢复，跟着服务器状态改开关，
    /// 上传本机新复制的、写入别的设备刚复制的、刷新菜单行。
    fn tick(&mut self) {
        self.ticks = self.ticks.wrapping_add(1);
        // 登录窗的回调、换令牌的结果不能等到切回素笺
        self.apply_account_events();
        if self.ticks.is_multiple_of(INPUT_SOURCE_EVERY) {
            self.follow_input_source();
        }
        if self.suspended {
            return;
        }
        self.follow_server_state();
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

    /// 只在用素笺时同步：切走超过 [`OTHER_INPUT_GRACE`] 就停掉同步线程，切回来按配置重新起。
    fn follow_input_source(&mut self) {
        match input_source::qingjian_selected() {
            Some(false) => {
                let since = *self.other_input_since.get_or_insert_with(Instant::now);
                if !self.suspended && since.elapsed() >= OTHER_INPUT_GRACE {
                    tracing::info!("切到了别的输入法，素笺云暂停同步");
                    self.suspended = true;
                    self.sync = None;
                    self.data = None;
                }
            }
            Some(true) => {
                self.other_input_since = None;
                if self.suspended {
                    tracing::info!("切回素笺，素笺云恢复同步");
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
        if let Some(feature) = toggled_feature(tag) {
            self.toggle(feature);
            return;
        }
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
            TAG_RELOAD => {
                self.load_config();
                self.refresh_consents();
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
            TAG_CANCEL_JOIN => self.cancel_join(),
            TAG_SIGN_OUT => self.sign_out(),
            // TAG_CREATE_SPACE / TAG_JOIN_WITH_CODE / TAG_CLEAR_INPUT_LOG 由输入法壳拦截：
            // 它们要先弹原生弹窗（出境同意 / 输匹配码 / 清空确认），拿到结果再调下面导出的函数
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

    fn signed_in(&self) -> bool {
        self.config.as_ref().is_some_and(AgentConfig::signed_in)
    }

    fn display(&self) -> Display {
        if let Some(reason) = &self.unconfigured {
            return Display::Unconfigured(reason.clone());
        }
        if !self.signed_in() {
            return Display::SignedOut;
        }
        if self.paused {
            return Display::Paused;
        }
        match &self.sync {
            Some(sync) => Display::Sync {
                status: sync.status(),
                pending: sync.pending(),
            },
            None => Display::SignedIn,
        }
    }

    /// 强制、状态行变了、或隔了 [`MENU_REFRESH`] 才重算；算出来和上次一样就不动版本号。
    fn refresh_menu(&mut self, force: bool) {
        let display = self.display();
        let status_changed = self.lines.first() != Some(&Line::Text(status_line(&display)));
        if !force && !status_changed && self.menu_built.elapsed() < MENU_REFRESH {
            return;
        }
        let account = AccountMenu {
            signed_in: self.signed_in(),
            signing_in: self.flow.signing_in(),
            consents: self
                .config
                .as_ref()
                .map(AgentConfig::consents)
                .unwrap_or_default(),
            note: self.note.clone(),
        };
        let data = self.data.as_ref().map(DataSync::status);
        let lines = build_lines(&display, &account, data.as_ref(), &self.history);
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
