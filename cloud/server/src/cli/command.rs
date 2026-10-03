//! 子命令。

use clap::Subcommand;

use super::DeviceCommand;
use super::serve::ServeArgs;

#[derive(Debug, Subcommand)]
pub enum Command {
    /// 启动 HTTP 服务。
    Serve(ServeArgs),

    /// 把数据库拷一份一致的快照到 `path`（服务运行中也可以）。
    Backup { path: std::path::PathBuf },

    /// 管理设备令牌。
    Device {
        #[command(subcommand)]
        command: DeviceCommand,
    },
}
