//! `PUT /v1/memory/contacts/{contact_id}` 的请求体：登记一个对象，成功是 204。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContactRegistration {
    /// 场景 id，同 `MemoryItem::scene`：2026-10-05 起登记对象时也不带场景，
    /// 恒为 `None` 且不序列化，留着只是让老数据读得进。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scene: Option<String>,
}
