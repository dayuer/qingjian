//! 对象层用户词的快照：引擎查词要 `&Dictionary` / `&WordList`，对象层却锁在 `Mutex` 里借不出引用，
//! 所以 [`super::ScopedLearner`] 自己存一份副本，只在换人（`scope_changed`）、造词、删词之后重建，不随按键重建。
//! 快照记着建它时叠加层的代数，代数对不上（换了人还没重建）时一律当没有，宁可少出词也不串到别人那里。

use qingjian_dictionary::{Dictionary, WordList};
use qingjian_learning::FrequencyLearner;

use qingjian_core::Learner;

use super::Overlay;

#[derive(Debug, Default)]
pub struct Snapshot {
    words: Option<Dictionary>,

    english: Option<WordList>,

    /// 建快照时叠加层的代数（[`Overlay::generation`]）。
    generation: u64,
}

impl Snapshot {
    /// 按当前叠加层重建；没选人时两份都空。
    pub fn of(overlay: &Overlay) -> Self {
        let layer = overlay.contact();
        Self {
            words: layer.and_then(copy_words),
            english: layer.and_then(copy_english),
            generation: overlay.generation(),
        }
    }

    /// 快照对得上叠加层时才给。
    pub fn words(&self, generation: u64) -> Option<&Dictionary> {
        self.words
            .as_ref()
            .filter(|_| self.generation == generation)
    }

    pub fn english(&self, generation: u64) -> Option<&WordList> {
        self.english
            .as_ref()
            .filter(|_| self.generation == generation)
    }
}

/// `Dictionary` 不能克隆，按它自己的 TSV 格式（`词\t拼音\t词频`）抄一份。
fn copy_words(layer: &FrequencyLearner) -> Option<Dictionary> {
    let source = layer.user_words()?;
    let tsv: String = source
        .entries()
        .map(|entry| format!("{}\t{}\t{}\n", entry.text, entry.pinyin, entry.frequency))
        .collect();
    Dictionary::parse(&tsv)
        .inspect_err(|error| tracing::warn!(%error, "对象层用户词快照重建失败"))
        .ok()
}

/// 同上，`WordList` 的格式是 `词\t编码\t词频`。
fn copy_english(layer: &FrequencyLearner) -> Option<WordList> {
    let source = layer.user_english()?;
    let tsv: String = source
        .entries()
        .map(|(code, word, frequency)| format!("{word}\t{code}\t{frequency}\n"))
        .collect();
    WordList::parse(&tsv)
        .inspect_err(|error| tracing::warn!(%error, "对象层英文词快照重建失败"))
        .ok()
}
