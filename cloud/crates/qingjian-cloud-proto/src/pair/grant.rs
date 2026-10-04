//! `POST /v1/pair/join` 的响应：新设备凭它轮询 `GET /v1/pair/join/{request_id}` 取令牌。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PairJoinGrant {
    pub request_id: String,

    /// 轮询时放在 `X-Pair-Secret` 请求头里，不放查询串（查询串会进访问日志）。
    pub secret: String,

    /// 这次申请的过期时间，Unix 毫秒。
    pub expires_at: i64,
}
