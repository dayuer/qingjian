//! 启动学习数据同步需要的配置。

use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct DataSyncConfig {
    pub server: String,

    pub token: String,

    /// 输入法的数据目录。
    pub ime_dir: PathBuf,

    /// 基线与进度放在哪。
    pub state_dir: PathBuf,

    /// 是否同步学习数据。
    pub sync_learning: bool,

    /// 是否同步 `config.toml`（设置与自定义短语）。
    pub sync_config: bool,
}
