//! 计数表的一个值。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CountEntry {
    pub count: i64,

    /// 个人英文词的写法（键是小写）；别的表为 `None`。
    pub display: Option<String>,
}
