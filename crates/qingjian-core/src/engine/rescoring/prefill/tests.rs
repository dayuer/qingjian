//! B4 在 Core 一侧的行为：前文变了送预填（同一段只送一次）、私密进出清缓存、知微分数表跨按键保留、
//! 预填不顶掉排着的打分、复用数进统计。假打分器，不依赖模型文件。

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use qingjian_dictionary::Dictionary;

use crate::engine::Engine;
use crate::sentence::{Conversion, SentenceScorer, SentenceWord};

/// 记下自己被调了什么的假打分器。
#[derive(Clone, Default)]
struct Recorder {
    events: Arc<Mutex<Vec<String>>>,

    /// 分数不看按键（像知微）。
    keyless: bool,

    /// 预填与打分各睡这么久，用来让任务在队列里排起来。
    delay: Duration,
}

impl Recorder {
    fn events(&self) -> Vec<String> {
        self.events.lock().unwrap().clone()
    }

    fn push(&self, event: String) {
        self.events.lock().unwrap().push(event);
    }

    /// 等到某条事件出现（后台线程上发生的）。
    fn wait_for(&self, event: &str) {
        let started = Instant::now();
        while !self.events().iter().any(|e| e == event) {
            assert!(
                started.elapsed() < Duration::from_secs(5),
                "等不到 {event}：{:?}",
                self.events()
            );
            std::thread::sleep(Duration::from_millis(2));
        }
    }
}

impl SentenceScorer for Recorder {
    fn score(&self, _context: &str, _keys: &str, texts: &[&str]) -> Vec<f64> {
        std::thread::sleep(self.delay);
        self.push(format!("score:{}", texts.join(",")));
        texts
            .iter()
            .map(|t| if *t == "油箱" { -1.0 } else { -20.0 })
            .collect()
    }

    fn prefill(&self, context: &str) {
        std::thread::sleep(self.delay);
        self.push(format!("prefill:{context}"));
    }

    fn forget(&self) {
        self.push("forget".to_owned());
    }

    fn uses_keys(&self) -> bool {
        !self.keyless
    }

    fn prefix_reuse(&self) -> (usize, usize) {
        (3, 5)
    }
}

const WORDS: &str = "邮箱\tyou xiang\t9000\n油箱\tyou xiang\t3000\n邮\tyou\t900\n又\tyou\t800\n想\txiang\t900\n开发\tkai fa\t9000\n";

fn engine() -> Engine {
    Engine::new(Dictionary::parse(WORDS).unwrap())
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

#[test]
fn changed_context_is_prefilled_once() {
    let word = Recorder {
        keyless: true,
        ..Recorder::default()
    };
    let mut engine = engine().with_async_word_scorer(Box::new(word.clone()), Some(1.0));
    engine.set_rescoring_context(Some("我今天想".to_owned()));
    engine.set_rescoring_context(Some("我今天想".to_owned()));
    engine.set_rescoring_context(Some("我今天想去".to_owned()));
    word.wait_for("prefill:我今天想去");
    let prefills = word
        .events()
        .iter()
        .filter(|e| e.starts_with("prefill:"))
        .count();
    assert!(prefills <= 2, "同一段只送一次：{:?}", word.events());
}

#[test]
fn private_input_forgets_and_does_not_prefill() {
    let word = Recorder {
        keyless: true,
        ..Recorder::default()
    };
    let sentence = Recorder::default();
    let mut engine = engine()
        .with_async_sentence_scorer(Box::new(sentence.clone()), Some(0.5), None, None)
        .with_async_word_scorer(Box::new(word.clone()), Some(1.0));
    engine.set_private(true);
    word.wait_for("forget");
    sentence.wait_for("forget");
    engine.set_rescoring_context(Some("验证码 123456".to_owned()));
    engine.set_private(false);
    let started = Instant::now();
    while word.events().iter().filter(|e| *e == "forget").count() < 2 {
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "{:?}",
            word.events()
        );
        std::thread::sleep(Duration::from_millis(2));
    }
    assert!(
        word.events().iter().all(|e| !e.starts_with("prefill:")),
        "私密时不预填：{:?}",
        word.events()
    );
}

#[test]
fn sync_scorers_forget_directly() {
    let word = Recorder {
        keyless: true,
        ..Recorder::default()
    };
    let mut engine = engine().with_word_scorer(Box::new(word.clone()), Some(1.0));
    engine.forget_neural_context();
    assert_eq!(word.events(), ["forget"]);
}

/// 预填排在打分后面时不能把打分顶掉：被顶掉的打分永远不回，引擎会一直以为它在飞。
#[test]
fn prefill_does_not_swallow_a_queued_score() {
    let sentence = Recorder {
        delay: Duration::from_millis(80),
        ..Recorder::default()
    };
    let mut engine =
        engine().with_async_sentence_scorer(Box::new(sentence.clone()), Some(0.5), None, None);
    // 第一条预填占住线程，后面的打分与第二条预填在队列里排着
    engine.set_rescoring_context(Some("上文一".to_owned()));
    let mut paths = vec![path("开饭", -10.0), path("开放", -11.0)];
    engine.rescore_paths(&mut paths, "kaifang");
    assert!(engine.request_rescoring());
    engine.set_rescoring_context(Some("上文二".to_owned()));
    let started = Instant::now();
    while engine.rescoring_in_flight() {
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "打分被顶掉了：{:?}",
            sentence.events()
        );
        engine.poll_rescoring();
        std::thread::sleep(Duration::from_millis(2));
    }
    sentence.wait_for("prefill:上文二");
    let stats = engine.last_rescore_stats().expect("打分回来了");
    assert_eq!((stats.reused_tokens, stats.prefix_tokens), (3, 5));
}

/// 知微不看按键：同一前文下按键变了分数表不清，三次查询里 邮箱 / 油箱 只打一次（看按键的每键都打）。
#[test]
fn keyless_word_scores_survive_keystrokes() {
    for (keyless, expected) in [(true, 1), (false, 3)] {
        let word = Recorder {
            keyless,
            ..Recorder::default()
        };
        let mut engine = engine().with_word_scorer(Box::new(word.clone()), Some(1.0));
        for input in ["youxiang", "youxian", "youxiang"] {
            engine.set_input(input);
            engine.query().unwrap();
        }
        let scored = word
            .events()
            .iter()
            .filter(|e| e.starts_with("score:") && e.contains("油箱"))
            .count();
        assert_eq!(scored, expected, "keyless={keyless}: {:?}", word.events());
    }
}
