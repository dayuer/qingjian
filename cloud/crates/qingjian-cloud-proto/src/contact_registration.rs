//! `PUT /v1/memory/contacts/{contact_id}` 的请求体：登记一个对象，成功是 204。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContactRegistration {
    /// 场景 id，同 `MemoryItem::scene`。
    pub scene: String,
}
