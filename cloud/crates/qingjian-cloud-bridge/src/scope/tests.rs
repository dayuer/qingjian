//! 分区学习：全局层之外只在选了对象时开对象层（场景自 2026-10-05 起只是用户自建的分组，不再有场景层）。
//! **按人隔离**：选了人时写只进这个人的对象层、不碰全局、不记个人 n-gram；没选人时写全局。
//! 计数类读两层加权、对象之间互不相通、计数类以外的读全局、删词连叠加层一起删、落盘刷全部层、
//! 对象目录不在就不开对象层（也不重建，那时当没选人）。

use std::path::PathBuf;

use qingjian_core::sentence::Context;
use qingjian_core::{Candidate, CandidateKind, Engine, Learner};
use qingjian_dictionary::{Dictionary, WordList};

use super::{ContactPick, ScopeState, ScopedLearner, contact_learning_dir, is_contact_id};

const A: &str = "0123456789abcdef0123456789abcdef";
const B: &str = "fedcba9876543210fedcba9876543210";
const K: u32 = ScopedLearner::OVERLAY_WEIGHT;

fn candidate(text: &str) -> Candidate {
    Candidate {
        text: text.to_owned(),
        kind: CandidateKind::Chinese,
        syllables: Vec::new(),
        reading: None,
        translation: None,
        aux_code: None,
    }
}

/// 每个测试一个临时的学习数据目录，记忆目录在它下面的 `memory/`；对象 A、B 的目录先建好（建对象时才有）。
fn dirs(name: &str) -> (PathBuf, PathBuf) {
    let user = std::env::temp_dir().join(format!("qj-scope-{name}-{}", std::process::id()));
    std::fs::remove_dir_all(&user).ok();
    let memory = user.join("memory");
    std::fs::create_dir_all(memory.join(A)).unwrap();
    std::fs::create_dir_all(memory.join(B)).unwrap();
    (user, memory)
}

#[test]
fn someone_picked_writes_only_their_layer() {
    let (user, memory) = dirs("contact");
    let mut learner = ScopedLearner::open(&user, &memory, Some(A));
    let handle = learner.handle();
    for _ in 0..10 {
        learner.record(&candidate("宝贝"));
    }
    assert_eq!(learner.weight("宝贝"), K * 10, "只进 A 的对象层，不碰全局");
    handle.switch(Some(B));
    assert_eq!(learner.weight("宝贝"), 0, "对象 B 读不到 A 那份");
    handle.switch(None);
    assert_eq!(learner.weight("宝贝"), 0, "不指定也读不到");
    handle.switch(Some(A));
    assert_eq!(
        learner.weight("宝贝"),
        K * 10,
        "换层时落过盘，换回同一对象读得到"
    );
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn nobody_picked_writes_global_and_everyone_reads_it() {
    let (user, memory) = dirs("global");
    let mut learner = ScopedLearner::open(&user, &memory, None);
    let handle = learner.handle();
    for _ in 0..3 {
        learner.record(&candidate("开会"));
    }
    assert_eq!(learner.weight("开会"), 3, "不指定写全局");
    handle.switch(Some(A));
    assert_eq!(learner.weight("开会"), 3, "选了人也读得到全局");
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn what_you_type_for_one_person_does_not_leak() {
    let (user, memory) = dirs("isolation");
    let mut learner = ScopedLearner::open(&user, &memory, Some(A));
    let handle = learner.handle();
    learner.record(&candidate("宝贝"));
    handle.switch(Some(B));
    assert_eq!(learner.weight("宝贝"), 0, "换个人就看不到了");
    handle.switch(None);
    assert_eq!(learner.weight("宝贝"), 0, "不指定也看不到");
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn counts_overlay_for_choices_raw_and_typos() {
    let (user, memory) = dirs("counts");
    let mut learner = ScopedLearner::open(&user, &memory, Some(A));
    learner.record_choice("bb", "宝贝");
    learner.record_raw("bb");
    learner.record_typo("bv", "bei");
    assert_eq!(learner.choice_weight("bb", "宝贝"), K, "选了人只算对象层");
    assert_eq!(learner.raw_count("bb"), K);
    assert_eq!(learner.typo_count("bv", "bei"), K);
    learner.unrecord_choice("bb", "宝贝");
    learner.unrecord_typo("bv", "bei");
    assert_eq!(learner.choice_weight("bb", "宝贝"), 0);
    assert_eq!(learner.typo_count("bv", "bei"), 0);
    learner.record(&candidate("宝贝"));
    learner.unrecord("宝贝");
    assert_eq!(learner.weight("宝贝"), 0);
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn forwarding_is_complete() {
    let (user, memory) = dirs("forward");
    let mut learner = ScopedLearner::open(&user, &memory, None);
    assert_eq!(learner.merge_remote("user\tadd\t开发\t3\n"), 1);
    learner.learn_word("青简", &["qing".to_owned(), "jian".to_owned()]);
    assert!(learner.user_words().is_some());
    learner.learn_english("gist");
    assert!(learner.user_english().is_some());
    learner.flush();
    let read = |name: &str| std::fs::read_to_string(user.join(name)).unwrap_or_default();
    assert!(read("user.tsv").contains("开发"));
    assert!(read("user-words.tsv").contains("青简"));
    assert!(read("user-english.tsv").contains("gist"));
    assert!(learner.forget("开发").learning);
    assert!(learner.forget_english("gist"));
    assert_eq!(learner.weight("开发"), 0);
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn transitions_go_to_global_only_when_nobody_is_picked() {
    let (user, memory) = dirs("transition");
    let mut learner = ScopedLearner::open(&user, &memory, None);
    learner.record_transition(Context::START, "你好", 1);
    assert!(learner.user_ngram().is_some(), "不指定时记转移");
    learner.unrecord_transition(Context::START, "你好", 1);
    assert!(learner.user_ngram().is_none(), "撤的也是全局那份");

    // 选了人就一个字都不记：个人 n-gram 只读全局，记了会在别的对象下冒出来
    let mut learner = ScopedLearner::open(&user, &memory, Some(A));
    learner.record_transition(Context::START, "宝贝", 1);
    assert!(learner.user_ngram().is_none(), "选了人不记转移");
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn forget_clears_the_open_overlay_layers() {
    let (user, memory) = dirs("forget");
    let mut learner = ScopedLearner::open(&user, &memory, Some(A));
    learner.record(&candidate("宝贝"));
    learner.learn_english("honey");
    assert_eq!(learner.weight("宝贝"), K);
    let forgotten = learner.forget("宝贝");
    assert!(forgotten.learning);
    assert_eq!(learner.weight("宝贝"), 0, "全局与对象层一起删");
    assert!(learner.forget_english("honey"));
    assert!(!learner.forget_english("honey"));
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn missing_contact_dir_is_not_recreated() {
    let (user, memory) = dirs("missing");
    let gone = "11111111111111111111111111111111";
    let mut learner = ScopedLearner::open(&user, &memory, Some(gone));
    learner.record(&candidate("宝贝"));
    learner.flush();
    assert_eq!(learner.weight("宝贝"), 1, "对象目录不在，只剩全局层");
    assert!(!memory.join(gone).exists());
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn flush_writes_the_layers_that_got_the_writes() {
    let (user, memory) = dirs("flush");
    let mut learner = ScopedLearner::open(&user, &memory, Some(A));
    learner.record(&candidate("宝贝"));
    learner.flush();
    assert!(
        contact_learning_dir(&memory, A).join("user.tsv").is_file(),
        "写了对象层"
    );
    assert!(!user.join("user.tsv").exists(), "选了人不写全局");

    let mut learner = ScopedLearner::open(&user, &memory, None);
    learner.record(&candidate("开会"));
    learner.flush();
    assert!(user.join("user.tsv").is_file(), "不指定写全局");
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn bad_contact_ids_are_ignored() {
    assert!(is_contact_id(A));
    assert!(!is_contact_id("../etc"));
    assert!(!is_contact_id(&A.to_uppercase()));
    let (user, memory) = dirs("bad-id");
    let mut learner = ScopedLearner::open(&user, &memory, Some("../x"));
    learner.record(&candidate("宝贝"));
    assert_eq!(learner.weight("宝贝"), 1, "不合格的 id 当没选对象，写全局");
    assert!(!memory.join("..").join("x").join("learning").exists());
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn scope_state_defaults() {
    let state: ScopeState =
        serde_json::from_str(r#"{"scene":"dating","contact_id":null,"hints":false}"#).unwrap();
    assert_eq!(state.scene, "dating", "旧文件里多出的开关字段忽略");
    assert_eq!(state.contact_id, None);
    assert!(state.last.is_empty(), "旧文件没有 last");
    assert_eq!(ScopeState::default().scene, "daily");

    let state = ScopeState {
        scene: "daily".to_owned(),
        contact_id: Some(A.to_owned()),
        last: [
            ("daily".to_owned(), A.to_owned()),
            ("dating".to_owned(), B.to_owned()),
        ]
        .into(),
        used: [(A.to_owned(), 1_791_043_200)].into(),
    };
    let json = serde_json::to_value(&state).unwrap();
    assert_eq!(json["last"]["dating"], B, "last 按场景 id 存");
    assert_eq!(json["used"][A], 1_791_043_200);
    assert_eq!(serde_json::from_value::<ScopeState>(json).unwrap(), state);
}

#[test]
fn contact_pick_follows_the_c_convention() {
    assert_eq!(ContactPick::from_arg(None), ContactPick::Last);
    assert_eq!(ContactPick::from_arg(Some("")), ContactPick::Nobody);
    assert_eq!(
        ContactPick::from_arg(Some(A)),
        ContactPick::Contact(A.to_owned())
    );
}

fn engine(learner: ScopedLearner) -> Engine {
    Engine::new(Dictionary::parse("你好\tni hao\t100\n").unwrap())
        .with_english(WordList::parse("hello\n").unwrap())
        .with_learner(Box::new(learner))
}

/// 敲 `input` 时候选里有没有 `text`。
fn shows(engine: &mut Engine, input: &str, text: &str) -> bool {
    engine.set_input(input);
    let found = engine
        .query()
        .unwrap()
        .candidates
        .items
        .iter()
        .any(|item| item.text == text);
    engine.set_input("");
    found
}

fn dudu() -> Vec<String> {
    vec!["du".to_owned(), "du".to_owned()]
}

/// 会话换人的做法：换层后调 `scope_changed`（`session/memory` 的 `switch_layers`）。
fn switch(engine: &mut Engine, handle: &super::ScopeHandle, contact: Option<&str>) {
    handle.switch(contact);
    engine.learner_mut().scope_changed();
}

#[test]
fn new_words_made_for_someone_stay_with_them() {
    let (user, memory) = dirs("new-words");
    let learner = ScopedLearner::open(&user, &memory, Some(A));
    let handle = learner.handle();
    let mut engine = engine(learner);
    engine.learner_mut().learn_word("嘟嘟", &dudu());
    engine.learner_mut().learn_english("gist");
    assert!(shows(&mut engine, "dudu", "嘟嘟"), "选着 A 看得到 A 的新词");
    assert!(shows(&mut engine, "gist", "gist"), "英文新词也是");

    switch(&mut engine, &handle, Some(B));
    assert!(!shows(&mut engine, "dudu", "嘟嘟"), "换到 B 就没有");
    assert!(!shows(&mut engine, "gist", "gist"));
    assert!(engine.learner().user_words().is_none(), "也没进全局");
    assert!(engine.learner().user_english().is_none());

    switch(&mut engine, &handle, None);
    assert!(!shows(&mut engine, "dudu", "嘟嘟"), "不指定也没有");
    assert!(!shows(&mut engine, "gist", "gist"));

    switch(&mut engine, &handle, Some(A));
    assert!(
        shows(&mut engine, "dudu", "嘟嘟"),
        "换回 A 又有了（落过盘）"
    );
    assert!(shows(&mut engine, "gist", "gist"));
    engine.learner_mut().flush();
    assert!(
        !user.join("user-words.tsv").exists(),
        "全局用户词文件都没建"
    );
    assert!(
        std::fs::read_to_string(contact_learning_dir(&memory, A).join("user-words.tsv"))
            .unwrap()
            .contains("嘟嘟")
    );
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn someone_picked_sees_global_and_their_own_words() {
    let (user, memory) = dirs("both-words");
    let learner = ScopedLearner::open(&user, &memory, None);
    let handle = learner.handle();
    let mut engine = engine(learner);
    engine
        .learner_mut()
        .learn_word("青简", &["qing".to_owned(), "jian".to_owned()]);
    switch(&mut engine, &handle, Some(A));
    engine.learner_mut().learn_word("嘟嘟", &dudu());
    assert!(shows(&mut engine, "qingjian", "青简"), "全局用户词照样有");
    assert!(shows(&mut engine, "dudu", "嘟嘟"), "这个人的也有");
    switch(&mut engine, &handle, Some(B));
    assert!(shows(&mut engine, "qingjian", "青简"), "全局的谁都看得到");
    assert!(!shows(&mut engine, "dudu", "嘟嘟"));
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn nobody_picked_learns_new_words_globally() {
    let (user, memory) = dirs("global-words");
    let mut learner = ScopedLearner::open(&user, &memory, None);
    learner.learn_word("嘟嘟", &dudu());
    learner.learn_english("gist");
    assert!(learner.user_words().is_some(), "不指定照旧写全局");
    assert!(learner.user_english().is_some());
    assert!(learner.scoped_user_words().is_none());
    let handle = learner.handle();
    let mut engine = engine(learner);
    switch(&mut engine, &handle, Some(A));
    assert!(shows(&mut engine, "dudu", "嘟嘟"), "选了人也看得到全局的");
    assert!(shows(&mut engine, "gist", "gist"));
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn stale_snapshot_never_leaks_before_scope_changed() {
    let (user, memory) = dirs("stale");
    let mut learner = ScopedLearner::open(&user, &memory, Some(A));
    learner.learn_word("嘟嘟", &dudu());
    learner.learn_english("gist");
    assert!(learner.scoped_user_words().is_some());
    let handle = learner.handle();
    handle.switch(Some(B));
    assert!(
        learner.scoped_user_words().is_none(),
        "没调 scope_changed 也查不到 A 的"
    );
    assert!(learner.scoped_user_english().is_none());
    learner.scope_changed();
    assert!(learner.scoped_user_words().is_none(), "B 自己没有新词");
    handle.switch(Some(A));
    assert!(learner.scoped_user_words().is_none(), "换回 A 要等重建");
    learner.scope_changed();
    assert!(learner.scoped_user_words().is_some());
    assert!(learner.scoped_user_english().is_some());
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn forgetting_a_persons_word_drops_it_from_candidates() {
    let (user, memory) = dirs("forget-word");
    let learner = ScopedLearner::open(&user, &memory, Some(A));
    let mut engine = engine(learner);
    engine.learner_mut().learn_word("嘟嘟", &dudu());
    engine.learner_mut().learn_english("gist");
    assert!(engine.learner_mut().forget("嘟嘟").user_word);
    assert!(engine.learner_mut().forget_english("gist"));
    assert!(!shows(&mut engine, "dudu", "嘟嘟"));
    assert!(!shows(&mut engine, "gist", "gist"));
    std::fs::remove_dir_all(&user).ok();
}
