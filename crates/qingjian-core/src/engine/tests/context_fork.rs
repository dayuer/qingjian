//! 宿主前文进词级排序（整句首词不用）（素笺分叉，见 cloud/docs/specs/2026-10-04-context-prediction-design.md）。

use qingjian_dictionary::Dictionary;

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
            (_, "邮箱") => -5.0,
            (_, "油箱") => -5.4,
            (_, "汽车") | (_, "发送") | (_, "今天") => -5.0,
            (_, "送") | (_, "发") => -8.0,
            _ => return None,
        })
    }
}

const WORDS: &str = "邮箱\tyou xiang\t9000\n油箱\tyou xiang\t3000\n汽车\tqi che\t9000\n发送\tfa song\t9000\n发\tfa\t20000\n送\tsong\t20000\n今天\tjin tian\t9000\n";

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
fn the_sentence_first_word_ignores_the_context() {
    // 整句首词仍按句首算：前文只给词级排序。让首词也看前文会让 --replay 整句少 3 句（见计划文件 Task 2）
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
    assert_eq!(sentence_of(&mut engine), "邮箱发送");
}
