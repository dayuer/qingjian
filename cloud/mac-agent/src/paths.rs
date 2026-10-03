//! 本程序的目录：配置与同步进度在 Application Support，日志在 Logs。与输入法的 `Qingjian` 目录分开，互不干扰。

use std::path::PathBuf;

const DIR_NAME: &str = "QingjianCloud";

fn home() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

/// `~/Library/Application Support/QingjianCloud/`
pub fn support_dir() -> Option<PathBuf> {
    Some(home()?.join("Library/Application Support").join(DIR_NAME))
}

/// `~/Library/Logs/QingjianCloud/`
pub fn log_dir() -> Option<PathBuf> {
    let dir = home()?.join("Library/Logs").join(DIR_NAME);
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir)
}

/// 配置文件路径。
pub fn config_path() -> Option<PathBuf> {
    Some(support_dir()?.join("config.toml"))
}

/// 输入法的数据目录 `~/Library/Application Support/Qingjian/`：学习数据、`config.toml`，以及收件箱 `sync/inbox.tsv`。
pub fn ime_dir() -> Option<PathBuf> {
    Some(home()?.join("Library/Application Support/Qingjian"))
}
