//! 词文本 → 本词库里这个词全部读音的词频之和。整句词图按它算读音占比，挡住多音字的冷门读音。

use std::collections::HashMap;

use crate::matching::Match;

#[derive(Debug, Default)]
pub(super) struct TextTotals {
    totals: HashMap<Box<str>, u64>,
}

impl TextTotals {
    pub(super) fn build<'a>(entries: impl Iterator<Item = Match<'a>>) -> Self {
        let mut totals: HashMap<Box<str>, u64> = HashMap::new();
        for entry in entries {
            *totals.entry(entry.text.into()).or_insert(0) += u64::from(entry.frequency);
        }
        Self { totals }
    }

    pub(super) fn get(&self, text: &str) -> u64 {
        self.totals.get(text).copied().unwrap_or(0)
    }
}
