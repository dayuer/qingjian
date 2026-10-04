//! 一条等着写的「记一笔」，也是 `pending-keyboard.jsonl` 里的一行。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(in crate::session) struct PendingNote {
    pub(in crate::session) contact_id: String,

    pub(in crate::session) text: String,

    /// 用户点「记」的时间（Unix 秒），补写时仍按这个时间建卡。
    pub(in crate::session) at: i64,
}
