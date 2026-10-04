//! 分区学习：恋爱场景的写不进全局、对象之间互不相通、计数类读三层加权、其余方法读全局、恋爱场景不记转移、
//! 日常与工作选了人时只开对象层、全局与对象层都写，删词连叠加层一起删、落盘三层都刷、对象目录不在就不开对象层（也不重建）。

use std::path::PathBuf;

use qingjian_cloud_proto::Scene;
use qingjian_core::sentence::Context;
use qingjian_core::{Candidate, CandidateKind, Learner};

use super::{
    ContactPick, ScopeState, ScopedLearner, contact_learning_dir, is_contact_id, parse_scene,
    scene_label, scene_learning_dir, scene_name,
};

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
fn dating_writes_do_not_reach_work() {
    let (user, memory) = dirs("dating-work");
    let mut learner = ScopedLearner::open(&user, &memory, Scene::Dating, Some(A));
    let handle = learner.handle();
    for _ in 0..10 {
        learner.record(&candidate("宝贝"));
    }
    // 恋爱场景读「全局 + k×场景 + k×对象」：全局没写，场景层与对象层各 10 次
    assert_eq!(learner.weight("宝贝"), 2 * K * 10);
    handle.switch(Scene::Work, None);
    assert_eq!(learner.weight("宝贝"), 0, "工作场景只读全局");
    handle.switch(Scene::Dating, Some(A));
    assert_eq!(
        learner.weight("宝贝"),
        2 * K * 10,
        "换层时落过盘，换回同一对象读得到"
    );
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn contacts_are_isolated() {
    let (user, memory) = dirs("contacts");
    let mut learner = ScopedLearner::open(&user, &memory, Scene::Dating, Some(A));
    let handle = learner.handle();
    for _ in 0..10 {
        learner.record(&candidate("宝贝"));
    }
    handle.switch(Scene::Dating, Some(B));
    assert_eq!(learner.weight("宝贝"), K * 10, "对象 B 只读到场景层");
    handle.switch(Scene::Dating, None);
    assert_eq!(learner.weight("宝贝"), K * 10, "不指定对象也只有场景层");
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn daily_and_work_share_global() {
    let (user, memory) = dirs("daily-work");
    let mut learner = ScopedLearner::open(&user, &memory, Scene::Daily, None);
    let handle = learner.handle();
    for _ in 0..3 {
        learner.record(&candidate("开会"));
    }
    assert_eq!(learner.weight("开会"), 3);
    handle.switch(Scene::Work, None);
    assert_eq!(learner.weight("开会"), 3);
    handle.switch(Scene::Dating, None);
    assert_eq!(learner.weight("开会"), 3, "恋爱场景也读全局，叠加层是空的");
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn daily_and_work_contacts_write_global_and_contact_layers() {
    for scene in [Scene::Daily, Scene::Work] {
        let (user, memory) = dirs(&format!("contact-{}", scene_name(scene)));
        let mut learner = ScopedLearner::open(&user, &memory, scene, Some(A));
        let handle = learner.handle();
        for _ in 0..3 {
            learner.record(&candidate("周报"));
        }
        // 只开对象层、不开场景层：全局 3 次 + k×对象层 3 次
        assert_eq!(learner.weight("周报"), 3 + K * 3, "{scene:?}");
        learner.record_transition(Context::START, "周报", 1);
        assert!(learner.user_ngram().is_some(), "{scene:?} 记转移");
        learner.flush();
        assert!(user.join("user.tsv").is_file(), "{scene:?} 写了全局");
        assert!(
            contact_learning_dir(&memory, A).join("user.tsv").is_file(),
            "{scene:?} 写了对象层"
        );
        assert!(
            !scene_learning_dir(&memory, scene).exists(),
            "{scene:?} 不开场景层"
        );
        handle.switch(scene, Some(B));
        assert_eq!(learner.weight("周报"), 3, "{scene:?} 换人只剩全局");
        handle.switch(scene, None);
        assert_eq!(learner.weight("周报"), 3, "{scene:?} 不指定只读全局");
        learner.unrecord("周报");
        handle.switch(scene, Some(A));
        assert_eq!(
            learner.weight("周报"),
            2 + K * 3,
            "{scene:?} 不指定时只撤全局"
        );
        std::fs::remove_dir_all(&user).ok();
    }
}

#[test]
fn counts_overlay_for_choices_raw_and_typos() {
    let (user, memory) = dirs("counts");
    let mut learner = ScopedLearner::open(&user, &memory, Scene::Dating, Some(A));
    learner.record_choice("bb", "宝贝");
    learner.record_raw("bb");
    learner.record_typo("bv", "bei");
    assert_eq!(learner.choice_weight("bb", "宝贝"), 2 * K);
    assert_eq!(learner.raw_count("bb"), 2 * K);
    assert_eq!(learner.typo_count("bv", "bei"), 2 * K);
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
    for scene in [Scene::Daily, Scene::Dating, Scene::Work] {
        let (user, memory) = dirs(&format!("forward-{}", scene_name(scene)));
        let contact = (scene == Scene::Dating).then_some(A);
        let mut learner = ScopedLearner::open(&user, &memory, scene, contact);
        assert_eq!(learner.merge_remote("user\tadd\t开发\t3\n"), 1, "{scene:?}");
        learner.learn_word("青简", &["qing".to_owned(), "jian".to_owned()]);
        assert!(learner.user_words().is_some(), "{scene:?}");
        learner.learn_english("gist");
        assert!(learner.user_english().is_some(), "{scene:?}");
        learner.flush();
        let read = |name: &str| std::fs::read_to_string(user.join(name)).unwrap_or_default();
        assert!(read("user.tsv").contains("开发"), "{scene:?}");
        assert!(read("user-words.tsv").contains("青简"), "{scene:?}");
        assert!(read("user-english.tsv").contains("gist"), "{scene:?}");
        assert!(learner.forget("开发").learning, "{scene:?}");
        assert!(learner.forget_english("gist"), "{scene:?}");
        assert_eq!(learner.weight("开发"), 0, "{scene:?}");
        std::fs::remove_dir_all(&user).ok();
    }
}

#[test]
fn dating_does_not_write_transitions() {
    for scene in [Scene::Daily, Scene::Dating, Scene::Work] {
        let (user, memory) = dirs(&format!("transition-{}", scene_name(scene)));
        let contact = (scene == Scene::Dating).then_some(A);
        let mut learner = ScopedLearner::open(&user, &memory, scene, contact);
        learner.record_transition(Context::START, "你好", 1);
        assert_eq!(
            learner.user_ngram().is_some(),
            scene != Scene::Dating,
            "{scene:?}"
        );
        std::fs::remove_dir_all(&user).ok();
    }
    // 日常记下的转移，恋爱场景照样读得到、也不会被恋爱场景撤掉
    let (user, memory) = dirs("transition-read");
    let mut learner = ScopedLearner::open(&user, &memory, Scene::Daily, None);
    learner.record_transition(Context::START, "你好", 1);
    learner.handle().switch(Scene::Dating, Some(A));
    learner.unrecord_transition(Context::START, "你好", 1);
    assert!(learner.user_ngram().is_some());
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn forget_clears_the_open_overlay_layers() {
    let (user, memory) = dirs("forget");
    let mut learner = ScopedLearner::open(&user, &memory, Scene::Dating, Some(A));
    learner.record(&candidate("宝贝"));
    learner.learn_english("honey");
    assert_eq!(learner.weight("宝贝"), 2 * K);
    let forgotten = learner.forget("宝贝");
    assert!(forgotten.learning);
    assert_eq!(learner.weight("宝贝"), 0, "场景层与对象层一起删");
    assert!(learner.forget_english("honey"));
    assert!(!learner.forget_english("honey"));
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn missing_contact_dir_is_not_recreated() {
    let (user, memory) = dirs("missing");
    let gone = "11111111111111111111111111111111";
    let mut learner = ScopedLearner::open(&user, &memory, Scene::Dating, Some(gone));
    learner.record(&candidate("宝贝"));
    learner.flush();
    assert_eq!(learner.weight("宝贝"), K, "对象目录不在，只有场景层");
    assert!(!memory.join(gone).exists());
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn flush_writes_all_layers() {
    let (user, memory) = dirs("flush");
    let mut learner = ScopedLearner::open(&user, &memory, Scene::Dating, Some(A));
    learner.record(&candidate("宝贝"));
    learner.flush();
    assert!(
        scene_learning_dir(&memory, Scene::Dating)
            .join("user.tsv")
            .is_file()
    );
    assert!(contact_learning_dir(&memory, A).join("user.tsv").is_file());
    assert!(!user.join("user.tsv").exists(), "恋爱场景不写全局");
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn bad_contact_ids_are_ignored_and_scenes_parse() {
    assert!(is_contact_id(A));
    assert!(!is_contact_id("../etc"));
    assert!(!is_contact_id(&A.to_uppercase()));
    assert_eq!(parse_scene("party"), None);
    for scene in [Scene::Daily, Scene::Dating, Scene::Work] {
        assert_eq!(parse_scene(scene_name(scene)), Some(scene));
    }
    let (user, memory) = dirs("bad-id");
    let mut learner = ScopedLearner::open(&user, &memory, Scene::Dating, Some("../x"));
    learner.record(&candidate("宝贝"));
    assert_eq!(
        learner.weight("宝贝"),
        K,
        "不合格的 id 当没选对象，只有场景层"
    );
    assert!(!memory.join("..").join("x").join("learning").exists());
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn scope_state_defaults() {
    let state: ScopeState =
        serde_json::from_str(r#"{"scene":"dating","contact_id":null,"hints":false}"#).unwrap();
    assert_eq!(state.scene, Scene::Dating, "旧文件里多出的开关字段忽略");
    assert_eq!(state.contact_id, None);
    assert!(state.last.is_empty(), "旧文件没有 last");
    assert_eq!(ScopeState::default().scene, Scene::Daily);

    let state = ScopeState {
        scene: Scene::Daily,
        contact_id: Some(A.to_owned()),
        last: [(Scene::Daily, A.to_owned()), (Scene::Dating, B.to_owned())].into(),
    };
    let json = serde_json::to_value(&state).unwrap();
    assert_eq!(json["last"]["dating"], B, "last 按场景名存");
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
    assert_eq!(scene_label(Scene::Daily), "日常");
}
