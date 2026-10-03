//! 青简 Cloud 的 Mac 菜单栏常驻程序：本机复制的文本上传到自己的服务器，别的设备复制的写进本机剪贴板，`⌘V` 直接贴。
//! 独立的 `.app`（LSUIElement，无 Dock 图标），不是输入法的一部分。配置在
//! `~/Library/Application Support/QingjianCloud/config.toml`，日志在 `~/Library/Logs/QingjianCloud/`。

// 别的平台上只编出一个报错退出的空壳（让整个 workspace 在 Linux 上能构建、测试配置解析）
#![cfg_attr(not(target_os = "macos"), allow(dead_code))]

mod config;
mod paths;

#[cfg(target_os = "macos")]
mod app;
#[cfg(target_os = "macos")]
mod history;
#[cfg(target_os = "macos")]
mod menu;
#[cfg(target_os = "macos")]
mod pasteboard;
#[cfg(target_os = "macos")]
mod watcher;

#[cfg(target_os = "macos")]
fn main() {
    let _guard = init_logging();
    app::run();
}

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("qingjian-cloud-mac 只能在 macOS 上运行");
    std::process::exit(1);
}

/// 日志按天滚动写到 `~/Library/Logs/QingjianCloud/`；返回的 guard 活到进程结束，退出前把缓冲刷盘。
#[cfg(target_os = "macos")]
fn init_logging() -> Option<tracing_appender::non_blocking::WorkerGuard> {
    use tracing_subscriber::EnvFilter;
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let dir = paths::log_dir()?;
    let appender = tracing_appender::rolling::Builder::new()
        .rotation(tracing_appender::rolling::Rotation::DAILY)
        .filename_prefix("agent")
        .filename_suffix("log")
        .max_log_files(7)
        .build(&dir)
        .ok()?;
    let (writer, guard) = tracing_appender::non_blocking(appender);
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_ansi(false)
        .with_writer(writer)
        .init();
    Some(guard)
}
