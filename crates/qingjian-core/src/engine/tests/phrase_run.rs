//! 连打短语：分三段打（shufu / zhe / ne）、每段整段选完，选够次数就造成用户词，整串连着打时它排第一。

use super::*;

/// 复现 `shufuzhene` 的形状：整串最优切分是 zhen'e（真饿），zhe'ne 的 着 + 呢 在词图里出不来。
const PHRASE: &str = "舒服\tshu fu\t5000\n着\tzhe\t3000\n真\tzhen\t9000\n饿\te\t5000\n呢\tne\t800\n哪\tne\t600\n着呢\tzhe ne\t200\n";

fn phrase_engine() -> Engine {
    let shared = Arc::new(Mutex::new((Vec::new(), sentence::UserNgram::default())));
    let learner = WordLearner {
        shared,
        ..WordLearner::default()
    };
    Engine::new(Dictionary::parse(PHRASE).unwrap()).with_learner(Box::new(learner))
}

fn pick(engine: &mut Engine, input: &str, text: &str) {
    engine.set_input(input);
    let candidate = engine
        .query()
        .unwrap()
        .candidates
        .items
        .into_iter()
        .find(|c| c.text == text && c.kind == CandidateKind::Chinese)
        .unwrap_or_else(|| panic!("{input} 的候选里没有 {text}"));
    engine.commit(&candidate);
    assert!(engine.composition().is_empty());
}

/// 分三段打一轮：舒服 / 着 / 呢，打完回车（透传打断链）。
fn type_in_three(engine: &mut Engine) {
    pick(engine, "shufu", "舒服");
    pick(engine, "zhe", "着");
    pick(engine, "ne", "呢");
    engine.note_passthrough('\n');
}

fn first_for(engine: &mut Engine, input: &str) -> String {
    engine.set_input(input);
    let first = texts_of(engine).remove(0);
    engine.clear();
    first
}

#[test]
fn three_pieces_typed_apart_become_a_phrase_after_enough_rounds() {
    let mut engine = phrase_engine();
    assert_ne!(first_for(&mut engine, "shufuzhene"), "舒服着呢");
    for _ in 1..PHRASE_RUN_THRESHOLD {
        type_in_three(&mut engine);
    }
    assert_ne!(
        first_for(&mut engine, "shufuzhene"),
        "舒服着呢",
        "不到次数就不造词"
    );
    type_in_three(&mut engine);
    assert_eq!(first_for(&mut engine, "shufuzhene"), "舒服着呢");
}

#[test]
fn erasing_the_last_piece_and_choosing_another_word_retracts_the_phrase() {
    let mut engine = phrase_engine();
    for _ in 1..PHRASE_RUN_THRESHOLD {
        type_in_three(&mut engine);
    }
    // 第三轮最后一段选了 呢 又删掉改选 哪：这一轮的 舒服着呢 不算
    pick(&mut engine, "shufu", "舒服");
    pick(&mut engine, "zhe", "着");
    pick(&mut engine, "ne", "呢");
    engine.note_backspace();
    pick(&mut engine, "ne", "哪");
    engine.note_passthrough('\n');
    assert_eq!(
        engine.learner().choice_weight("shufuzhene", "舒服着呢"),
        PHRASE_RUN_THRESHOLD - 1
    );
    assert_ne!(first_for(&mut engine, "shufuzhene"), "舒服着呢");
}

#[test]
fn punctuation_between_the_pieces_breaks_the_phrase() {
    let mut engine = phrase_engine();
    for _ in 0..PHRASE_RUN_THRESHOLD + 1 {
        pick(&mut engine, "shufu", "舒服");
        engine.note_passthrough('，');
        pick(&mut engine, "zhe", "着");
        pick(&mut engine, "ne", "呢");
        engine.note_passthrough('\n');
    }
    assert_eq!(engine.learner().choice_weight("shufuzhene", "舒服着呢"), 0);
    assert_ne!(first_for(&mut engine, "shufuzhene"), "舒服着呢");
}

#[test]
fn a_long_pause_between_the_pieces_breaks_the_phrase() {
    let mut engine = phrase_engine();
    for _ in 0..PHRASE_RUN_THRESHOLD + 1 {
        pick(&mut engine, "shufu", "舒服");
        pick(&mut engine, "zhe", "着");
        // 打完 着 隔了一小时才接着打 呢
        engine.chain.age_run(std::time::Duration::from_secs(3600));
        pick(&mut engine, "ne", "呢");
        engine.note_passthrough('\n');
    }
    assert_eq!(engine.learner().choice_weight("shufuzhene", "舒服着呢"), 0);
    assert_ne!(first_for(&mut engine, "shufuzhene"), "舒服着呢");
}

#[test]
fn switching_focus_between_the_pieces_breaks_the_phrase() {
    let mut engine = phrase_engine();
    for _ in 0..PHRASE_RUN_THRESHOLD + 1 {
        pick(&mut engine, "shufu", "舒服");
        // 换了应用 / 输入框（壳调 break_chain）
        engine.break_chain();
        pick(&mut engine, "zhe", "着");
        pick(&mut engine, "ne", "呢");
        engine.note_passthrough('\n');
    }
    assert_eq!(engine.learner().choice_weight("shufuzhene", "舒服着呢"), 0);
    assert_ne!(first_for(&mut engine, "shufuzhene"), "舒服着呢");
}
