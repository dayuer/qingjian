//! 素笺云收件箱：Cloud 的常驻程序把别的设备的学习增量写成数据目录下的 `sync/inbox.tsv`，
//! 这里每拍看一眼，有就合并进学习数据、落盘、删掉文件。没装 Cloud 时文件不存在，什么都不做。
//! 格式与合并规则见 `qingjian-learning` 的 `frequency_learner/remote.rs`，整体设计见 `cloud/docs/design.md`。

use std::io::ErrorKind;

use crate::app::paths;
use crate::host::Host;

/// 收件箱在数据目录里的相对路径。
const INBOX: &str = "sync/inbox.tsv";

impl Host {
    /// 先合并、落盘，最后才删文件：中途崩溃时文件还在，下次启动重新合并（最坏多算一次，不会丢）。
    pub(in crate::host) fn apply_cloud_inbox(&mut self) {
        let Some(path) = paths::user_data_dir().map(|dir| dir.join(INBOX)) else {
            return;
        };
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(error) if error.kind() == ErrorKind::NotFound => return,
            Err(error) => {
                tracing::warn!(%error, "素笺云收件箱读不了");
                return;
            }
        };
        let applied = self.engine.learner_mut().merge_remote(&text);
        if let Err(error) = std::fs::remove_file(&path) {
            tracing::warn!(%error, "素笺云收件箱删不掉");
        }
        tracing::info!(applied, "合并了别的设备的学习数据");
    }
}
