//! 启动同步需要的配置。

use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct SyncConfig {
    /// 服务器地址，如 `https://cloud.example.com`。
    pub server: String,

    /// 设备令牌（`qingjian-cloud device add` 打印的那串）。
    pub token: String,

    /// 进度与离线队列放在哪个目录。
    pub state_dir: PathBuf,
}
