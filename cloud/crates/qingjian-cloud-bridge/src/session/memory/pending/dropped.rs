//! 待办补写成素材时被拒绝、没记上的「记一笔」：只记条数与原因，不记原话，也是 `memory/dropped-keyboard.json` 的内容。
//! 键盘下次出现时用 `qj_memory_dropped` 取走、提示一次（「有 n 条没记上：……」），取完清零。

use serde::{Deserialize, Serialize};

use crate::memory::MemoryError;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct DroppedNotes {
    /// 这个人没整理的素材满了，装不下。
    pub material_limit: usize,

    /// 这个人已经被忘掉（名单上没有了）。
    pub contact_gone: usize,
}

impl DroppedNotes {
    pub fn is_empty(&self) -> bool {
        self.material_limit == 0 && self.contact_gone == 0
    }

    /// 按拒绝的原因记 `count` 条：素材满了记 `material_limit`，其余（补写时只会是名单上没这个人）记 `contact_gone`。
    pub(in crate::session) fn add(&mut self, error: &MemoryError, count: usize) {
        let slot = match error {
            MemoryError::MaterialLimit { .. } => &mut self.material_limit,
            _ => &mut self.contact_gone,
        };
        *slot = slot.saturating_add(count);
    }
}
