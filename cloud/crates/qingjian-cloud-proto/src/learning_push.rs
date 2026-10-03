//! `POST /v1/learning` 的请求体：本机自上次同步以来的变化。

use serde::{Deserialize, Serialize};

/// 一次最多推多少条，客户端按这个分批。
pub const MAX_LEARNING_PUSH: usize = 2000;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LearningPush {
    /// 计数表的增量：`(表, 键, 带符号的增量, 写法)`。
    #[serde(default)]
    pub counts: Vec<CountDelta>,

    /// 集合表的新增或修改。
    #[serde(default)]
    pub puts: Vec<SetPut>,

    /// 集合表的删除。
    #[serde(default)]
    pub deletes: Vec<SetDelete>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CountDelta {
    pub table: String,

    pub key: String,

    pub delta: i64,

    /// 可选的显示写法（个人英文词的大小写）。
    #[serde(default)]
    pub value: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetPut {
    pub table: String,

    pub key: String,

    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetDelete {
    pub table: String,

    pub key: String,
}

impl LearningPush {
    pub fn len(&self) -> usize {
        self.counts.len() + self.puts.len() + self.deletes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
