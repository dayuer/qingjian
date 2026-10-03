//! 落盘的学习数据同步进度。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LearningState {
    /// 已经并进基线的服务器 `seq`。
    pub cursor: u64,

    /// 已写出、等输入法合并的收件箱：合并后把它并进基线，`cursor` 前进到 `pending_cursor`。
    pub pending: Option<String>,

    pub pending_cursor: u64,
}
