//! 本地整句模型的异步重排：壳在停键 [`DEBOUNCE`] 后调一拍 [`Session::rescore_tick`] 把整句路径送去后台打分，
//! 打分线程算完就调壳设的回调（[`Session::set_rescore_notify`]），壳在主线程上再调一拍取结果、只重排本地候选
//! （云端插进来的词原样留着，不再发一次云请求）。`qj_poll` 的 250ms 定时器也顺带调它，回调丢了也不至于不重排。

use std::time::Duration;

use super::cloud::CLOUD_POSITION;
use super::{MAX_CANDIDATES, Session};
use crate::entry::Entry;

/// 最后一次改缓冲后停这么久才送去打分：连打时不白算（与 Mac 壳一致）。壳的一次性定时器按这个数排。
pub const DEBOUNCE: Duration = Duration::from_millis(80);

impl Session {
    /// 后台打分算完时在后台线程上调的回调；`None` 清掉。壳在回调里把 [`Self::rescore_tick`] 排进主线程。
    pub fn set_rescore_notify(&mut self, notify: Option<Box<dyn Fn() + Send + Sync>>) {
        self.engine.set_rescore_notifier(notify);
    }

    /// 一拍：分回来了重排本地候选并返回 `true`（候选栏要重画）；停键够久且有没打分的路径就送去打分。
    pub fn rescore_tick(&mut self) -> bool {
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
