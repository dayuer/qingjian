//! 「素笺云」的行：直接摊在输入法菜单的**顶层**。素笺云链在输入法进程里
//! （`qingjian-cloud-mac`，自带定时器与后台线程），这里只照它给的行画菜单项，点了把 tag 传回去。
//! 自用分叉补丁，见 `cloud/docs/fork-patch.md`。
//!
//! **为什么不用子菜单**：IMK 的输入法菜单只转发**顶层**项的动作，子菜单里的项点了一点反应都没有
//! （2026-10-06 实测：菜单项画出来 tag/enabled/action 全对，点击不到 `menuAction:`）。原先套在
//! 「素笺云 ›」子菜单下，用户根本点不动。

use std::cell::Cell;

use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_app_kit::NSMenuItem;
use qingjian_cloud_mac::Line;

use super::MenuAction;
use super::menu::action_item;
use super::target::MenuTarget;

/// 素笺云的行：可点的一组在前，纯展示的一组在后（见 [`CloudAgentMenu::rows`]）。
pub type CloudRows = (Vec<Retained<NSMenuItem>>, Vec<Retained<NSMenuItem>>);

pub struct CloudAgentMenu {
    /// 上次画的菜单行版本号，没变就不重画。
    drawn: Cell<Option<u64>>,
}

impl CloudAgentMenu {
    pub fn new() -> Self {
        Self {
            drawn: Cell::new(None),
        }
    }

    /// 素笺云的行，分成**可点**与**纯展示**两组：IMK 的输入法菜单要求可点项连成一片，
    /// 中间夹纯展示项会在每次按键后把会话停用再新建（打不了字），所以调用方要分两段排。
    /// 分隔线在顶层由调用方自己加，这里丢掉。
    /// 版本号没变返回 `None`（调用方不重建菜单）。
    pub fn rows(&self, mtm: MainThreadMarker, target: &MenuTarget) -> Option<CloudRows> {
        let revision = qingjian_cloud_mac::menu_revision();
        if self.drawn.get() == Some(revision) {
            return None;
        }
        self.drawn.set(Some(revision));
        let mut actions = Vec::new();
        let mut notes = Vec::new();
        for line in qingjian_cloud_mac::menu_lines() {
            if matches!(line, Line::Separator) {
                continue;
            }
            let item = draw_line(mtm, &line, target);
            if item.action().is_some() {
                actions.push(item);
            } else {
                notes.push(item);
            }
        }
        Some((actions, notes))
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
