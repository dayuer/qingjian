//! 设置页里可选的拼音方案：全拼与七套双拼（注音与「只用形码」iOS 键盘不支持，不列）。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchemeOption {
    /// 写进 `[general] scheme` 的值。
    pub key: String,

    pub label: String,
}
