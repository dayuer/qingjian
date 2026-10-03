use serde::{Deserialize, Serialize};

use super::Package;

/// 查到的新版本，够菜单和「关于」页显示用。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Available {
    pub version: String,

    /// 索引里的渠道：`alpha` / `beta` / `rc` / `stable`。
    pub channel: String,

    pub date: String,

    /// 更新日志，一行一条。
    pub notes: Vec<String>,

    /// 这台机器的安装包；索引里没给下载地址时为空，只提示。
    #[serde(default)]
    pub package: Option<Package>,
}
