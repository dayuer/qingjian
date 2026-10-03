//! 输入日志的上传与拉取。行是输入法 `input-log.jsonl` 的原文，服务器不改。

use serde::{Deserialize, Serialize};

/// `POST /v1/input-log` 的请求体。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InputLogPush {
    /// 这一批的唯一标识（文件标识 + 起始偏移）；重发同一批只记一次。
    pub batch_id: String,

    pub lines: Vec<String>,
}

/// 拉取到的一行。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InputLogLine {
    pub seq: u64,

    pub device: String,

    pub line: String,
}

/// `GET /v1/input-log?since=&limit=` 的响应；`POST` 的回执也用它（`lines` 为空）。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct InputLogPage {
    pub lines: Vec<InputLogLine>,

    pub latest: u64,

    /// 清空过几次；变了说明之前拉到的都已作废。
    pub generation: u64,
}
