//! 要用户单独同意才开的云功能；服务端没开的功能，对应接口返回 403。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Feature {
    /// 跨设备剪贴板。
    Clipboard,

    /// 学习数据与 `config.toml` 同步。
    Sync,

    /// 上传输入日志。
    InputLog,

    /// 大模型代理（云联想、润色）。
    Llm,

    /// 素材上传（2B）与记忆卡下发。
    Memory,
}

impl Feature {
    pub const ALL: [Feature; 5] = [
        Feature::Clipboard,
        Feature::Sync,
        Feature::InputLog,
        Feature::Llm,
        Feature::Memory,
    ];

    /// 路径与 JSON 里的名字（`PUT /v1/consents/{feature}`）。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Clipboard => "clipboard",
            Self::Sync => "sync",
            Self::InputLog => "input_log",
            Self::Llm => "llm",
            Self::Memory => "memory",
        }
    }

    pub fn parse(text: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|f| f.as_str() == text)
    }
}
