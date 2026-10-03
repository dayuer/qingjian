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

    /// 大模型用量：按天、按设备。
    Usage {
        /// 最近多少天。
        #[arg(long, default_value_t = 30)]
        days: u32,
    },

    /// 把汇总的输入日志导出成 jsonl（可给 qingjian-cli --replay 用）。
    ExportLog {
        /// 输出文件；不填打印到标准输出。
        #[arg(long)]
        out: Option<std::path::PathBuf>,

        /// 只导出这台设备的。
        #[arg(long)]
        device: Option<String>,
    },

    /// 管理设备令牌。
    Device {
        #[command(subcommand)]
        command: DeviceCommand,
    },
}
