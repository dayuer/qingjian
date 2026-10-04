//! `POST /v1/pair/join` 的请求体（不要鉴权）：新设备拿匹配码申请加入，等旧设备允许。

use serde::{Deserialize, Serialize};

use crate::Device;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PairJoin {
    /// 用户输入的匹配码，服务端会再规范化一遍。
    pub code: String,

    pub device: Device,
}
