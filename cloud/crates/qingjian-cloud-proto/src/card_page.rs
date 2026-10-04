//! `GET /v1/memory/cards?since=<seq>&limit=500` 的响应：`seq > since` 的卡（含墓碑），按 seq 升序。

use serde::{Deserialize, Serialize};

use crate::MemoryCard;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CardPage {
    pub cards: Vec<MemoryCard>,

    /// 服务端当前最大的 seq。
    pub latest: i64,
}
