//! 分区学习：全局层之外只在选了对象时开对象层（场景自 2026-10-05 起只是用户自建的分组，不再有场景层），
//! 计数类读两层加权、写两层都写、对象之间互不相通、计数类以外的读全局、删词连叠加层一起删、
//! 落盘两层都刷、对象目录不在就不开对象层（也不重建）。

use std::path::PathBuf;

use qingjian_core::sentence::Context;
use qingjian_core::{Candidate, CandidateKind, Learner};

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
fn a_contact_layer_sits_on_top_of_global() {
    let (user, memory) = dirs("contact");
    let mut learner = ScopedLearner::open(&user, &memory, Some(A));
    let handle = learner.handle();
    for _ in 0..10 {
        learner.record(&candidate("宝贝"));
    }
    assert_eq!(learner.weight("宝贝"), 10 + K * 10, "全局 + k×对象层");
    handle.switch(Some(B));
    assert_eq!(learner.weight("宝贝"), 10, "对象 B 只剩全局");
    handle.switch(None);
    assert_eq!(learner.weight("宝贝"), 10, "不指定只剩全局");
    handle.switch(Some(A));
    assert_eq!(
        learner.weight("宝贝"),
        10 + K * 10,
        "换层时落过盘，换回同一对象读得到"
    );
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn contacts_are_isolated() {
    let (user, memory) = dirs("contacts");
    let mut learner = ScopedLearner::open(&user, &memory, Some(A));
    let handle = learner.handle();
    for _ in 0..10 {
        learner.record(&candidate("宝贝"));
    }
    handle.switch(Some(B));
    assert_eq!(learner.weight("宝贝"), 10, "对象 B 读不到 A 那份");
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn counts_overlay_for_choices_raw_and_typos() {
    let (user, memory) = dirs("counts");
    let mut learner = ScopedLearner::open(&user, &memory, Some(A));
    learner.record_choice("bb", "宝贝");
    learner.record_raw("bb");
    learner.record_typo("bv", "bei");
    assert_eq!(learner.choice_weight("bb", "宝贝"), 1 + K);
    assert_eq!(learner.raw_count("bb"), 1 + K);
    assert_eq!(learner.typo_count("bv", "bei"), 1 + K);
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
    let mut learner = ScopedLearner::open(&user, &memory, Some(A));
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
fn transitions_go_to_global() {
    let (user, memory) = dirs("transition");
    let mut learner = ScopedLearner::open(&user, &memory, Some(A));
    learner.record_transition(Context::START, "你好", 1);
    assert!(learner.user_ngram().is_some(), "记转移");
    learner.unrecord_transition(Context::START, "你好", 1);
    assert!(learner.user_ngram().is_none(), "撤的也是全局那份");
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn forget_clears_the_open_overlay_layers() {
    let (user, memory) = dirs("forget");
    let mut learner = ScopedLearner::open(&user, &memory, Some(A));
    learner.record(&candidate("宝贝"));
    learner.learn_english("honey");
    assert_eq!(learner.weight("宝贝"), 1 + K);
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
fn flush_writes_both_layers() {
    let (user, memory) = dirs("flush");
    let mut learner = ScopedLearner::open(&user, &memory, Some(A));
    learner.record(&candidate("宝贝"));
    learner.flush();
    assert!(user.join("user.tsv").is_file(), "写了全局");
    assert!(contact_learning_dir(&memory, A).join("user.tsv").is_file());
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
    assert_eq!(learner.weight("宝贝"), 1, "不合格的 id 当没选对象");
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
