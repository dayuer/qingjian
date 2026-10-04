//! 提示是怎么来的：打字碰上了卡片里的词，或日子与约定快到了。

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum HintReason {
    Match,

    Today,
}
