//! 素材所属的场景。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Scene {
    /// 日常。
    Daily,

    /// 恋爱。
    Dating,

    /// 工作，缺省不记。
    Work,
}
