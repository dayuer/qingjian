//! 程序主体：主线程上 0.5 秒一拍，看本机剪贴板有没有变、取收到的事件、刷新菜单。
//! 网络都在 `ClipboardSync` 的后台线程里，主线程从不等网络。

use std::cell::RefCell;

use objc2::rc::Retained;
use objc2::{MainThreadMarker, sel};
use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy};
use objc2_foundation::NSTimer;
use qingjian_cloud_client::{ClipboardSync, SyncConfig};
use qingjian_cloud_proto::EventKind;

use crate::config::AgentConfig;
use crate::history::History;
use crate::menu::{
    Display, StatusMenu, TAG_OPEN_CONFIG, TAG_OPEN_LOGS, TAG_PAUSE, TAG_QUIT, TAG_RELOAD, Target,
};
use crate::watcher::ClipboardWatcher;
use crate::{pasteboard, paths};

/// 主循环间隔（秒）。
const TICK: f64 = 0.5;

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

    /// 没配置好的原因，菜单里显示。
    unconfigured: Option<String>,

    paused: bool,

    watcher: ClipboardWatcher,

    history: History,

    /// 上次画菜单时的状态，没变就不重建菜单。
    shown: Option<Display>,

    timer: Option<Retained<NSTimer>>,
}

impl App {
    fn new(mtm: MainThreadMarker) -> Self {
        let mut app = Self {
            mtm,
            menu: StatusMenu::new(mtm),
            sync: None,
            unconfigured: None,
            paused: false,
            watcher: ClipboardWatcher::new(),
            history: History::default(),
            shown: None,
            timer: None,
        };
        app.load_config();
        app
    }

    fn load_config(&mut self) {
        // 先停旧的，再按新配置起；进度文件按服务器地址区分，换服务器会从头同步
        self.sync = None;
        self.history = History::default();
        let (Some(config_path), Some(state_dir)) = (paths::config_path(), paths::support_dir())
        else {
            self.unconfigured = Some("找不到用户目录".to_owned());
            return;
        };
        let config = match AgentConfig::load(&config_path) {
            Ok(config) => config,
            Err(reason) => {
                tracing::info!(%reason, "未启用同步");
                self.unconfigured = Some(reason);
                return;
            }
        };
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

    /// 每拍：上传本机新复制的、写入别的设备刚复制的、刷新菜单。
    pub fn tick(&mut self) {
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
            TAG_QUIT => NSApplication::sharedApplication(self.mtm).terminate(None),
            index if index >= 0 => {
                if let Some(entry) = self.history.get(index as usize) {
                    let count = pasteboard::write_text(&entry.text);
                    self.watcher.note_own_write(count);
                }
            }
            _ => {}
        }
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
        if force || self.shown.as_ref() != Some(&display) {
            self.menu.update(&display, &self.history);
            self.shown = Some(display);
        }
    }
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
