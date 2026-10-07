//! 对话 / 书面两份语言模型按左侧上文的语域插值：P = w·P对话 + (1 − w)·P书面。
//!
//! 两份模型各自在本语域里都比单份混合语料的好、出了语域就塌（对话模型在书面留出集上 21%）；
//! 固定一个 w 两头顾不到，所以 w 跟着上文走：上文末尾 [`CONTEXT_CHARS`] 个汉字在两份模型下的平均一元对数似然比
//! 过了 [`THRESHOLD`] 就当对话、否则当书面，没有上文用中间值。参数在 1c 的对话 / 书面开发集上定，
//! 留出集与外部集只做验证（2026-10-08，见 docs/notes/old-lexicon-analysis.md）。
//! 词表是两份模型各自的；只在一份里的词按另一份概率 0 算。

use std::sync::atomic::{AtomicU64, Ordering};

use super::{LanguageModel, is_han};

/// 判语域看上文末尾几个汉字。
const CONTEXT_CHARS: usize = 20;

/// 平均每字 log P对话 − log P书面 超过它就当对话。
const THRESHOLD: f64 = -0.4;

/// 上文像对话 / 像书面 / 没有上文时对话模型的权重。
const DIALOG_WEIGHT: f64 = 0.8;
const WRITTEN_WEIGHT: f64 = 0.2;
const UNKNOWN_WEIGHT: f64 = 0.4;

pub struct RegisterMix {
    dialog: Box<dyn LanguageModel>,

    written: Box<dyn LanguageModel>,

    /// 当前对话模型的权重（`f64` 的位），[`LanguageModel::observe_context`] 改它。
    weight: AtomicU64,
}

impl RegisterMix {
    pub fn new(dialog: Box<dyn LanguageModel>, written: Box<dyn LanguageModel>) -> Self {
        Self {
            dialog,
            written,
            weight: AtomicU64::new(UNKNOWN_WEIGHT.to_bits()),
        }
    }

    /// 当前对话模型的权重。
    pub fn weight(&self) -> f64 {
        f64::from_bits(self.weight.load(Ordering::Relaxed))
    }

    /// 上文的语域 → 对话模型的权重。上文里两份模型都认识的汉字一个都没有就是「不知道」。
    fn weight_for(&self, context: &str) -> f64 {
        let mut tail: Vec<char> = context.chars().rev().filter(|c| is_han(*c)).collect();
        tail.truncate(CONTEXT_CHARS);
        let mut buffer = [0u8; 4];
        let ratios: Vec<f64> = tail
            .iter()
            .filter_map(|c| {
                let c: &str = c.encode_utf8(&mut buffer);
                Some(self.dialog.unigram_log_prob(c)? - self.written.unigram_log_prob(c)?)
            })
            .collect();
        if ratios.is_empty() {
            return UNKNOWN_WEIGHT;
        }
        let mean = ratios.iter().sum::<f64>() / ratios.len() as f64;
        if mean > THRESHOLD {
            DIALOG_WEIGHT
        } else {
            WRITTEN_WEIGHT
        }
    }

    fn mix(&self, dialog: Option<f64>, written: Option<f64>) -> Option<f64> {
        if dialog.is_none() && written.is_none() {
            return None;
        }
        let w = self.weight();
        let p = w * dialog.map_or(0.0, f64::exp) + (1.0 - w) * written.map_or(0.0, f64::exp);
        Some(p.max(f64::MIN_POSITIVE).ln())
    }
}

impl LanguageModel for RegisterMix {
    fn log_prob(&self, previous: Option<&str>, word: &str) -> Option<f64> {
        self.mix(
            self.dialog.log_prob(previous, word),
            self.written.log_prob(previous, word),
        )
    }

    fn unigram_log_prob(&self, word: &str) -> Option<f64> {
        self.mix(
            self.dialog.unigram_log_prob(word),
            self.written.unigram_log_prob(word),
        )
    }

    fn observe_context(&self, context: &str) {
        self.weight
            .store(self.weight_for(context).to_bits(), Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests;
