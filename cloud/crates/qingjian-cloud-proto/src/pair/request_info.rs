//! `GET /v1/pair/requests` 列表的一项：等旧设备处理的加入申请。

use serde::{Deserialize, Serialize};

use crate::Platform;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PairRequestInfo {
    /// 处理时接在 `/v1/pair/requests/` 后面。
    pub id: String,

    /// 新设备报的设备名。
    pub name: String,

    pub platform: Platform,

    /// 申请时间，Unix 毫秒。
    pub at: i64,
}
