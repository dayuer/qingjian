//! 记忆卡片的类型。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CardKind {
    /// 日期：生日、纪念日。
    Date,

    /// 约定：答应过的事。
    Promise,

    /// 偏好：喜欢与不喜欢。
    Preference,

    /// 近况：最近发生的事。
    Recent,

    /// 其它。
    Other,
}
