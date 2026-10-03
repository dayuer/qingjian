//! 落盘的同步进度：拉到哪个 `seq`、本机在服务端叫什么名字。

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::ClientError;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncState {
    /// 已经处理到的最大 `seq`。
    pub cursor: u64,

    /// 本机的设备名（`whoami` 得到），用来认出自己发的事件。
    pub device: Option<String>,

    /// 上次同步的服务器地址；换了服务器进度要清零。
    pub server: Option<String>,
}

impl SyncState {
    /// 读不到或坏了都当作从头开始：最坏是把服务端保留的历史再拉一遍。
    pub fn load(path: &Path) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, path: &Path) -> Result<(), ClientError> {
        let mut name = path.file_name().unwrap_or_default().to_os_string();
        name.push(".tmp");
        let temp = path.with_file_name(name);
        std::fs::write(&temp, serde_json::to_vec(self).unwrap_or_default())?;
        std::fs::rename(&temp, path)?;
        Ok(())
    }
}
