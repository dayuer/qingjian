//! `qingjian-cloud`：`serve` 起服务，`device add|list|remove` 管理设备令牌。

use std::process::ExitCode;

use clap::Parser;
use qingjian_cloud_server::cli::Cli;

fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .with_ansi(std::io::IsTerminal::is_terminal(&std::io::stderr()))
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();
    match Cli::parse().run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("错误：{error}");
            ExitCode::FAILURE
        }
    }
}
