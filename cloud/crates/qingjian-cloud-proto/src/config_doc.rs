//! 同步的配置文件（输入法的 `config.toml`，含设置与自定义短语）。整份按「后写的赢」处理，冲突时客户端留备份。

use serde::{Deserialize, Serialize};

/// `GET /v1/config` 的响应。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigDoc {
    pub text: String,

    /// 每次写入加一，从 1 开始。
    pub version: u64,

    /// 最后写入的设备。
    pub device: String,

    /// 最后写入的时间，Unix 毫秒。
    pub at: i64,
}

/// `PUT /v1/config` 的请求体。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PutConfig {
    pub text: String,

    /// 期望服务器上当前的版本（0 表示还没有）；不一致返回 409，客户端重新比对。
    pub if_version: u64,
}
