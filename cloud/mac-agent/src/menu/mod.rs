//! 菜单栏图标与下拉菜单：连接状态、最近的剪贴板（点一条复制到本机）、暂停、打开配置 / 日志、退出。
//! 菜单项的 tag 区分动作：非负数是历史条目的下标，负数是固定动作。

mod display;
mod target;

use objc2::rc::Retained;
use objc2::{MainThreadMarker, sel};
use objc2_app_kit::{
    NSImage, NSMenu, NSMenuItem, NSStatusBar, NSStatusItem, NSVariableStatusItemLength,
};
use objc2_foundation::{NSString, ns_string};
use qingjian_cloud_client::{DataStatus, Status};

use crate::history::History;

pub use display::Display;
pub use target::Target;

pub const TAG_PAUSE: isize = -1;
pub const TAG_RELOAD: isize = -2;
pub const TAG_OPEN_CONFIG: isize = -3;
pub const TAG_OPEN_LOGS: isize = -4;
pub const TAG_QUIT: isize = -5;
pub const TAG_SYNC_NOW: isize = -6;
pub const TAG_USE_LLM: isize = -7;

pub struct StatusMenu {
    item: Retained<NSStatusItem>,

    target: Retained<Target>,

    mtm: MainThreadMarker,
}

impl StatusMenu {
    pub fn new(mtm: MainThreadMarker) -> Self {
        let item = NSStatusBar::systemStatusBar().statusItemWithLength(NSVariableStatusItemLength);
        item.setAutosaveName(Some(ns_string!("QingjianCloudAgent")));
        if let Some(button) = item.button(mtm) {
            let image = NSImage::imageWithSystemSymbolName_accessibilityDescription(
                ns_string!("doc.on.clipboard"),
                Some(ns_string!("青简 Cloud")),
            );
            match image {
                Some(image) => {
                    image.setTemplate(true);
                    button.setImage(Some(&image));
                }
                None => button.setTitle(ns_string!("剪")),
            }
        }
        Self {
            item,
            target: Target::new(mtm),
            mtm,
        }
    }

    /// 按当前状态与历史重建整个菜单（条目少，重建比增量改简单可靠）。
    pub fn update(&self, display: &Display, data: Option<&DataStatus>, history: &History) {
        let mtm = self.mtm;
        if let Some(button) = self.item.button(mtm) {
            // 离线、没配置、暂停时图标变灰，不弹任何通知
            let dim = !matches!(
                display,
                Display::Sync {
                    status: Status::Online,
                    ..
                }
            );
            button.setAppearsDisabled(dim);
        }
        let menu = NSMenu::new(mtm);
        menu.setAutoenablesItems(false);
        menu.addItem(&self.disabled(&status_line(display)));
        if let Some(data) = data {
            menu.addItem(&self.disabled(&data_line(data)));
        }
        menu.addItem(&NSMenuItem::separatorItem(mtm));
        let mut any = false;
        for (index, entry) in history.entries().enumerate() {
            menu.addItem(&self.action(&entry.title(), index as isize));
            any = true;
        }
        if !any {
            menu.addItem(&self.disabled("还没有剪贴板记录"));
        }
        menu.addItem(&NSMenuItem::separatorItem(mtm));
        let pause = if matches!(display, Display::Paused) {
            "继续同步"
        } else {
            "暂停同步"
        };
        menu.addItem(&self.action(pause, TAG_PAUSE));
        if data.is_some() {
            menu.addItem(&self.action("立即同步学习数据", TAG_SYNC_NOW));
        }
        if !matches!(display, Display::Unconfigured(_)) {
            menu.addItem(&self.action("让青简使用 Cloud 的大模型", TAG_USE_LLM));
        }
        menu.addItem(&self.action("重新加载配置", TAG_RELOAD));
        menu.addItem(&self.action("打开配置文件…", TAG_OPEN_CONFIG));
        menu.addItem(&self.action("打开日志目录", TAG_OPEN_LOGS));
        menu.addItem(&NSMenuItem::separatorItem(mtm));
        menu.addItem(&self.action("退出青简 Cloud", TAG_QUIT));
        self.item.setMenu(Some(&menu));
    }

    fn action(&self, title: &str, tag: isize) -> Retained<NSMenuItem> {
        let item = unsafe {
            NSMenuItem::initWithTitle_action_keyEquivalent(
                self.mtm.alloc(),
                &NSString::from_str(title),
                Some(sel!(menuAction:)),
                ns_string!(""),
            )
        };
        unsafe { item.setTarget(Some(&self.target)) };
        item.setTag(tag);
        item
    }

    fn disabled(&self, title: &str) -> Retained<NSMenuItem> {
        let item = unsafe {
            NSMenuItem::initWithTitle_action_keyEquivalent(
                self.mtm.alloc(),
                &NSString::from_str(title),
                None,
                ns_string!(""),
            )
        };
        item.setEnabled(false);
        item
    }
}

fn status_line(display: &Display) -> String {
    match display {
        Display::Unconfigured(reason) => reason.clone(),
        Display::Paused => "已暂停同步".to_owned(),
        Display::Sync { status, pending } => {
            let base = match status {
                Status::Connecting => "正在连接…".to_owned(),
                Status::Online => "已连接".to_owned(),
                Status::Offline(error) => format!("离线，稍后自动重试（{}）", short(error)),
                Status::Unauthorized => "令牌无效：在服务器上重新登记设备".to_owned(),
            };
            if *pending > 0 {
                format!("{base} · {pending} 条待上传")
            } else {
                base
            }
        }
    }
}

fn data_line(data: &DataStatus) -> String {
    let mut line = if let Some(error) = &data.error {
        format!("学习数据：同步失败，稍后重试（{}）", short(error))
    } else if data.waiting_for_ime {
        "学习数据：等输入法合并（切到青简打几个字）".to_owned()
    } else if let Some(ms) = data.last_ok_ms {
        format!("学习数据：{}同步", ago(ms))
    } else {
        "学习数据：正在同步…".to_owned()
    };
    if data.config_conflict {
        line.push_str(" · 设置有冲突，旧的一份已备份");
    }
    line
}

/// 「刚刚」/「N 分钟前」/「N 小时前」。
fn ago(ms: i64) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or_default();
    let minutes = (now - ms).max(0) / 60_000;
    match minutes {
        0 => "刚刚".to_owned(),
        1..=59 => format!("{minutes} 分钟前"),
        _ => format!("{} 小时前", minutes / 60),
    }
}

/// 错误信息在菜单里只显示前 40 个字符，完整的在日志里。
fn short(text: &str) -> String {
    text.chars().take(40).collect()
}
