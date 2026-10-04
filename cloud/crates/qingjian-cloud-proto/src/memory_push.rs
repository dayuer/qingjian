//! `POST /v1/memory/materials` 的请求体：一次最多 `MAX_MEMORY_ITEMS` 条，成功是 202 [`crate::MemoryAccepted`]。

use serde::{Deserialize, Serialize};

use crate::MemoryItem;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryPush {
    pub items: Vec<MemoryItem>,
}
