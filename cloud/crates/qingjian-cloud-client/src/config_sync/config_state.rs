//! 配置同步的进度：上次一致时的服务器版本与文件指纹。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigState {
    pub version: u64,

    pub hash: Option<u64>,
}
