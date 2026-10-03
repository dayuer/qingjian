//! 「青简 Cloud ›」子菜单第一行显示的状态。

use qingjian_cloud_client::Status;

/// 子菜单显示的状态：连接状态之外还有「没配置好」与「暂停」。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Display {
    Unconfigured(String),
    Paused,
    Sync { status: Status, pending: usize },
}
