//! 「素笺云 ›」子菜单第一行显示的状态。

use qingjian_cloud_client::Status;

/// 子菜单显示的状态：连接状态之外还有「没配置好」「没登录」「登录了但没开剪贴板」与「暂停」。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Display {
    /// 配置文件读不了或同步起不来，带原因。
    Unconfigured(String),

    /// 还没登录（或令牌失效后清掉了）。
    SignedOut,

    /// 登录了，但没开跨设备剪贴板，没有连接状态可显示。
    SignedIn,

    Paused,

    Sync {
        status: Status,
        pending: usize,
    },
}
