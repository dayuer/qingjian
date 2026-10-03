//! 菜单栏显示的状态。

use qingjian_cloud_client::Status;

/// 菜单栏显示时的状态：连接状态之外还有「没配置好」与「暂停」。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Display {
    Unconfigured(String),
    Paused,
    Sync { status: Status, pending: usize },
}
