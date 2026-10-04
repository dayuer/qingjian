//! 知微词级重排的测试：假打分器，不依赖模型文件。

use std::time::{Duration, Instant};

use qingjian_dictionary::Dictionary;

use crate::candidate::CandidateKind;
use crate::engine::Engine;
use crate::sentence::{LanguageModel, SentenceScorer};

/// 假知微：偏爱某个文本。
struct Prefers(&'static str);

impl SentenceScorer for Prefers {
    fn score(&self, _context: &str, _keys: &str, texts: &[&str]) -> Vec<f64> {
        texts
            .iter()
            .map(|t| if *t == self.0 { -1.0 } else { -20.0 })
            .collect()
    }
}

/// 假知微：算不了。
struct Broken;

impl SentenceScorer for Broken {
    fn score(&self, _context: &str, _keys: &str, _texts: &[&str]) -> Vec<f64> {
        Vec::new()
    }
}

/// 句首强烈偏向 又 + 想：`youxiang` 的整句首选是 又想。
struct LikesYouXiang;

impl LanguageModel for LikesYouXiang {
    fn log_prob(&self, previous: Option<&str>, word: &str) -> Option<f64> {
        Some(match (previous, word) {
            (None, "又") => -1.0,
            (Some("又"), "想") => -1.0,
            (_, "邮箱") | (_, "油箱") => -8.0,
            (_, "邮") | (_, "又") | (_, "想") => -6.0,
            _ => return None,
        })
    }
}

const WORDS: &str = "邮箱\tyou xiang\t9000\n油箱\tyou xiang\t3000\n邮\tyou\t900\n又\tyou\t800\n想\txiang\t900\n开发\tkai fa\t9000\n开\tkai\t20000\n";

fn engine() -> Engine {
    Engine::new(Dictionary::parse(WORDS).unwrap())
}

fn all(engine: &Engine) -> Vec<(CandidateKind, String)> {
    engine
        .query()
        .unwrap()
        .candidates
        .items
        .into_iter()
        .map(|c| (c.kind, c.text))
        .collect()
}

fn words(engine: &Engine) -> Vec<String> {
    all(engine)
        .into_iter()
        .filter(|(kind, _)| *kind == CandidateKind::Chinese)
        .map(|(_, text)| text)
        .collect()
}

#[test]
fn sync_word_scorer_reorders_the_top_tier() {
    let mut engine = engine().with_word_scorer(Box::new(Prefers("油箱")), Some(1.0));
    engine.set_input("youxiang");
    assert_eq!(words(&engine)[..2], ["油箱", "邮箱"]);
    assert!(!engine.rescoring_pending());
}

#[test]
fn only_candidates_in_the_first_tier_move() {
    // 开 是前缀词（coverage 小），不与 开发 同档：打分器再偏爱它也翻不上来
    let mut engine = engine().with_word_scorer(Box::new(Prefers("开")), Some(1.0));
    engine.set_input("kaifa");
    assert_eq!(words(&engine)[0], "开发");
}

#[test]
fn the_sentence_candidate_is_never_moved() {
    // 句首强偏向 又想：整句 又想 排第一；打分器偏爱前缀词 邮，整句与 邮箱 都不该被压到 邮 下面
    let mut engine = engine()
        .with_language_model(Box::new(LikesYouXiang))
        .with_word_scorer(Box::new(Prefers("邮")), Some(1.0));
    engine.set_input("youxiang");
    let items = all(&engine);
    assert_eq!(items[0], (CandidateKind::Sentence, "又想".to_owned()));
    let position = |text: &str| items.iter().position(|(_, t)| t == text).unwrap();
    assert!(position("邮箱") < position("邮"));
}

#[test]
fn a_failing_scorer_keeps_the_static_order() {
    let mut engine = engine().with_word_scorer(Box::new(Broken), Some(1.0));
    engine.set_input("youxiang");
    assert_eq!(words(&engine)[..2], ["邮箱", "油箱"]);
}

#[test]
fn async_word_scorer_waits_for_request_and_poll() {
    let mut engine = engine().with_async_word_scorer(Box::new(Prefers("油箱")), Some(1.0));
    engine.set_input("youxiang");
    assert_eq!(words(&engine)[..2], ["邮箱", "油箱"]);
    assert!(engine.rescoring_pending());
    assert!(engine.request_rescoring());
    let started = Instant::now();
    while !engine.poll_rescoring() {
        assert!(started.elapsed() < Duration::from_secs(5), "后台没回结果");
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(words(&engine)[..2], ["油箱", "邮箱"]);
    assert!(!engine.rescoring_pending());
}

#[test]
fn without_a_word_scorer_nothing_changes() {
    let mut engine = engine();
    engine.set_input("youxiang");
    assert_eq!(words(&engine)[..2], ["邮箱", "油箱"]);
    assert!(!engine.has_word_scorer());
}

/// 慢通变：每次打分睡 60 ms，让知微的结果先到。
struct SlowSentence;

impl SentenceScorer for SlowSentence {
    fn score(&self, _context: &str, _keys: &str, texts: &[&str]) -> Vec<f64> {
        std::thread::sleep(Duration::from_millis(60));
        texts
            .iter()
            .map(|t| if *t == "开放" { -1.0 } else { -20.0 })
            .collect()
    }
}

#[test]
fn rescoring_stays_in_flight_until_both_models_have_answered() {
    // 通变慢、知微快：知微的结果先到，壳/CLI 若在第一次收到结果就停轮询，通变的结果就丢了
    let mut engine = engine()
        .with_async_sentence_scorer(Box::new(SlowSentence), Some(1.0), None, None)
        .with_async_word_scorer(Box::new(Prefers("油箱")), Some(1.0));
    engine.set_input("kaifang");
    engine.query().unwrap();
    engine.set_input("youxiang");
    engine.query().unwrap();
    assert!(!engine.rescoring_in_flight());
    assert!(engine.request_rescoring());
    assert!(engine.rescoring_in_flight());
    let started = Instant::now();
    let mut rounds = 0;
    while engine.rescoring_in_flight() {
        assert!(started.elapsed() < Duration::from_secs(5), "后台没回结果");
        if engine.poll_rescoring() {
            rounds += 1;
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    assert!(rounds >= 1);
    assert!(!engine.rescoring_in_flight());
    assert_eq!(words(&engine)[..2], ["油箱", "邮箱"]);
}

#[test]
fn nothing_is_in_flight_without_models() {
    let mut engine = engine();
    engine.set_input("youxiang");
    engine.query().unwrap();
    assert!(!engine.request_rescoring());
    assert!(!engine.rescoring_in_flight());
}
