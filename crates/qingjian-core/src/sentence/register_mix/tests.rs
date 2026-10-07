use std::collections::HashMap;

use super::*;

/// 假模型：一元按表给，二元一律等于一元。
struct Table(HashMap<&'static str, f64>);

impl LanguageModel for Table {
    fn log_prob(&self, _previous: Option<&str>, word: &str) -> Option<f64> {
        self.0.get(word).map(|p| p.ln())
    }

    fn unigram_log_prob(&self, word: &str) -> Option<f64> {
        self.log_prob(None, word)
    }
}

fn mix() -> RegisterMix {
    // 吗 对话里常见、书面少见；年 反过来；餐馆 只有对话模型认识
    let dialog = Table(HashMap::from([
        ("吗", 0.02),
        ("年", 0.001),
        ("餐馆", 0.004),
    ]));
    let written = Table(HashMap::from([("吗", 0.001), ("年", 0.02)]));
    RegisterMix::new(Box::new(dialog), Box::new(written))
}

#[test]
fn weight_follows_the_register_of_the_left_context() {
    let mix = mix();
    assert_eq!(mix.weight(), UNKNOWN_WEIGHT);
    mix.observe_context("好吗吗");
    assert_eq!(mix.weight(), DIALOG_WEIGHT);
    mix.observe_context("年年年");
    assert_eq!(mix.weight(), WRITTEN_WEIGHT);
    // 两份模型都不认识的字、标点与字母不算证据
    mix.observe_context("hello，");
    assert_eq!(mix.weight(), UNKNOWN_WEIGHT);
}

#[test]
fn probabilities_mix_linearly_and_a_missing_word_counts_as_zero() {
    let mix = mix();
    mix.observe_context("吗");
    let expected = DIALOG_WEIGHT * 0.02 + WRITTEN_WEIGHT * 0.001;
    assert!((mix.log_prob(None, "吗").unwrap() - expected.ln()).abs() < 1e-12);
    let only_dialog = DIALOG_WEIGHT * 0.004;
    assert!((mix.log_prob(None, "餐馆").unwrap() - only_dialog.ln()).abs() < 1e-12);
    assert_eq!(mix.log_prob(None, "不认识"), None);
}
