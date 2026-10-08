//! 本地整句模型的异步重排：键盘控制器每 250ms 调一次 `qj_poll`，这里在那一拍里接上加载好的模型、
//! 停键够久就把整句路径送去后台打分、分回来了就只重排本地候选（云端插进来的词原样留着，不再发一次云请求）。
//! 照的是 Mac 壳 `host/model` 的防抖 + 轮询，只是借了现成的定时器。

use std::time::Duration;

use super::cloud::CLOUD_POSITION;
use super::{MAX_CANDIDATES, Session};
use crate::entry::Entry;

/// 最后一次改缓冲后停这么久才送去打分：连打时不白算，又在 250ms 的轮询里最多晚一拍。
const DEBOUNCE: Duration = Duration::from_millis(150);

impl Session {
    /// 轮询一拍：分回来了重排本地候选并返回 `true`（候选栏要重画）。
    pub(super) fn poll_rescoring(&mut self) -> bool {
        self.attach_loaded_model();
        if !self.composing() {
            return false;
        }
        if self.engine.poll_rescoring() {
            self.requery_local();
            return true;
        }
        let idle = self
            .last_edit
            .is_some_and(|edited| edited.elapsed() >= DEBOUNCE);
        if idle && !self.engine.rescoring_in_flight() && self.engine.rescoring_pending() {
            self.engine.request_rescoring();
        }
        false
    }

    /// 带着新到的神经分重查一遍：本地候选换成新次序，云端的词按原样插回第 [`CLOUD_POSITION`] 格起。
    fn requery_local(&mut self) {
        let Ok(query) = self.engine.query() else {
            return;
        };
        let cloud: Vec<Entry> = std::mem::take(&mut self.entries)
            .into_iter()
            .filter(|entry| matches!(entry, Entry::Cloud(_)))
            .collect();
        self.preedit = query.marked_text();
        self.entries.extend(
            query
                .candidates
                .items
                .into_iter()
                .take(MAX_CANDIDATES)
                .map(Entry::Local),
        );
        let mut position = CLOUD_POSITION.min(self.entries.len());
        for entry in cloud {
            if self.entries.iter().all(|e| e.text() != entry.text()) {
                self.entries.insert(position, entry);
                position += 1;
            }
        }
    }
}
