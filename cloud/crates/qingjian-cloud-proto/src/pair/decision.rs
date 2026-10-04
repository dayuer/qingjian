//! `POST /v1/pair/requests/{id}` 的请求体：旧设备允许或拒绝一条加入申请。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PairDecision {
    pub allow: bool,
}
