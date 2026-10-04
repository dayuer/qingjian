//! `POST /v1/memory/materials` 的响应：服务端收下了几条（重复的不算）。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryAccepted {
    pub accepted: u32,
}
