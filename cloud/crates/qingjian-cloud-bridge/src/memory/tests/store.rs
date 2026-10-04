//! 记忆存储：往返、8 个上限、忘掉一个人、坏文件改名、并发写、App 整份写回的规则、旧格式的 cards.json。
//! 两个进程同时写、读失败不覆盖、忘掉的人不复活在 `sync.rs`。

use qingjian_cloud_proto::{CardKind, Scene};

use super::{card, contact, id, pick, temp_dir};
use crate::memory::{MemoryError, MemorySnapshot, MemoryStore};
use crate::scope::ScopeState;

#[test]
fn round_trips_contacts_cards_and_state() {
    let user = temp_dir("round-trip");
    let store = MemoryStore::open(&user);
    assert!(store.contacts().is_empty());
    assert_eq!(store.state(), ScopeState::default());

    store.put_contact(contact(1, Scene::Dating)).unwrap();
    assert_eq!(store.contacts(), vec![contact(1, Scene::Dating)]);
    let mut renamed = contact(1, Scene::Dating);
    renamed.name = "小美".to_owned();
    store.put_contact(renamed.clone()).unwrap();
    assert_eq!(store.contacts(), vec![renamed], "同 id 是改，不是加");

    let cards = vec![card(
        1,
        CardKind::Date,
        "生日",
        &["生日"],
        Some("2026-10-05"),
        1,
    )];
    store.put_cards(&id(1), &cards).unwrap();
    assert_eq!(store.cards(&id(1)), cards);
    assert!(store.cards("../x").is_empty());

    let state = store.update_scope(Scene::Dating, &pick(1), 0).unwrap();
    assert_eq!(state.contact_id, Some(id(1)));
    assert_eq!(store.state(), state);
    let state = store.update_scope(Scene::Work, &pick(1), 0).unwrap();
    assert_eq!(state.contact_id, None, "不是这个场景的人当不指定");
    let state = store.update_scope(Scene::Dating, &pick(9), 0).unwrap();
    assert_eq!(state.contact_id, None, "名单上没有的对象当不指定");
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn ninth_dating_contact_hits_the_limit() {
    let user = temp_dir("limit");
    let store = MemoryStore::open(&user);
    for n in 0..8 {
        store.put_contact(contact(n, Scene::Dating)).unwrap();
    }
    assert!(matches!(
        store.put_contact(contact(8, Scene::Dating)),
        Err(MemoryError::ContactLimit(Scene::Dating))
    ));
    store.put_contact(contact(9, Scene::Daily)).unwrap();
    store.put_contact(contact(0, Scene::Dating)).unwrap();
    assert_eq!(store.contacts().len(), 9, "日常另算，改已有的不算新增");
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn forget_contact_removes_the_directory() {
    let user = temp_dir("forget");
    let store = MemoryStore::open(&user);
    store.put_contact(contact(1, Scene::Dating)).unwrap();
    store
        .put_cards(&id(1), &[card(1, CardKind::Other, "喜欢猫", &[], None, 1)])
        .unwrap();
    let dir = user.join("memory").join(id(1));
    assert!(dir.is_dir());
    store.forget_contact(&id(1)).unwrap();
    assert!(!dir.exists());
    assert!(store.contacts().is_empty());
    assert!(matches!(
        store.forget_contact("../x"),
        Err(MemoryError::Invalid(_))
    ));
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn broken_files_are_renamed_and_read_as_empty() {
    let user = temp_dir("broken");
    let store = MemoryStore::open(&user);
    let memory = user.join("memory");
    std::fs::create_dir_all(&memory).unwrap();
    std::fs::write(memory.join("contacts.json"), "{not json").unwrap();
    assert!(store.contacts().is_empty());
    let names: Vec<String> = std::fs::read_dir(&memory)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert!(
        names.iter().any(|n| n.starts_with("contacts.json.broken-")),
        "{names:?}"
    );
    assert!(!memory.join("contacts.json").exists());

    store.put_contact(contact(1, Scene::Dating)).unwrap();
    std::fs::create_dir_all(memory.join(id(1))).unwrap();
    std::fs::write(memory.join(id(1)).join("cards.json"), "[{").unwrap();
    let snapshot = store.snapshot().unwrap();
    assert_eq!(snapshot.broken, vec![id(1)]);
    assert!(snapshot.cards[&id(1)].is_empty());
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn concurrent_writes_leave_a_parseable_file() {
    let user = temp_dir("concurrent");
    let store = MemoryStore::open(&user);
    store.put_contact(contact(1, Scene::Dating)).unwrap();
    let threads: Vec<_> = (0..2u32)
        .map(|t| {
            let store = store.clone();
            std::thread::spawn(move || {
                for i in 0..50u32 {
                    let cards = vec![card(
                        t * 100 + i,
                        CardKind::Other,
                        "x",
                        &[],
                        None,
                        i64::from(i),
                    )];
                    store.put_cards(&id(1), &cards).unwrap();
                }
            })
        })
        .collect();
    for thread in threads {
        thread.join().unwrap();
    }
    let dir = user.join("memory").join(id(1));
    assert_eq!(store.try_cards(&id(1)).unwrap().len(), 1);
    assert_eq!(
        store.snapshot().unwrap().revs[&id(1)],
        100,
        "每写一次修订号加一"
    );
    let leftovers: Vec<String> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|name| name.ends_with(".tmp"))
        .collect();
    assert!(leftovers.is_empty(), "{leftovers:?}");
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn snapshot_write_replaces_all_but_the_current_scene() {
    let user = temp_dir("snapshot");
    let store = MemoryStore::open(&user);
    store.put_contact(contact(1, Scene::Dating)).unwrap();
    store.put_contact(contact(2, Scene::Dating)).unwrap();
    store
        .put_cards(&id(2), &[card(2, CardKind::Other, "x", &[], None, 1)])
        .unwrap();
    store.update_scope(Scene::Dating, &pick(2), 0).unwrap();

    let mut snapshot = store.snapshot().unwrap();
    snapshot.contacts.retain(|c| c.id == id(1));
    snapshot.cards.remove(&id(2));
    snapshot.cards.insert(
        id(1),
        vec![card(5, CardKind::Preference, "喜欢草莓", &[], None, 2)],
    );
    snapshot.state = ScopeState {
        scene: Scene::Work,
        ..ScopeState::default()
    };
    store.write_snapshot(&snapshot).unwrap();

    assert_eq!(store.contacts(), vec![contact(1, Scene::Dating)]);
    assert!(
        !user.join("memory").join(id(2)).exists(),
        "名单上没了的人连目录一起删"
    );
    assert_eq!(store.cards(&id(1)).len(), 1);
    let state = store.state();
    assert_eq!(state.scene, Scene::Dating, "场景以键盘写的为准");
    assert_eq!(state.contact_id, None, "当前对象被删就退回不指定");
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn snapshot_write_only_rewrites_changed_contacts() {
    let user = temp_dir("changed");
    let store = MemoryStore::open(&user);
    store.put_contact(contact(1, Scene::Dating)).unwrap();
    store.put_contact(contact(2, Scene::Dating)).unwrap();
    store
        .put_cards(&id(1), &[card(1, CardKind::Other, "a", &[], None, 1)])
        .unwrap();
    store
        .put_cards(&id(2), &[card(2, CardKind::Other, "b", &[], None, 1)])
        .unwrap();
    let mut snapshot = store.snapshot().unwrap();
    snapshot.cards.get_mut(&id(1)).unwrap()[0].text = "a2".to_owned();
    store.write_snapshot(&snapshot).unwrap();
    let after = store.snapshot().unwrap();
    assert_eq!(after.revs[&id(1)], 2);
    assert_eq!(after.revs[&id(2)], 1, "没变的对象不重写");
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn legacy_card_arrays_read_as_revision_zero() {
    let user = temp_dir("legacy");
    let store = MemoryStore::open(&user);
    store.put_contact(contact(1, Scene::Dating)).unwrap();
    let legacy = vec![card(1, CardKind::Other, "旧格式", &[], None, 1)];
    std::fs::write(
        user.join("memory").join(id(1)).join("cards.json"),
        serde_json::to_string(&legacy).unwrap(),
    )
    .unwrap();
    let snapshot = store.snapshot().unwrap();
    assert_eq!(snapshot.cards[&id(1)], legacy);
    assert_eq!(snapshot.revs[&id(1)], 0);
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn snapshot_write_validates() {
    let user = temp_dir("validate");
    let store = MemoryStore::open(&user);
    let ok = MemorySnapshot {
        contacts: vec![contact(1, Scene::Dating)],
        ..MemorySnapshot::default()
    };

    let mut unknown = ok.clone();
    unknown.cards.insert(id(9), Vec::new());
    assert!(matches!(
        store.write_snapshot(&unknown),
        Err(MemoryError::Invalid(_))
    ));

    let mut bad_date = ok.clone();
    bad_date.cards.insert(
        id(1),
        vec![card(1, CardKind::Date, "生日", &[], Some("2026-13-01"), 0)],
    );
    assert!(matches!(
        store.write_snapshot(&bad_date),
        Err(MemoryError::Invalid(_))
    ));

    let mut bad_id = ok.clone();
    bad_id.contacts[0].id = "../x".to_owned();
    assert!(matches!(
        store.write_snapshot(&bad_id),
        Err(MemoryError::Invalid(_))
    ));

    let nine = MemorySnapshot {
        contacts: (0..9).map(|n| contact(n, Scene::Dating)).collect(),
        ..MemorySnapshot::default()
    };
    assert!(matches!(
        store.write_snapshot(&nine),
        Err(MemoryError::ContactLimit(Scene::Dating))
    ));

    store.write_snapshot(&ok).unwrap();
    assert_eq!(store.contacts().len(), 1);
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn errors_have_codes_for_swift() {
    let json: serde_json::Value =
        serde_json::from_str(&MemoryError::ContactLimit(Scene::Daily).to_json()).unwrap();
    assert_eq!(json["code"], "contact_limit");
    assert_eq!(json["message"], "日常最多 8 个人");
    assert_eq!(
        MemoryError::ContactLimit(Scene::Work).message(),
        "工作最多 8 个人"
    );
    assert_eq!(MemoryError::Invalid("x").code(), "invalid");
    assert_eq!(MemoryError::Conflict.code(), "conflict");
    assert_eq!(MemoryError::Io(std::io::Error::other("x")).code(), "io");
    assert_eq!(crate::memory::new_id().unwrap().len(), 32);
}

#[test]
fn card_limits_count_characters() {
    let user = temp_dir("limits");
    let store = MemoryStore::open(&user);
    store.put_contact(contact(1, Scene::Dating)).unwrap();
    let text_ok = |text: &str| {
        store
            .put_cards(&id(1), &[card(1, CardKind::Other, text, &[], None, 1)])
            .is_ok()
    };
    assert!(text_ok(&"字".repeat(200)));
    assert!(!text_ok(&"字".repeat(201)));
    assert!(text_ok(&"😀".repeat(200)), "emoji 一个算一个字");
    assert!(!text_ok(&"😀".repeat(201)));
    assert!(text_ok(&format!("{}{}", "a".repeat(100), "中".repeat(100))));

    let keywords_ok = |keywords: &[&str]| {
        store
            .put_cards(&id(1), &[card(1, CardKind::Other, "x", keywords, None, 1)])
            .is_ok()
    };
    let eight: Vec<String> = (0..8).map(|n| format!("关键词{n}")).collect();
    let eight: Vec<&str> = eight.iter().map(String::as_str).collect();
    assert!(keywords_ok(&eight));
    let mut nine = eight.clone();
    nine.push("第九个");
    assert!(!keywords_ok(&nine), "最多 8 个");
    assert!(!keywords_ok(&["海"]), "至少 2 字");
    assert!(keywords_ok(&["八个字的关键词呀"]));
    assert!(!keywords_ok(&["九个字的关键词呀呀"]), "至多 8 字");
    assert!(!keywords_ok(&["  "]));

    assert!(matches!(
        store.add_note(&id(1), &"记".repeat(201), 2),
        Err(MemoryError::Invalid(_))
    ));
    let too_long = MemorySnapshot {
        contacts: vec![contact(1, Scene::Dating)],
        cards: [(
            id(1),
            vec![card(2, CardKind::Other, &"字".repeat(201), &[], None, 1)],
        )]
        .into_iter()
        .collect(),
        ..MemorySnapshot::default()
    };
    assert!(matches!(
        store.write_snapshot(&too_long),
        Err(MemoryError::Invalid(_))
    ));
    std::fs::remove_dir_all(&user).ok();
}
