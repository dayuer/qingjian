//! 键盘拿不到 `memory/.lock`（200 毫秒超时）时先记在内存里的写入：「记一笔」按顺序排队，切场景只留最新一次。
//! 下次 refresh、poll、flush 或下一次写入时重试；卡片与场景在内存里已经生效，只是磁盘上晚几步。

mod note;

use std::collections::VecDeque;

use qingjian_cloud_proto::Scene;

pub(in crate::session) use self::note::PendingNote;

/// 待写的「记一笔」最多几条，超出丢最旧的。
pub(in crate::session) const MAX_PENDING_NOTES: usize = 32;

#[derive(Debug, Default)]
pub(in crate::session) struct PendingWrites {
    notes: VecDeque<PendingNote>,

    scope: Option<(Scene, Option<String>)>,
}

impl PendingWrites {
    pub(in crate::session) fn is_empty(&self) -> bool {
        self.notes.is_empty() && self.scope.is_none()
    }

    #[cfg(test)]
    pub(in crate::session) fn has_scope(&self) -> bool {
        self.scope.is_some()
    }

    pub(in crate::session) fn note_count(&self) -> usize {
        self.notes.len()
    }

    /// 排到队尾；满了丢最旧的（记日志）。
    pub(in crate::session) fn push_note(&mut self, note: PendingNote) {
        if self.notes.len() >= MAX_PENDING_NOTES {
            self.notes.pop_front();
            tracing::warn!("待写的记一笔太多，丢掉最旧的一条");
        }
        self.notes.push_back(note);
    }

    pub(in crate::session) fn front_note(&self) -> Option<&PendingNote> {
        self.notes.front()
    }

    pub(in crate::session) fn pop_note(&mut self) -> Option<PendingNote> {
        self.notes.pop_front()
    }

    /// 后一次覆盖前一次。
    pub(in crate::session) fn set_scope(&mut self, scene: Scene, contact: Option<String>) {
        self.scope = Some((scene, contact));
    }

    pub(in crate::session) fn take_scope(&mut self) -> Option<(Scene, Option<String>)> {
        self.scope.take()
    }

    /// 重试没成功时放回去；期间若有更新的一次（重入）就不覆盖它。
    pub(in crate::session) fn restore_scope(&mut self, scope: (Scene, Option<String>)) {
        self.scope.get_or_insert(scope);
    }
}
