//! 素材的来源：发出的话，或用户手动「记一笔」。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MemoryKind {
    /// 一次发送出去的话。
    Sent,

    /// 用户手动记的一笔。
    Note,
}
