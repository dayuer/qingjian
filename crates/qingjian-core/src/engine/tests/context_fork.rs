//! 宿主前文进词级排序与整句首词（素笺分叉，见 cloud/docs/specs/2026-10-04-context-prediction-design.md、docs/notes/context-eval.md）。

use std::collections::HashMap;

use qingjian_dictionary::Dictionary;

use super::CountingLearner;
use crate::candidate::CandidateKind;
use crate::engine::Engine;
use crate::engine::query::left_context::{LeftContext, left_context_of};
use crate::sentence::{Context, LanguageModel, NoLanguageModel};

/// 汽车 → 油箱、发送 → 邮箱；句首 邮箱 比 油箱 略常见。
struct ContextModel;

impl LanguageModel for ContextModel {
    fn log_prob(&self, previous: Option<&str>, word: &str) -> Option<f64> {
        Some(match (previous, word) {
            (Some("汽车"), "油箱") => -2.0,
            (Some("汽车"), "邮箱") => -9.0,
            (Some("发送"), "邮箱") => -2.0,
            (Some("发送"), "油箱") => -9.0,
            // 今天 → 油箱 分数高但二元表里没有（退回一元算出来的那种）
            (Some("今天"), "油箱") => -2.0,
            (Some("汽车"), "罚") => -1.0,
            (Some("罚"), "送") => -0.5,
            (_, "邮箱") => -5.0,
            (_, "油箱") => -5.4,
            (_, "汽车") | (_, "发送") | (_, "今天") => -5.0,
            (_, "送") | (_, "发") | (_, "罚") => -8.0,
            _ => return None,
        })
    }

    fn knows_pair(&self, previous: &str, word: &str) -> bool {
        matches!(
            (previous, word),
            ("汽车", "油箱" | "邮箱" | "罚") | ("发送", "邮箱" | "油箱") | ("罚", "送")
        )
    }
}

const WORDS: &str = "罚\tfa\t20000\n邮箱\tyou xiang\t9000\n油箱\tyou xiang\t3000\n汽车\tqi che\t9000\n发送\tfa song\t9000\n发\tfa\t20000\n送\tsong\t20000\n今天\tjin tian\t9000\n";

fn context_engine() -> Engine {
    Engine::new(Dictionary::parse(WORDS).unwrap()).with_language_model(Box::new(ContextModel))
}

fn first(engine: &mut Engine, input: &str) -> String {
    engine.set_input(input);
    let text = engine.query().unwrap().candidates.items[0].text.clone();
    engine.clear();
    text
}

#[test]
fn left_context_takes_the_last_two_words_of_a_han_tail() {
    let model = ContextModel;
    assert_eq!(
        left_context_of("今天汽车", &model).context(),
        Context::after_two("今天", "汽车")
    );
    assert_eq!(
        left_context_of("汽车", &model).context(),
        Context::after("汽车")
    );
    // 标点、空格、字母结尾都是句首
    assert_eq!(left_context_of("汽车。", &model), LeftContext::default());
    assert_eq!(left_context_of("汽车 ", &model), LeftContext::default());
    assert_eq!(left_context_of("汽车abc", &model), LeftContext::default());
    assert_eq!(left_context_of("", &model), LeftContext::default());
    // 只看末尾 8 个字：前面再长也不影响
    let long = format!("{}今天汽车", "龘".repeat(20));
    assert_eq!(
        left_context_of(&long, &model).context(),
        Context::after_two("今天", "汽车")
    );
    // 模型一个词都不认识：句首
    assert_eq!(
        left_context_of("汽车", &NoLanguageModel),
        LeftContext::default()
    );
}

#[test]
fn host_context_reorders_word_candidates_at_sentence_start() {
    let mut engine = context_engine();
    assert_eq!(first(&mut engine, "youxiang"), "邮箱");
    engine.history_mut().record("汽车");
    assert_eq!(first(&mut engine, "youxiang"), "油箱");
    // 壳给的前文压过本会话历史（`clear()` 会清掉壳给的前文，所以先喂拼音再给前文，与壳第一键的顺序一致）
    engine.set_input("youxiang");
    engine.set_rescoring_context(Some("请发送".to_owned()));
    assert_eq!(engine.query().unwrap().candidates.items[0].text, "邮箱");
    engine.clear();
}

#[test]
fn punctuation_at_the_end_of_the_context_means_sentence_start() {
    let mut engine = context_engine();
    engine.history_mut().record("汽车，");
    assert_eq!(first(&mut engine, "youxiang"), "邮箱");
}

#[test]
fn private_input_ignores_the_context() {
    let mut engine = context_engine();
    engine.history_mut().record("汽车");
    engine.set_private(true);
    assert_eq!(first(&mut engine, "youxiang"), "邮箱");
    engine.set_private(false);
    engine.history_mut().record("汽车");
    assert_eq!(first(&mut engine, "youxiang"), "油箱");
}

#[test]
fn the_commit_chain_wins_over_host_context() {
    let mut engine = context_engine();
    engine.set_input("fasong");
    let candidate = engine.query().unwrap().candidates.items[0].clone();
    assert_eq!(candidate.text, "发送");
    engine.commit(&candidate);
    // 上屏把 发送 写进了历史；换成别的前文，链上仍是 发送
    engine.history_mut().clear();
    engine.history_mut().record("汽车");
    assert_eq!(first(&mut engine, "youxiang"), "邮箱");
    // 链断了就看前文
    engine.break_chain();
    engine.history_mut().clear();
    engine.history_mut().record("汽车");
    assert_eq!(first(&mut engine, "youxiang"), "油箱");
}

#[test]
fn a_multi_word_sentence_follows_the_context() {
    // 实验（exp/sentence-left-context-a3）：不看上文时最优本来就是多词整句，首词才接前文；见 docs/notes/context-eval.md
    let mut engine = context_engine();
    let sentence_of = |engine: &mut Engine| {
        engine.set_input("youxiangfasong");
        let text = engine
            .query()
            .unwrap()
            .candidates
            .items
            .iter()
            .find(|c| c.kind == CandidateKind::Sentence)
            .expect("有整句候选")
            .text
            .clone();
        engine.clear();
        text
    };
    assert_eq!(sentence_of(&mut engine), "邮箱发送");
    engine.history_mut().record("汽车");
    assert_eq!(sentence_of(&mut engine), "油箱发送");
}

/// 整句：不看上文时最优是 `words`；接上文（`history`）后的最优是什么。
fn sentence_after(history: &str, input: &str) -> String {
    let mut engine = context_engine();
    engine.history_mut().record(history);
    engine.set_input(input);
    engine
        .query()
        .unwrap()
        .candidates
        .items
        .iter()
        .find(|c| c.kind == CandidateKind::Sentence)
        .expect("有整句候选")
        .text
        .clone()
}

#[test]
fn an_unseen_pair_does_not_pull_the_sentence() {
    // 今天 → 油箱 分数高但语料没见过：首词仍按句首算
    assert_eq!(sentence_after("今天", "youxiangfasong"), "邮箱发送");
}

#[test]
fn the_context_never_shortens_the_first_word() {
    // 汽车 → 罚 → 送 都见过且分数高，但会把首词 发送 拆成单字 罚：不接
    assert_eq!(sentence_after("汽车", "fasongyouxiang"), "发送邮箱");
}

fn learning_engine() -> Engine {
    context_engine().with_learner(Box::new(CountingLearner(HashMap::new())))
}

#[test]
fn a_single_past_choice_yields_to_strong_context() {
    let mut engine = learning_engine();
    engine.learner_mut().record_choice("youxiang", "邮箱");
    engine.history_mut().record("汽车");
    // 选过一次 邮箱，但 汽车 后面 油箱 领先 7 nat
    assert_eq!(first(&mut engine, "youxiang"), "油箱");
}

#[test]
fn a_single_past_choice_still_wins_under_weak_context() {
    let mut engine = learning_engine();
    engine.learner_mut().record_choice("youxiang", "油箱");
    // 句首 邮箱 只领先 0.4 nat，小于 β·ln2：选过一次的 油箱 第一
    assert_eq!(first(&mut engine, "youxiang"), "油箱");
}

#[test]
fn many_past_choices_still_beat_weak_context() {
    let mut engine = learning_engine();
    for _ in 0..5 {
        engine.learner_mut().record_choice("youxiang", "油箱");
    }
    assert_eq!(first(&mut engine, "youxiang"), "油箱");
}
