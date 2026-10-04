//! `GET /v1/memory/processor` 的响应：同意页显示的大模型供应商。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessorInfo {
    /// 供应商名称，来自服务端配置；为空时客户端不允许打开记录。
    pub name: String,

    /// 是否零留存，由服务端按配置给出；缺省 false。
    #[serde(default)]
    pub zero_retention: bool,
}
