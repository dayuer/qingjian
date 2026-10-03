//! 青简 Cloud 纠错闭环：作为一台「设备」接服务器，从汇总的输入日志与学习数据里找问题，
//! 大模型修正，用上游 qingjian-cli 回放把关，通过的修正以学习数据推回服务器、各设备同步拿到。设计见 cloud/docs/design.md。

mod args;
mod engine_cli;
mod error;
mod llm;
mod logs;
mod pinyin;
mod proposal;
mod run;
mod state;
mod steps;

use std::process::ExitCode;
use std::time::Duration;

use clap::Parser;

fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .with_ansi(std::io::IsTerminal::is_terminal(&std::io::stderr()))
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();
    let args = args::Args::parse();
    if args.token.trim().is_empty() || args.server.trim().is_empty() {
        eprintln!(
            "错误：要给服务器地址与 tuner 的令牌（QINGJIAN_CLOUD_SERVER / QINGJIAN_TUNER_TOKEN）"
        );
        return ExitCode::FAILURE;
    }
    loop {
        match run::run_once(&args) {
            Ok(report) => println!("{report}"),
            Err(error) => {
                tracing::error!(%error, "纠错闭环失败");
                if args.every_hours == 0 {
                    return ExitCode::FAILURE;
                }
            }
        }
        if args.every_hours == 0 {
            return ExitCode::SUCCESS;
        }
        std::thread::sleep(Duration::from_secs(args.every_hours * 3600));
    }
}
