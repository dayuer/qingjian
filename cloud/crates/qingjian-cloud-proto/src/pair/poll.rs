//! `GET /v1/pair/join/{request_id}` 的响应：新设备轮询加入申请的结果。
//!
//! 按 `state` 区分；`approved` 时 `SessionGrant` 的字段平铺在同一个对象里。服务端只在允许后第一次取时签会话，再取就是 404。

use serde::{Deserialize, Serialize};

use crate::SessionGrant;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum PairPoll {
    /// 旧设备还没处理。
    Pending,

    Denied,

    Approved(SessionGrant),
}
