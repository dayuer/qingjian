//! 命令行：`serve` 起服务，`device` 管理设备令牌。数据目录缺省 `/data`（Docker 卷），可用 `QINGJIAN_CLOUD_DATA` 改。

mod command;
mod device_command;
mod serve;

use std::path::PathBuf;

use clap::Parser;

use crate::{ServerError, Store};

pub use command::Command;
pub use device_command::DeviceCommand;

/// 数据库在数据目录里的文件名。
const DATABASE_FILE: &str = "qingjian-cloud.sqlite3";

#[derive(Debug, Parser)]
#[command(name = "qingjian-cloud", version, about = "青简 Cloud 服务端")]
pub struct Cli {
    /// 数据目录（SQLite 数据库所在）。
    #[arg(
        long,
        env = "QINGJIAN_CLOUD_DATA",
        default_value = "/data",
        global = true
    )]
    pub data: PathBuf,

    #[command(subcommand)]
    pub command: Command,
}

impl Cli {
    pub fn run(self) -> Result<(), ServerError> {
        let store = Store::open(&self.data.join(DATABASE_FILE))?;
        match self.command {
            Command::Serve(args) => serve::run(store, args),
            Command::Backup { path } => {
                store.backup_to(&path)?;
                println!("已备份到 {}", path.display());
                Ok(())
            }
            Command::Device { command } => command.run(&store),
        }
    }
}
