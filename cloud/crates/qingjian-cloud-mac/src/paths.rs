//! 目录：本模块的配置与同步进度在 `QingjianCloud`，与输入法的 `Qingjian` 数据目录分开。日志走输入法的日志。

use std::path::PathBuf;

const DIR_NAME: &str = "QingjianCloud";

fn home() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

/// `~/Library/Application Support/QingjianCloud/`
pub fn support_dir() -> Option<PathBuf> {
    Some(home()?.join("Library/Application Support").join(DIR_NAME))
}

/// 配置文件路径。
pub fn config_path() -> Option<PathBuf> {
    Some(support_dir()?.join("config.toml"))
}

/// 输入法的数据目录 `~/Library/Application Support/Qingjian/`：学习数据、`config.toml`，以及收件箱 `sync/inbox.tsv`。
pub fn ime_dir() -> Option<PathBuf> {
    Some(home()?.join("Library/Application Support/Qingjian"))
}
