//! 场景是用户自建的分组：增删改名、删场景时人挪到默认场景、人数不限、每场景最多 4 个置顶、人换场景不受限；
//! 另外各场景记住上次选的人（含旧 state.json 回填、忘掉的人清掉、场景对不上时退回不指定）与各人上次被选中的时间。
//! 老数据（写死的三个场景、各场景单独学一层）的迁移见文件末尾。

use super::{contact, id, open_with_scenes, pick, temp_dir};
use crate::memory::{
    DEFAULT_SCENE_ID, MAX_PINNED, MemoryError, MemorySnapshot, MemoryStore, Scene, sanitized_scope,
    validate_scenes,
};
use crate::scope::{ContactPick, ScopeState};

#[test]
fn scenes_are_created_renamed_and_deleted() {
    let user = temp_dir("scene-crud");
    let store = MemoryStore::open(&user);
    let fresh = store.scenes();
    assert_eq!(fresh.len(), 1, "新装只有一个场景");
    assert_eq!(fresh[0].id, DEFAULT_SCENE_ID);
    assert_eq!(fresh[0].name, "日常");

    store
        .put_scene(Scene::new("a1b2".to_owned(), "家人".to_owned(), 7))
        .unwrap();
    store
        .put_scene(Scene::new("a1b2".to_owned(), "自己人".to_owned(), 7))
        .unwrap();
    assert_eq!(
        store
            .scenes()
            .iter()
            .map(|s| s.name.as_str())
            .collect::<Vec<_>>(),
        ["日常", "自己人"],
        "同 id 是改名，不新增"
    );

    store.delete_scene("a1b2").unwrap();
    assert_eq!(store.scenes().len(), 1);
    assert!(matches!(
        store.delete_scene(DEFAULT_SCENE_ID),
        Err(MemoryError::Invalid("至少要留一个场景"))
    ));
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn deleting_a_scene_moves_its_people_to_the_default_one() {
    let user = temp_dir("scene-delete");
    let store = open_with_scenes(&user);
    store.put_contact(contact(1, "work")).unwrap();
    store.put_contact(contact(2, "work")).unwrap();
    store.put_contact(contact(3, "daily")).unwrap();

    store.delete_scene("work").unwrap();
    let contacts = store.contacts();
    assert_eq!(contacts.len(), 3, "人一个不少");
    assert!(
        contacts.iter().all(|c| c.scene == DEFAULT_SCENE_ID),
        "都挪到默认场景"
    );
    assert!(store.scenes().iter().all(|s| s.id != "work"));
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn a_scene_holds_any_number_of_people() {
    let user = temp_dir("scene-unlimited");
    let store = open_with_scenes(&user);
    for n in 0..20 {
        store.put_contact(contact(n, "daily")).unwrap();
    }
    assert_eq!(store.contacts().len(), 20);
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn a_scene_holds_four_pinned_at_most() {
    let user = temp_dir("scene-pin");
    let store = open_with_scenes(&user);
    for n in 0..MAX_PINNED as u32 {
        let mut who = contact(n, "daily");
        who.pinned_at = Some(i64::from(n));
        store.put_contact(who).unwrap();
    }
    let mut fifth = contact(9, "daily");
    fifth.pinned_at = Some(9);
    assert!(matches!(
        store.put_contact(fifth.clone()),
        Err(MemoryError::PinLimit)
    ));
    assert_eq!(
        store.put_contact(fifth).unwrap_err().message(),
        "一个场景最多置顶 4 个人"
    );

    // 别的场景各自算
    let mut other = contact(10, "work");
    other.pinned_at = Some(1);
    store.put_contact(other).unwrap();

    // 取消一个置顶后能再置顶
    let mut first = contact(0, "daily");
    first.pinned_at = None;
    store.put_contact(first).unwrap();
    let mut fifth = contact(9, "daily");
    fifth.pinned_at = Some(9);
    store.put_contact(fifth).unwrap();
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn a_contact_may_change_its_scene() {
    let user = temp_dir("scene-moved");
    let store = open_with_scenes(&user);
    store.put_contact(contact(1, "daily")).unwrap();
    let moved = contact(1, "dating");
    store.put_contact(moved.clone()).unwrap();
    assert_eq!(store.contacts(), vec![moved], "换场景不受限了");

    // 场景名册上没有的分组（旧键盘写回的老 id）归到默认场景，不报错
    store.put_contact(contact(2, "nope")).unwrap();
    assert_eq!(store.contacts()[1].scene, "daily");
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn scenes_must_be_valid_and_are_written_back_whole() {
    let user = temp_dir("scene-validate");
    let store = open_with_scenes(&user);
    assert!(matches!(
        validate_scenes(&[]),
        Err(MemoryError::Invalid("至少要留一个场景"))
    ));
    assert!(matches!(
        store.write_snapshot(&MemorySnapshot::default()),
        Err(MemoryError::Invalid("至少要留一个场景"))
    ));

    store.put_contact(contact(1, "dating")).unwrap();
    let mut snapshot = store.snapshot().unwrap();
    assert_eq!(snapshot.scenes.len(), 3, "整份读要带上场景");
    snapshot.scenes = vec![Scene::new("only".to_owned(), "只剩我".to_owned(), 1)];
    store.write_snapshot(&snapshot).unwrap();
    assert_eq!(
        store.contacts()[0].scene,
        "only",
        "场景名册上没有了就归到默认场景"
    );

    snapshot.contacts.clear();
    snapshot.cards.clear();
    snapshot.revs.clear();
    store.write_snapshot(&snapshot).unwrap();
    assert_eq!(store.scenes().len(), 1, "整份写回会换掉场景");
    assert_eq!(store.scenes()[0].name, "只剩我");
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn an_empty_or_long_scene_name_is_rejected() {
    let user = temp_dir("scene-name");
    let store = open_with_scenes(&user);
    assert!(matches!(
        store.put_scene(Scene::new("x1".to_owned(), "  ".to_owned(), 0)),
        Err(MemoryError::Invalid("场景名不能是空的"))
    ));
    assert!(matches!(
        store.put_scene(Scene::new(
            "x1".to_owned(),
            "一二三四五六七八九".to_owned(),
            0
        )),
        Err(MemoryError::Invalid("场景名最多 8 个字"))
    ));
    assert!(matches!(
        store.put_scene(Scene::new("../oops".to_owned(), "歪".to_owned(), 0)),
        Err(MemoryError::Invalid("场景编号不对"))
    ));
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn a_fresh_install_gets_one_daily_scene() {
    let user = temp_dir("scene-migrate-fresh");
    let store = MemoryStore::open(&user);
    assert_eq!(store.contacts(), Vec::new());
    let scenes = store.scenes();
    assert_eq!(scenes.len(), 1);
    assert_eq!(scenes[0].id, DEFAULT_SCENE_ID);
    assert_eq!(scenes[0].name, "日常");
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn legacy_data_is_merged_into_one_daily_scene() {
    let user = temp_dir("scene-migrate");
    let memory = user.join("memory");
    std::fs::create_dir_all(memory.join("scene-dating").join("learning")).unwrap();
    std::fs::write(
        memory.join("contacts.json"),
        format!(
            r#"[{{"id":"{}","name":"小美","scene":"dating","created_at":1}},
                {{"id":"{}","name":"妈妈","scene":"daily","created_at":1}},
                {{"id":"{}","name":"老周","scene":"work","created_at":1}}]"#,
            id(1),
            id(2),
            id(3)
        ),
    )
    .unwrap();
    std::fs::write(
        memory.join("state.json"),
        format!(
            r#"{{"scene":"dating","contact_id":"{}","last":{{"dating":"{}","daily":"{}"}}}}"#,
            id(1),
            id(1),
            id(2)
        ),
    )
    .unwrap();

    let store = MemoryStore::open(&user);
    let snapshot = store.snapshot().unwrap();
    assert_eq!(snapshot.scenes.len(), 1, "三个并成一个");
    assert_eq!(snapshot.scenes[0].name, "日常");
    assert_eq!(snapshot.contacts.len(), 3, "人一个不少");
    assert!(
        snapshot
            .contacts
            .iter()
            .all(|c| c.scene == DEFAULT_SCENE_ID),
        "都归到「日常」"
    );
    let parked: Vec<String> = std::fs::read_dir(&memory)
        .unwrap()
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| name.starts_with("scene-dating.migrated-"))
        .collect();
    assert_eq!(parked.len(), 1, "老的场景学习目录改名留着：{parked:?}");
    assert!(!memory.join("scene-dating").exists(), "原名不再有");
    assert!(
        memory.join(&parked[0]).join("learning").is_dir(),
        "里面学过的东西还在，过 30 天才清"
    );
    assert_eq!(snapshot.state.scene, DEFAULT_SCENE_ID);
    assert_eq!(
        snapshot.state.last.keys().collect::<Vec<_>>(),
        [DEFAULT_SCENE_ID]
    );

    // 迁移只做一次：再读还是这些场景，不会被并第二遍
    let again = MemoryStore::open(&user);
    assert_eq!(again.scenes(), store.scenes());
    std::fs::remove_dir_all(&user).ok();
}
#[test]
fn switching_scenes_returns_to_the_last_pick() {
    let user = temp_dir("scene-last");
    let store = open_with_scenes(&user);
    store.put_contact(contact(1, "dating")).unwrap();
    store.put_contact(contact(2, "daily")).unwrap();
    store.put_contact(contact(3, "daily")).unwrap();

    store.update_scope("dating", &pick(1), 0).unwrap();
    store.update_scope("daily", &pick(3), 0).unwrap();
    let state = store.update_scope("work", &ContactPick::Last, 0).unwrap();
    assert_eq!(state.contact_id, None, "工作还没选过人");
    let state = store.update_scope("dating", &ContactPick::Last, 0).unwrap();
    assert_eq!(state.contact_id, Some(id(1)), "回到恋爱上次选的人");
    let state = store.update_scope("daily", &ContactPick::Last, 0).unwrap();
    assert_eq!(state.contact_id, Some(id(3)), "回到日常上次选的人");
    assert_eq!(state.last.get("dating"), Some(&id(1)));
    assert_eq!(state.last.get("daily"), Some(&id(3)));
    assert_eq!(store.state(), state, "last 写进了 state.json");

    // 明确不指定也记下来：切走再回来还是不指定，别的场景不受影响
    store
        .update_scope("daily", &ContactPick::Nobody, 0)
        .unwrap();
    store.update_scope("dating", &ContactPick::Last, 0).unwrap();
    let state = store.update_scope("daily", &ContactPick::Last, 0).unwrap();
    assert_eq!(state.contact_id, None);
    assert_eq!(state.last.get("dating"), Some(&id(1)));
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn old_state_file_backfills_the_dating_contact() {
    let user = temp_dir("scene-backfill");
    let store = open_with_scenes(&user);
    store.put_contact(contact(1, "dating")).unwrap();
    let old = format!(r#"{{"scene":"dating","contact_id":"{}"}}"#, id(1));
    std::fs::write(user.join("memory").join("state.json"), old).unwrap();
    let state = store.state();
    assert!(state.last.is_empty(), "旧文件没有 last");
    let state = sanitized_scope(state, &store.contacts());
    assert_eq!(state.last.get("dating"), Some(&id(1)), "读时补上");

    // 旧文件直接切走再切回来，也回到原来的人
    std::fs::write(
        user.join("memory").join("state.json"),
        format!(r#"{{"scene":"dating","contact_id":"{}"}}"#, id(1)),
    )
    .unwrap();
    store.update_scope("work", &ContactPick::Last, 0).unwrap();
    let state = store.update_scope("dating", &ContactPick::Last, 0).unwrap();
    assert_eq!(state.contact_id, Some(id(1)));
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn forgotten_contacts_leave_last() {
    let user = temp_dir("scene-forget");
    let store = open_with_scenes(&user);
    store.put_contact(contact(1, "dating")).unwrap();
    store.put_contact(contact(2, "daily")).unwrap();
    store.update_scope("dating", &pick(1), 0).unwrap();
    store.update_scope("daily", &pick(2), 0).unwrap();

    // App 忘掉恋爱的那个人：整份写回时 state 跟着理顺
    let mut snapshot = store.snapshot().unwrap();
    snapshot.contacts.retain(|c| c.id != id(1));
    snapshot.cards.remove(&id(1));
    store.write_snapshot(&snapshot).unwrap();
    let state = store.state();
    assert_eq!(state.last.get("dating"), None, "忘掉的人从 last 清掉");
    assert_eq!(state.contact_id, Some(id(2)), "当前日常的人不受影响");

    // 键盘这边：名单上没了的人，切场景时也不会回到它
    store.forget_contact(&id(2)).unwrap();
    let state = store.update_scope("daily", &ContactPick::Last, 0).unwrap();
    assert_eq!(state.contact_id, None);
    assert!(state.last.is_empty());
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn a_contact_from_another_scene_is_sanitized() {
    let contacts = vec![contact(1, "dating"), contact(2, "work")];
    let state = ScopeState {
        scene: "daily".to_owned(),
        contact_id: Some(id(1)),
        last: [("daily".to_owned(), id(2)), ("work".to_owned(), id(2))].into(),
        used: [(id(2), 1), (id(9), 2)].into(),
    };
    let state = sanitized_scope(state, &contacts);
    assert_eq!(state.contact_id, None, "恋爱的人不能在日常里选");
    assert_eq!(
        state.last,
        [("work".to_owned(), id(2))].into(),
        "last 里场景对不上的也去掉"
    );
    assert_eq!(state.used, [(id(2), 1)].into(), "used 里不在名单上的去掉");

    let user = temp_dir("scene-mismatch");
    let store = open_with_scenes(&user);
    store.put_contact(contact(1, "dating")).unwrap();
    let state = store.update_scope("work", &pick(1), 0).unwrap();
    assert_eq!(state.contact_id, None);
    assert_eq!(state.scene, "work");
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn picking_someone_records_when_and_forgetting_clears_it() {
    let user = temp_dir("scene-used");
    let store = open_with_scenes(&user);
    store.put_contact(contact(1, "dating")).unwrap();
    store.put_contact(contact(2, "daily")).unwrap();
    store.update_scope("dating", &pick(1), 100).unwrap();
    store.update_scope("daily", &pick(2), 200).unwrap();
    let state = store
        .update_scope("dating", &ContactPick::Last, 300)
        .unwrap();
    assert_eq!(state.used.get(&id(1)), Some(&300), "切回上次的人也算选中");
    assert_eq!(state.used.get(&id(2)), Some(&200));
    let state = store
        .update_scope("dating", &ContactPick::Nobody, 400)
        .unwrap();
    assert_eq!(state.used.get(&id(1)), Some(&300), "不指定不记时间");

    let old: ScopeState = serde_json::from_str(r#"{"scene":"daily"}"#).unwrap();
    assert!(old.used.is_empty(), "旧文件没有 used");

    store.forget_contact(&id(1)).unwrap();
    let state = store
        .update_scope("daily", &ContactPick::Last, 500)
        .unwrap();
    assert_eq!(state.used.get(&id(1)), None, "忘掉的人一并清掉");
    assert_eq!(state.used.get(&id(2)), Some(&500));
    std::fs::remove_dir_all(&user).ok();
}
