//! 词级排序与整句首词的上文：会话链上没有词（句首）时，从宿主光标前的文字末尾切出最后一两个词。
//!
//! 只看末尾 [`LEFT_CONTEXT_CHARS`] 个连续汉字，用静态语言模型最大匹配切词（[`sentence::segment_text`]）；
//! 末尾不是汉字（标点、空格、字母）就当句首，与上屏时标点打断链的规则一致。私密输入时不看前文。
//! 素笺分叉，见 cloud/docs/specs/2026-10-04-context-prediction-design.md。

use crate::engine::Engine;
use crate::sentence::{self, Context, LanguageModel, is_han};

/// 前文末尾最多看几个汉字。
pub(in crate::engine) const LEFT_CONTEXT_CHARS: usize = 8;

/// 拥有文本的上文，借出 [`Context`]。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(in crate::engine) struct LeftContext {
    /// 前一个词；`None` 是句首。
    previous: Option<String>,

    /// 再前一个词。
    earlier: Option<String>,
}

impl LeftContext {
    pub fn context(&self) -> Context<'_> {
        Context {
            previous: self.previous.as_deref(),
            earlier: self.earlier.as_deref(),
        }
    }
}

/// 从一段文字的末尾切出上文。
pub(in crate::engine) fn left_context_of(text: &str, model: &dyn LanguageModel) -> LeftContext {
    let mut tail: Vec<char> = text
        .chars()
        .rev()
        .take_while(|c| is_han(*c))
        .take(LEFT_CONTEXT_CHARS)
        .collect();
    if tail.is_empty() {
        return LeftContext::default();
    }
    tail.reverse();
    let tail: String = tail.into_iter().collect();
    let Some(mut clauses) = sentence::segment_text(&tail, model) else {
        return LeftContext::default();
    };
    let Some(mut words) = clauses.pop() else {
        return LeftContext::default();
    };
    let previous = words.pop();
    let earlier = words.pop();
    LeftContext { previous, earlier }
}

impl Engine {
    /// 下一个词的上文：链上有词就用链（本会话刚上屏的词最可信），否则从宿主前文末尾切
    /// （壳没给前文时 [`Self::rescoring_context`] 退回本会话历史）。私密输入时只认链，不看前文。
    pub(in crate::engine) fn word_context(&self) -> LeftContext {
        let chain = self.chain.context();
        if let Some(previous) = chain.previous {
            return LeftContext {
                previous: Some(previous.to_owned()),
                earlier: chain.earlier.map(str::to_owned),
            };
        }
        if self.is_private() {
            return LeftContext::default();
        }
        left_context_of(&self.rescoring_context(), &*self.language_model)
    }
}
