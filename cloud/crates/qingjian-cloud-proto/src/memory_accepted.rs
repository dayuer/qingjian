//! `POST /v1/memory/materials` 的响应：服务端收下了几条（重复的不算）与跳过了几条。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryAccepted {
    pub accepted: u32,

    /// 服务端跳过的条数：引用了未登记或已删除对象的，以及 `at` 早于 2020-01-01 或晚于当前时间加一天的；不再整批 400。
    #[serde(default)]
    pub skipped: u32,
}
