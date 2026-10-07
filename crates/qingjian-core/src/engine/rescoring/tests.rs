use std::time::{Duration, Instant};

use super::*;
use crate::sentence::SentenceWord;

/// 这几个测试里路径共同解释的那段按键。
const KEYS: &str = "kaifang";

/// 假打分器：偏爱某个文本，其余都给低分。
struct Prefers(&'static str);

impl SentenceScorer for Prefers {
    fn score(&self, _context: &str, _keys: &str, texts: &[&str]) -> Vec<f64> {
        texts
            .iter()
            .map(|t| if *t == self.0 { -1.0 } else { -20.0 })
            .collect()
    }
}

fn path(text: &str, score: f64) -> Conversion {
    Conversion {
        text: text.to_owned(),
        syllables: Vec::new(),
        words: vec![SentenceWord {
            text: text.to_owned(),
            syllables: Vec::new(),
            placeholder: false,
        }],
        score,
        static_score: score,
        penalty: 0.0,
    }
}

fn engine() -> Engine {
    Engine::new(Dictionary::parse("开发\tkai fa\t9000\n").unwrap())
}

fn texts(paths: &[Conversion]) -> Vec<&str> {
    paths.iter().map(|p| p.text.as_str()).collect()
}

#[test]
fn sync_scorer_reorders_paths_in_place() {
    let engine = engine().with_sentence_scorer(Box::new(Prefers("开放")), Some(0.5), None, None);
    let mut paths = vec![path("开饭", -10.0), path("开放", -11.0)];
    engine.rescore_paths(&mut paths, KEYS);
    assert_eq!(texts(&paths), ["开放", "开饭"]);
    // λ 0.5 排出来的名次：开饭 −10 + 0.5·(−20 + 10) = −15 落后于 开放 −11 + 0.5·(−1 + 11) = −6。
    // 分只用来排名次，不写回：`score` 还是静态尺度的路径分，跨读法的比较（拼写纠错、混输）靠它
    assert!((paths[0].score - -11.0).abs() < 1e-9);
    assert!((paths[1].score - -10.0).abs() < 1e-9);
    assert!(!engine.rescoring_pending());
}

#[test]
fn async_scorer_waits_for_the_shell_to_request_and_poll() {
    let mut engine =
        engine().with_async_sentence_scorer(Box::new(Prefers("开放")), Some(0.5), None, None);
    let mut paths = vec![path("开饭", -10.0), path("开放", -11.0)];
    engine.rescore_paths(&mut paths, KEYS);
    // 第一次：没分，顺序不动，记下要分的
    assert_eq!(texts(&paths), ["开饭", "开放"]);
    assert!(engine.rescoring_pending());
    assert!(engine.request_rescoring());
    assert!(!engine.rescoring_pending());
    let started = Instant::now();
    while !engine.poll_rescoring() {
        assert!(started.elapsed() < Duration::from_secs(5), "后台没回结果");
        std::thread::sleep(Duration::from_millis(5));
    }
    let mut paths = vec![path("开饭", -10.0), path("开放", -11.0)];
    engine.rescore_paths(&mut paths, KEYS);
    assert_eq!(texts(&paths), ["开放", "开饭"]);
    // 没有新的要打的就不发
    assert!(!engine.request_rescoring());
}

#[test]
fn a_changed_context_discards_the_cached_scores() {
    let mut engine =
        engine().with_async_sentence_scorer(Box::new(Prefers("开放")), Some(0.5), None, None);
    engine.history_mut().record("今天");
    let mut paths = vec![path("开饭", -10.0), path("开放", -11.0)];
    engine.rescore_paths(&mut paths, KEYS);
    assert!(engine.request_rescoring());
    let started = Instant::now();
    while !engine.poll_rescoring() {
        assert!(started.elapsed() < Duration::from_secs(5));
        std::thread::sleep(Duration::from_millis(5));
    }
    // 上屏了别的字，前文变了：缓存作废，又得重新要
    engine.history_mut().record("很好");
    let mut paths = vec![path("开饭", -10.0), path("开放", -11.0)];
    engine.rescore_paths(&mut paths, KEYS);
    assert_eq!(texts(&paths), ["开饭", "开放"]);
    assert!(engine.rescoring_pending());
}

#[test]
fn the_shell_context_wins_over_session_history() {
    let mut engine = engine().with_sentence_scorer(Box::new(Prefers("开放")), None, None, Some(4));
    engine.history_mut().record("本会话上屏的历史");
    assert_eq!(engine.rescoring_context(), "屏的历史");
    engine.set_rescoring_context(Some("应用里光标前的文本".to_owned()));
    assert_eq!(engine.rescoring_context(), "前的文本");
    engine.set_rescoring_context(None);
    assert_eq!(engine.rescoring_context(), "屏的历史");
}

#[test]
fn changed_keys_discard_the_cached_scores() {
    let mut engine =
        engine().with_async_sentence_scorer(Box::new(Prefers("开放")), Some(0.5), None, None);
    let mut paths = vec![path("开饭", -10.0), path("开放", -11.0)];
    engine.rescore_paths(&mut paths, KEYS);
    assert!(engine.request_rescoring());
    let started = Instant::now();
    while !engine.poll_rescoring() {
        assert!(started.elapsed() < Duration::from_secs(5));
        std::thread::sleep(Duration::from_millis(5));
    }
    // 同一段前文，但换了一段按键：P2C 的条件变了，缓存不能复用
    let mut paths = vec![path("开饭", -10.0), path("开放", -11.0)];
    engine.rescore_paths(&mut paths, "kaifan");
    assert_eq!(texts(&paths), ["开饭", "开放"]);
    assert!(engine.rescoring_pending());
}

/// 假打分器：偏爱某个文本，生成时给出固定的几条。
struct PrefersAndGenerates(&'static str, &'static [&'static str]);

impl SentenceScorer for PrefersAndGenerates {
    fn score(&self, _context: &str, _keys: &str, texts: &[&str]) -> Vec<f64> {
        texts
            .iter()
            .map(|t| if *t == self.0 { -1.0 } else { -20.0 })
            .collect()
    }

    fn generate(&self, _keys: &str, _beam: usize, _max_chars: usize) -> Vec<String> {
        self.1.iter().map(|t| (*t).to_owned()).collect()
    }
}

/// 静态最优路径整段就是一个词时不重排：重排把多词拆分换上来，会绕过「整段是一个词就不出整句」那道关，
/// 把词级首选挤下去（2026-10-08 回放：`shihou` 时候 → 是后、`hexin` 核心 → 和新）。
#[test]
fn a_single_word_reading_is_not_displaced_by_a_rescored_split() {
    let dictionary =
        Dictionary::parse("时候\tshi hou\t90000\n是\tshi\t90000\n后\thou\t50000\n").unwrap();
    let mut engine = Engine::new(dictionary).with_sentence_scorer(
        Box::new(Prefers("是后")),
        Some(0.5),
        Some(f64::INFINITY),
        None,
    );
    engine.set_input("shihou");
    let query = engine.query().unwrap();
    assert_eq!(query.candidates.items[0].text, "时候");
    assert!(query.candidates.items.iter().all(|c| c.text != "是后"));
}

/// 拼写纠错把整段纠成一个词库词时词图已经读通，不让模型照着原样按键生成：
/// `kehudaun` 纠成 客户端，生成的 可互断 插在最前会把它挤下去。
#[test]
fn no_generation_when_correction_lands_on_a_whole_word() {
    let dictionary = Dictionary::parse(
        "客户端\tke hu duan\t90000\n客户\tke hu\t80000\n可\tke\t90000\n互\thu\t100\n断\tduan\t100\n",
    )
    .unwrap();
    let mut engine = Engine::new(dictionary).with_sentence_scorer(
        Box::new(PrefersAndGenerates("可互断", &["可互断"])),
        Some(0.5),
        None,
        None,
    );
    engine.set_input("kehudaun");
    let query = engine.query().unwrap();
    assert!(query.correction.is_some(), "这条要走拼写纠错");
    assert_eq!(query.candidates.items[0].text, "客户端");
    assert!(
        query
            .candidates
            .items
            .iter()
            .all(|c| c.kind != CandidateKind::Generated)
    );
}
