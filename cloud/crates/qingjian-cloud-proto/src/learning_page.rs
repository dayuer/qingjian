//! `GET /v1/learning?since=&limit=` 的响应，以及推送后的回执。

use serde::{Deserialize, Serialize};

use crate::LearningRow;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LearningPage {
    /// 按 `seq` 升序。
    pub rows: Vec<LearningRow>,

    /// 学习数据当前最新的 `seq`。
    pub latest: u64,
}
