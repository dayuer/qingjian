//! 服务器上一条学习数据的当前值（合并了所有设备）。`GET /v1/learning` 返回这些。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LearningRow {
    /// 表名：`user` / `choices` / `ngram` / `typos` / `english`（计数表）或 `words`（集合表）。
    pub table: String,

    /// 键；多列的键用 `\t` 连起来，与输入法的文件格式一致。
    pub key: String,

    /// 计数表的合计次数（不会小于 0）；集合表为 0。
    #[serde(default)]
    pub count: i64,

    /// 集合表的值（用户词的拼音）；计数表 `english` 放第一次的写法。
    #[serde(default)]
    pub value: Option<String>,

    /// 集合表里被删掉的（墓碑）。
    #[serde(default)]
    pub deleted: bool,

    /// 最后一次改动的序号。
    pub seq: u64,
}
