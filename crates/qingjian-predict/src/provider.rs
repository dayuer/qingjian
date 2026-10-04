//! 云联想走哪家服务：青简 Cloud（地址与令牌由平台层从 Cloud 的配置里拿，`[predict]` 里不用填）
//! 或自定义的 OpenAI 兼容接口（`base_url` / `model` / `api_key` 照填）。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PredictProvider {
    #[default]
    Qingjian,

    Custom,
}
