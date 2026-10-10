//! 连打短语：分几段打、每段整段选完一个词（`shufu` 选 舒服、`zhe` 选 着、`ne` 选 呢），合起来就是用户想要的短语。
//! 两个词的合并归 `try_auto_word`；这里管三个以上、合起来不超过四字的，记成「整串拼音 → 短语」的选择，
//! 选够 [`PHRASE_RUN_THRESHOLD`] 次、词库里没有就造成用户词，下次整串连着打它排第一。
//! 只靠个人二元帮不上：整句转换只走最优切分（`shufuzhene` 是 zhen'e），着 → 呢 根本进不了词图。

use std::time::Instant;

use crate::candidate::{Candidate, CandidateKind};
use crate::engine::learning::Learner;
use crate::engine::{AUTO_WORD_MAX_CHARS, Engine, PHRASE_RUN_THRESHOLD};

impl Engine {
    /// 一个词整段选完上屏了（`whole` 为真：这段拼音只出了这一个词、按全拼打、没纠错）：接上连打短语，
    /// 末尾三个以上的词够条件就记一次选择。返回记下的 `(学习键, 短语, 这次是否造了词)`，撤销时退回；不够条件返回 `None`。
    pub(super) fn extend_run(
        &mut self,
        text: &str,
        syllables: &[String],
        whole: bool,
    ) -> Option<(String, String, bool)> {
        if !whole {
            self.chain.break_run();
            return None;
        }
        self.chain.extend_run(text, syllables, Instant::now());
        let run = self.chain.run();
        // 从最长的后缀往短里找第一个够条件的：字数 = 音节数、不超过四字、至少三个词
        let (phrase, phrase_syllables) = (3..=run.len()).rev().find_map(|count| {
            let words = &run[run.len() - count..];
            let phrase: String = words.iter().map(|(t, _)| t.as_str()).collect();
            let syllables: Vec<String> =
                words.iter().flat_map(|(_, s)| s.iter().cloned()).collect();
            let chars = phrase.chars().count();
            (chars <= AUTO_WORD_MAX_CHARS && chars == syllables.len())
                .then_some((phrase, syllables))
        })?;
        let key = phrase_syllables.concat();
        self.learner.record_choice(&key, &phrase);
        let candidate = Candidate {
            text: phrase,
            kind: CandidateKind::Chinese,
            syllables: phrase_syllables,
            reading: None,
            translation: None,
            aux_code: None,
        };
        let learned = !self.knows_word(&candidate)
            && self.learner.choice_weight(&key, &candidate.text) >= PHRASE_RUN_THRESHOLD;
        if learned {
            tracing::debug!(text = %candidate.text, "连打短语，自动造词");
            self.learner
                .learn_word(&candidate.text, &candidate.syllables);
        }
        Some((key, candidate.text, learned))
    }
}
