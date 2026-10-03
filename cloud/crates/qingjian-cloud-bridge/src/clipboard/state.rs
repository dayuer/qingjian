//! 落盘的剪贴板进度：拉到哪了、哪一条已经给过用户（插入或关掉），键盘进程被杀后接着用。

use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ClipState {
    /// 已经看过的最大事件序号。
    pub seen: u64,

    /// 用户插入或关掉过的那条 `ClipAdded` 的序号；不大于它的不再给。
    pub handled: u64,
}

impl ClipState {
    pub fn load(path: &Path) -> Self {
        std::fs::read(path)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, path: &Path) {
        let temp = path.with_extension("json.tmp");
        let written = serde_json::to_vec(self)
            .map_err(std::io::Error::other)
            .and_then(|bytes| std::fs::write(&temp, bytes))
            .and_then(|()| std::fs::rename(&temp, path));
        if let Err(error) = written {
            tracing::warn!(%error, "剪贴板进度写不了");
        }
    }
}
