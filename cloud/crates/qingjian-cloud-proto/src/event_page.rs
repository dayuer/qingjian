//! `GET /v1/events` 的响应。

use serde::{Deserialize, Serialize};

use crate::Event;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventPage {
    /// 按 `seq` 升序。
    pub events: Vec<Event>,

    /// 服务端当前最新的 `seq`；`events` 最后一条小于它说明还有下一页。
    pub latest: u64,
}
