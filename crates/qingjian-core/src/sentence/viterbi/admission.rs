//! 整句格子的候补词能不能进词图：读音占比够高，并且在某个前驱后面比首段里最好的词还可能。

use crate::sentence::{LanguageModel, MIN_READING_SHARE, SpanWord, fallback_log_prob};

/// 某个前驱后面，首段里最可能的词的静态 log 概率。`previous` 为 `None` 是句首。
pub(super) fn head_ceiling(
    head: &[SpanWord],
    previous: Option<&str>,
    model: &dyn LanguageModel,
    log_total: f64,
) -> f64 {
    head.iter()
        .map(|w| {
            model
                .log_prob(previous, &w.text)
                .unwrap_or_else(|| fallback_log_prob(w.frequency, log_total))
        })
        .fold(f64::NEG_INFINITY, f64::max)
}

/// 候补词 `word` 能进词图：读音占比不低于 [`MIN_READING_SHARE`]，并且至少一个前驱后面它的静态 log 概率严格高于首段的最高值。
/// `previous_and_ceiling` 是每个前驱（`None` 为句首）与它对应的 [`head_ceiling`]。
/// 只看静态模型：个人常用的词已经靠 `seen` 进了首段。
pub(super) fn admits(
    word: &SpanWord,
    previous_and_ceiling: &[(Option<&str>, f64)],
    model: &dyn LanguageModel,
    log_total: f64,
) -> bool {
    if word.reading_share < MIN_READING_SHARE {
        return false;
    }
    let fallback = fallback_log_prob(word.frequency, log_total);
    previous_and_ceiling.iter().any(|&(previous, ceiling)| {
        model.log_prob(previous, &word.text).unwrap_or(fallback) > ceiling
    })
}
