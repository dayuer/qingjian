//! `GET /v1/memory/cards?since=<seq>&limit=500` 的响应：`seq > since` 的卡（含墓碑），按 seq 升序。

use serde::{Deserialize, Serialize};

use crate::MemoryCard;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CardPage {
    pub cards: Vec<MemoryCard>,

    /// 服务端当前最大的 seq。分页时不要用 `latest` 当下一次的 `since`，要用本页最后一张卡的 `seq`；`latest` 只表示服务端当前最大的 seq。
    pub latest: i64,
}
