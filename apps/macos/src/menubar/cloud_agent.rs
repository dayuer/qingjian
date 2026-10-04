//! 「素笺云 ›」子菜单：素笺云链在输入法进程里（`qingjian-cloud-mac`，自带定时器与后台线程），
//! 这里只照它给的菜单行画子菜单，点了哪项把 tag 传回去。输入法不碰剪贴板与同步。
//! 自用分叉补丁，见 `cloud/docs/fork-patch.md`。
//!
//! IMK 的坑（0.1.5-local.271 / 273 实测：打开输入源菜单或 activateServer 就崩在
//! `_copySynchronizedActions:withMenuItems:` 的 `CFRelease(NULL)`，输入法打不了字）：IMK 把菜单拆成
//! 「动作列表」（含隐藏项）和「展开的条目」两份按下标对齐，**带子菜单的父项排在隐藏项之后**就对不上，
//! 递归时拿到 nil。所以这个父项必须排在「有新版本」（平时隐藏）之前，紧挨着「模糊音」。
//! 子菜单里放分隔线、无动作的项、运行中整个换掉子菜单都没事——`cloud/scripts/imk-menu-repro.swift` 逐项验过。

use std::cell::Cell;

use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_app_kit::{NSMenu, NSMenuItem};
use qingjian_cloud_mac::Line;

use super::MenuAction;
use super::menu::action_item;
use super::target::MenuTarget;

pub struct CloudAgentMenu {
    /// 挂在输入法菜单上的父项；素笺云没启动（或出错停了）时隐藏且不挂子菜单。
    item: Retained<NSMenuItem>,

    /// 上次画的菜单行版本号，没变就不重画。
    drawn: Cell<Option<u64>>,
}

impl CloudAgentMenu {
    pub fn new(mtm: MainThreadMarker, target: &MenuTarget) -> Self {
        let item = action_item(mtm, "素笺云", None, target);
        item.setHidden(true);
        Self {
            item,
            drawn: Cell::new(None),
        }
    }

    pub fn item(&self) -> &NSMenuItem {
        &self.item
    }

    /// 每秒一次：菜单行的版本号变了才重画；每次新建一份子菜单整个换上。
    pub fn sync(&self, mtm: MainThreadMarker, target: &MenuTarget) {
        let revision = qingjian_cloud_mac::menu_revision();
        if self.drawn.get() == Some(revision) {
            return;
        }
        self.drawn.set(Some(revision));
        let lines = qingjian_cloud_mac::menu_lines();
        if lines.is_empty() {
            self.item.setHidden(true);
            self.item.setSubmenu(None);
            return;
        }
        let submenu = NSMenu::new(mtm);
        submenu.setAutoenablesItems(false);
        for line in &lines {
            submenu.addItem(&draw_line(mtm, line, target));
        }
        self.item.setSubmenu(Some(&submenu));
        self.item.setHidden(false);
    }
}

fn draw_line(mtm: MainThreadMarker, line: &Line, target: &MenuTarget) -> Retained<NSMenuItem> {
    match line {
        Line::Separator => NSMenuItem::separatorItem(mtm),
        Line::Text(title) => {
            let item = action_item(mtm, title, None, target);
            item.setEnabled(false);
            item
        }
        Line::Action(title, tag) => {
            let action = MenuAction::CloudAgent(*tag);
            // tag 超出转发范围（-100..100）的不该有；真有就画成不可点
            let valid = MenuAction::from_tag(action.tag()) == Some(action);
            let item = action_item(mtm, title, valid.then_some(action), target);
            item.setEnabled(valid);
            item
        }
    }
}
