//! 记忆卡片的来源。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CardSource {
    /// 云端每日整理从素材里学到的，要用户确认。
    Cloud,

    /// 用户手动记的。
    Manual,
}
