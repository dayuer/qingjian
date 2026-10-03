//! 候选栏里的一格：本地候选、云端补的词，或大模型给的整句补全。

use qingjian_core::Candidate;

pub enum Entry {
    Local(Candidate),

    /// 大模型补的词，带拼音，上屏走 `Engine::commit`。
    Cloud(Candidate),

    /// 整句补全，上屏走 `Engine::accept_prediction`（替换作用域内的整段拼音）。
    Sentence(String),
}

impl Entry {
    pub fn text(&self) -> &str {
        match self {
            Self::Local(candidate) | Self::Cloud(candidate) => &candidate.text,
            Self::Sentence(text) => text,
        }
    }

    pub fn is_cloud(&self) -> bool {
        !matches!(self, Self::Local(_))
    }
}
