//! 素笺云的 Mac 端，链进输入法进程（不另起常驻程序）：本机复制的文本上传、别的设备复制的写进本机剪贴板，
//! 学习数据、`config.toml` 与输入日志在各台 Mac 之间同步。自带 0.5 秒的定时器（输入法自己的定时器失焦就停），
//! 网络都在后台线程；只在当前输入法是素笺时同步，切走 30 秒后暂停。配置在 `~/Library/Application Support/QingjianCloud/config.toml`。
//!
//! 输入法调四个函数：启动时 [`start`]，「素笺云 ›」子菜单照 [`menu_lines`] 画，点了调 [`perform`]；
//! 云联想选素笺云时用 [`llm_endpoint`] 拿大模型代理的地址与令牌；
//! 换账号清了输入日志后 [`take_input_log_reset`] 返回一次 true，输入法据此丢掉输入日志写入端的缓冲并重开。
//! 设计见 `cloud/docs/design.md`，输入法侧的挂钩见 `cloud/docs/fork-patch.md`。

// 别的平台上只编配置解析等可移植部分（让整个 workspace 在 Linux 上能构建、测试）
#![cfg_attr(not(target_os = "macos"), allow(dead_code))]

mod account;
mod config;
mod llm_endpoint;
mod menu;
mod paths;

#[cfg(target_os = "macos")]
mod input_source;
#[cfg(target_os = "macos")]
mod pasteboard;
#[cfg(target_os = "macos")]
mod service;
#[cfg(target_os = "macos")]
mod timer;
#[cfg(target_os = "macos")]
mod watcher;

pub use account::take_input_log_reset;
pub use llm_endpoint::LlmEndpoint;
pub use menu::{
    Line, TAG_CANCEL_JOIN, TAG_CLEAR_INPUT_LOG, TAG_CREATE_SPACE, TAG_JOIN_WITH_CODE,
    TAG_OPEN_SETTINGS, TAG_SIGN_OUT,
};
#[cfg(target_os = "macos")]
pub use service::{
    CloudDevice, CloudStatus, clear_input_log, consent_enabled, create_space, join_with_code,
    llm_endpoint, menu_lines, menu_revision, pause_toggle, perform, refresh_account, revoke_device,
    start, status, sync_now, toggle_feature, unbind,
};
