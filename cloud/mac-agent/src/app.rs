//! 程序主体：主线程上 0.5 秒一拍，看本机剪贴板有没有变、取收到的事件、刷新菜单。
//! 网络都在 `ClipboardSync` 的后台线程里，主线程从不等网络。

use std::cell::RefCell;

use objc2::rc::Retained;
use objc2::{MainThreadMarker, sel};
use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy};
use objc2_foundation::NSTimer;
use qingjian_cloud_client::{ClipboardSync, DataStatus, DataSync, DataSyncConfig, SyncConfig};
use qingjian_cloud_proto::EventKind;

use crate::config::AgentConfig;
use crate::history::History;
use crate::menu::{
    Display, StatusMenu, TAG_OPEN_CONFIG, TAG_OPEN_LOGS, TAG_PAUSE, TAG_QUIT, TAG_RELOAD,
    TAG_SYNC_NOW, TAG_USE_LLM, Target,
};
use crate::watcher::ClipboardWatcher;
use crate::{ime_config, input_source, pasteboard, paths};

/// 主循环间隔（秒）。
const TICK: f64 = 0.5;

/// 每隔这么多拍看一次当前输入法（2 秒）。
const INPUT_SOURCE_EVERY: u32 = 4;

/// 连续这么久不是青简才退出：密码框里系统会临时切到英文键盘，不能一进密码框就退。
const OTHER_INPUT_GRACE: std::time::Duration = std::time::Duration::from_secs(30);

/// 状态没变也至少隔这么久重写一次 `menu.txt`：输入法据它的修改时间判断本程序还在不在。
const MENU_HEARTBEAT: std::time::Duration = std::time::Duration::from_secs(60);

/// 别的设备复制的条目，在这么久以内收到才自动写进本机剪贴板（毫秒）。
/// 更早的（离线很久后补拉到的）只进菜单里的历史，免得突然覆盖用户正在用的剪贴板。
const AUTO_PASTE_WINDOW_MS: i64 = 120_000;

thread_local! {
    static APP: RefCell<Option<App>> = const { RefCell::new(None) };
}

/// 在主线程上取 App；菜单回调重入时（正在处理上一个回调）直接跳过。
pub fn with(f: impl FnOnce(&mut App)) {
    APP.with(|cell| {
        if let Ok(mut guard) = cell.try_borrow_mut()
            && let Some(app) = guard.as_mut()
        {
            f(app);
        }
    });
}

pub fn run() {
    let mtm = MainThreadMarker::new().expect("must run on the main thread");
    let ns_app = NSApplication::sharedApplication(mtm);
    ns_app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);
    let mut app = App::new(mtm);
    app.refresh_menu(true);
    let target = Target::new(mtm);
    let timer = unsafe {
        NSTimer::scheduledTimerWithTimeInterval_target_selector_userInfo_repeats(
            TICK,
            &target,
            sel!(tick:),
            None,
            true,
        )
    };
    app.timer = Some(timer);
    APP.with(|cell| *cell.borrow_mut() = Some(app));
    tracing::info!("青简 Cloud 已启动");
    ns_app.run();
}

pub struct App {
    mtm: MainThreadMarker,

    menu: StatusMenu,

    /// 配置好了才有。
    sync: Option<ClipboardSync>,

    /// 学习数据与设置的同步；配置里关掉或没配置时为 `None`。
    data: Option<DataSync>,

    /// 上次画菜单时的学习数据状态。
    shown_data: Option<DataStatus>,

    /// 服务器地址，「使用 Cloud 的大模型」时写进输入法配置。
    server: Option<String>,

    /// 没配置好的原因，菜单里显示。
    unconfigured: Option<String>,

    paused: bool,

    watcher: ClipboardWatcher,

    history: History,

    /// 上次画菜单时的状态，没变就不重建菜单。
    shown: Option<Display>,

    /// 上次写 `menu.txt` 的时间，心跳用。
    menu_written: std::time::Instant,

    /// 拍数，按 [`INPUT_SOURCE_EVERY`] 看当前输入法。
    ticks: u32,

    /// 从什么时候起当前输入法不是青简。
    other_input_since: Option<std::time::Instant>,

    timer: Option<Retained<NSTimer>>,
}

impl App {
    fn new(mtm: MainThreadMarker) -> Self {
        let mut app = Self {
            mtm,
            menu: StatusMenu::new(mtm),
            sync: None,
            data: None,
            shown_data: None,
            server: None,
            unconfigured: None,
            paused: false,
            watcher: ClipboardWatcher::new(),
            history: History::default(),
            shown: None,
            menu_written: std::time::Instant::now(),
            ticks: 0,
            other_input_since: None,
            timer: None,
        };
        app.load_config();
        app
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
            Ok(config) => {
                self.menu.set_visible(config.menu_bar);
                config
            }
            Err(reason) => {
                tracing::info!(%reason, "未启用同步");
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
                tracing::warn!(%error, "同步启动失败");
                self.unconfigured = Some(format!("同步启动失败：{error}"));
            }
        }
    }

    /// 每拍：执行输入法子菜单发来的命令、上传本机新复制的、写入别的设备刚复制的、刷新菜单。
    pub fn tick(&mut self) {
        self.ticks = self.ticks.wrapping_add(1);
        if self.ticks.is_multiple_of(INPUT_SOURCE_EVERY) {
            if input_source::qingjian_selected() == Some(false) {
                let since = *self
                    .other_input_since
                    .get_or_insert_with(std::time::Instant::now);
                if since.elapsed() >= OTHER_INPUT_GRACE {
                    self.quit("切到了别的输入法");
                    return;
                }
            } else {
                self.other_input_since = None;
            }
        }
        for tag in take_commands() {
            self.perform(tag);
        }
        let copied = self.watcher.poll();
        let Some(sync) = &self.sync else {
            self.refresh_menu(false);
            return;
        };
        if let Some(text) = copied
            && !self.paused
        {
            sync.copy(text);
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
                let count = pasteboard::write_text(text);
                self.watcher.note_own_write(count);
                tracing::info!(seq = incoming.event.seq, device = %incoming.event.device, "写入剪贴板");
            }
        }
        self.refresh_menu(changed);
    }

    /// 菜单项的动作。
    pub fn perform(&mut self, tag: isize) {
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
            TAG_OPEN_LOGS => {
                if let Some(dir) = paths::log_dir() {
                    open(&[&dir.to_string_lossy()]);
                }
            }
            TAG_QUIT => self.quit("菜单里点了退出"),
            index if index >= 0 => {
                if let Some(entry) = self.history.get(index as usize) {
                    let count = pasteboard::write_text(&entry.text);
                    self.watcher.note_own_write(count);
                }
            }
            _ => {}
        }
    }

    /// 正常退出：删掉 `menu.txt`（输入法据此收起子菜单、下次切回青简时拉起本程序）。
    /// launchd 的 KeepAlive 只在异常退出时拉起，正常退出不会被立刻拉回来。
    fn quit(&mut self, reason: &str) {
        tracing::info!(reason, "青简 Cloud 退出");
        if let Some(path) = paths::menu_file() {
            let _ = std::fs::remove_file(path);
        }
        NSApplication::sharedApplication(self.mtm).terminate(None);
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

    fn refresh_menu(&mut self, force: bool) {
        let display = self.display();
        let data = self.data.as_ref().map(DataSync::status);
        let stale = self.menu_written.elapsed() >= MENU_HEARTBEAT;
        if force || stale || self.shown.as_ref() != Some(&display) || self.shown_data != data {
            self.menu.update(&display, data.as_ref(), &self.history);
            self.shown = Some(display);
            self.shown_data = data;
            self.menu_written = std::time::Instant::now();
        }
    }
}

/// 取走 `commands/` 里输入法写的命令（一个文件一个 tag），按文件名（写入时的纳秒时间）顺序返回。
fn take_commands() -> Vec<isize> {
    let Some(dir) = paths::commands_dir() else {
        return Vec::new();
    };
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut files: Vec<_> = entries.flatten().map(|entry| entry.path()).collect();
    files.sort();
    files
        .into_iter()
        .filter_map(|path| {
            let text = std::fs::read_to_string(&path).ok();
            let _ = std::fs::remove_file(&path);
            text?.trim().parse().ok()
        })
        .collect()
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
