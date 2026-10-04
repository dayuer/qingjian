//! 提醒文案里怎么称呼对象。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Pronoun {
    #[default]
    Ta,

    /// 他。
    TaM,

    /// 她。
    TaF,

    /// 直接用名字。
    Name,
}

impl Pronoun {
    /// 文案里的称呼；`Name` 时用 `name`。
    pub fn label(self, name: &str) -> String {
        match self {
            Self::Ta => "TA".to_owned(),
            Self::TaM => "他".to_owned(),
            Self::TaF => "她".to_owned(),
            Self::Name => name.to_owned(),
        }
    }
}
