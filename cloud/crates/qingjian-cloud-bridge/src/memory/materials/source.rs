//! 素材从哪来：剪贴板确认条，或键盘里手写。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MaterialSource {
    Clipboard,

    #[default]
    Typed,
}

impl MaterialSource {
    /// C 接口的参数：`clipboard` / `typed`，认不得按手写。
    pub fn parse(text: &str) -> Self {
        match text {
            "clipboard" => Self::Clipboard,
            _ => Self::Typed,
        }
    }
}
