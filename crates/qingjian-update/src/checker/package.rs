//! 新版本的本机安装包：从索引里取下载地址与校验和，下载后放数据目录的 `updates/`。

use serde::{Deserialize, Serialize};

/// 索引里给这台机器的安装包；有它才能自动下载。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Package {
    /// 文件名（`qingjian-0.1.5-local.270-macos-arm64.pkg`），下载后按它存。
    pub file: String,

    pub url: String,

    /// 小写十六进制；下载后对不上就丢掉。
    pub sha256: String,
}
