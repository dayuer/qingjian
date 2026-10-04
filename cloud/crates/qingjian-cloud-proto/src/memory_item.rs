//! 一条上传的素材（已在客户端脱敏）。

use serde::{Deserialize, Serialize};

use crate::{MemoryKind, Scene};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryItem {
    /// 客户端生成的 id，服务端按 `(session_id, client_id)` 去重。
    pub client_id: String,

    /// 对象的 id；不为空时必须已登记。
    pub contact_id: Option<String>,

    pub scene: Scene,

    pub kind: MemoryKind,

    /// 脱敏后的文字，最多 `MAX_MEMORY_TEXT_BYTES` 字节。
    pub text: String,

    /// 发生的时间，Unix 秒。
    pub at: i64,
}
