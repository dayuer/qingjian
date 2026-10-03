//! 菜单：连接状态、最近的剪贴板（点一条复制到本机）、暂停、打开配置 / 日志。
//! 缺省不占菜单栏，内容写成 `menu.txt` 由输入法「中☁」菜单的「青简 Cloud」子菜单显示；配置 `menu_bar = true` 时另有自己的图标。
//! 菜单项的 tag 区分动作：非负数是历史条目的下标，负数是固定动作。

mod display;
mod lines;
mod target;

use objc2::rc::Retained;
use objc2::{MainThreadMarker, sel};
use objc2_app_kit::{
    NSImage, NSMenu, NSMenuItem, NSStatusBar, NSStatusItem, NSVariableStatusItemLength,
};
use objc2_foundation::{NSString, ns_string};
use qingjian_cloud_client::{DataStatus, Status};

use crate::history::History;
use crate::paths;

pub use display::Display;
pub use lines::Line;
pub use target::Target;

pub const TAG_PAUSE: isize = -1;
pub const TAG_RELOAD: isize = -2;
pub const TAG_OPEN_CONFIG: isize = -3;
pub const TAG_OPEN_LOGS: isize = -4;
pub const TAG_QUIT: isize = -5;
pub const TAG_SYNC_NOW: isize = -6;
pub const TAG_USE_LLM: isize = -7;

pub struct StatusMenu {
    /// 菜单栏图标；配置里 `menu_bar = false`（缺省）时没有。
    item: Option<Retained<NSStatusItem>>,

    target: Retained<Target>,

    mtm: MainThreadMarker,
}

impl StatusMenu {
    pub fn new(mtm: MainThreadMarker) -> Self {
        Self {
            item: None,
            target: Target::new(mtm),
            mtm,
        }
    }

    /// 按配置显示或收起菜单栏图标；之后调一次 [`Self::update`] 把菜单挂上。
    pub fn set_visible(&mut self, visible: bool) {
        match (visible, &self.item) {
            (true, None) => self.item = Some(status_item(self.mtm)),
            (false, Some(item)) => {
                NSStatusBar::systemStatusBar().removeStatusItem(item);
                self.item = None;
            }
            _ => {}
        }
    }

    /// 按当前状态与历史重算菜单：写 `menu.txt` 给输入法；有图标时也重建图标的菜单（条目少，重建比增量改简单可靠）。
    pub fn update(&self, display: &Display, data: Option<&DataStatus>, history: &History) {
        let lines = build_lines(display, data, history);
        write_menu_file(&lines);
        let Some(item) = &self.item else {
            return;
        };
        let mtm = self.mtm;
        if let Some(button) = item.button(mtm) {
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
        for line in &lines {
            match line {
                Line::Text(title) => menu.addItem(&self.disabled(title)),
                Line::Action(title, tag) => menu.addItem(&self.action(title, *tag)),
                Line::Separator => menu.addItem(&NSMenuItem::separatorItem(mtm)),
            }
        }
        // 「退出」只放在自己的图标里：输入法子菜单里点了退出就再也打不开它了
        menu.addItem(&NSMenuItem::separatorItem(mtm));
        menu.addItem(&self.action("退出青简 Cloud", TAG_QUIT));
        item.setMenu(Some(&menu));
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

fn status_item(mtm: MainThreadMarker) -> Retained<NSStatusItem> {
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
    item
}

fn build_lines(display: &Display, data: Option<&DataStatus>, history: &History) -> Vec<Line> {
    let mut lines = vec![Line::Text(status_line(display))];
    if let Some(data) = data {
        lines.push(Line::Text(data_line(data)));
    }
    lines.push(Line::Separator);
    let before = lines.len();
    for (index, entry) in history.entries().enumerate() {
        lines.push(Line::Action(entry.title(), index as isize));
    }
    if lines.len() == before {
        lines.push(Line::Text("还没有剪贴板记录".to_owned()));
    }
    lines.push(Line::Separator);
    let pause = if matches!(display, Display::Paused) {
        "继续同步"
    } else {
        "暂停同步"
    };
    lines.push(Line::Action(pause.to_owned(), TAG_PAUSE));
    if data.is_some() {
        lines.push(Line::Action("立即同步学习数据".to_owned(), TAG_SYNC_NOW));
    }
    if !matches!(display, Display::Unconfigured(_)) {
        lines.push(Line::Action(
            "让青简使用 Cloud 的大模型".to_owned(),
            TAG_USE_LLM,
        ));
    }
    lines.push(Line::Action("重新加载配置".to_owned(), TAG_RELOAD));
    lines.push(Line::Action("打开配置文件…".to_owned(), TAG_OPEN_CONFIG));
    lines.push(Line::Action("打开日志目录".to_owned(), TAG_OPEN_LOGS));
    lines
}

/// 先写临时文件再改名，输入法读不到半截。
fn write_menu_file(lines: &[Line]) {
    let Some(path) = paths::menu_file() else {
        return;
    };
    let temp = path.with_extension("txt.tmp");
    let result =
        std::fs::write(&temp, lines::to_text(lines)).and_then(|()| std::fs::rename(&temp, &path));
    if let Err(error) = result {
        tracing::warn!(%error, "menu.txt 写入失败");
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
