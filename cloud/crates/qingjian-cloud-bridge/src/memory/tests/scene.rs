//! 每个场景各一组人：各自 8 个互不影响、建好后不能换场景、各场景记住上次选的人（含旧 state.json 回填、忘掉的人清掉、场景对不上时退回不指定）、
//! 各人上次被选中的时间。

use qingjian_cloud_proto::Scene;

use super::{contact, id, pick, temp_dir};
use crate::memory::{MAX_CONTACTS, MemoryError, MemorySnapshot, MemoryStore, sanitized_scope};
use crate::scope::{ContactPick, ScopeState};

#[test]
fn each_scene_holds_eight_independently() {
    let user = temp_dir("scene-limit");
    let store = MemoryStore::open(&user);
    let scenes = [Scene::Dating, Scene::Daily, Scene::Work];
    let mut n = 0;
    for scene in scenes {
        for _ in 0..MAX_CONTACTS {
            store.put_contact(contact(n, scene)).unwrap();
            n += 1;
        }
    }
    assert_eq!(store.contacts().len(), 3 * MAX_CONTACTS);
    for scene in scenes {
        assert!(
            matches!(
                store.put_contact(contact(n, scene)),
                Err(MemoryError::ContactLimit(full)) if full == scene
            ),
            "{scene:?} 第 9 个"
        );
        n += 1;
    }
    let error = store.put_contact(contact(99, Scene::Work)).unwrap_err();
    assert_eq!(error.message(), "工作最多 8 个人");

    let mut snapshot = store.snapshot().unwrap();
    snapshot.contacts.push(contact(100, Scene::Daily));
    assert!(matches!(
        store.write_snapshot(&snapshot),
        Err(MemoryError::ContactLimit(Scene::Daily))
    ));
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn a_contact_keeps_its_scene() {
    let user = temp_dir("scene-kept");
    let store = MemoryStore::open(&user);
    store.put_contact(contact(1, Scene::Daily)).unwrap();
    let moved = contact(1, Scene::Dating);
    assert!(matches!(
        store.put_contact(moved.clone()),
        Err(MemoryError::Invalid("换场景需要忘掉后重新加"))
    ));
    let snapshot = MemorySnapshot {
        contacts: vec![moved],
        ..MemorySnapshot::default()
    };
    assert!(matches!(
        store.write_snapshot(&snapshot),
        Err(MemoryError::Invalid(_))
    ));
    assert_eq!(store.contacts(), vec![contact(1, Scene::Daily)]);
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn switching_scenes_returns_to_the_last_pick() {
    let user = temp_dir("scene-last");
    let store = MemoryStore::open(&user);
    store.put_contact(contact(1, Scene::Dating)).unwrap();
    store.put_contact(contact(2, Scene::Daily)).unwrap();
    store.put_contact(contact(3, Scene::Daily)).unwrap();

    store.update_scope(Scene::Dating, &pick(1), 0).unwrap();
    store.update_scope(Scene::Daily, &pick(3), 0).unwrap();
    let state = store
        .update_scope(Scene::Work, &ContactPick::Last, 0)
        .unwrap();
    assert_eq!(state.contact_id, None, "工作还没选过人");
    let state = store
        .update_scope(Scene::Dating, &ContactPick::Last, 0)
        .unwrap();
    assert_eq!(state.contact_id, Some(id(1)), "回到恋爱上次选的人");
    let state = store
        .update_scope(Scene::Daily, &ContactPick::Last, 0)
        .unwrap();
    assert_eq!(state.contact_id, Some(id(3)), "回到日常上次选的人");
    assert_eq!(state.last.get(&Scene::Dating), Some(&id(1)));
    assert_eq!(state.last.get(&Scene::Daily), Some(&id(3)));
    assert_eq!(store.state(), state, "last 写进了 state.json");

    // 明确不指定也记下来：切走再回来还是不指定，别的场景不受影响
    store
        .update_scope(Scene::Daily, &ContactPick::Nobody, 0)
        .unwrap();
    store
        .update_scope(Scene::Dating, &ContactPick::Last, 0)
        .unwrap();
    let state = store
        .update_scope(Scene::Daily, &ContactPick::Last, 0)
        .unwrap();
    assert_eq!(state.contact_id, None);
    assert_eq!(state.last.get(&Scene::Dating), Some(&id(1)));
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn old_state_file_backfills_the_dating_contact() {
    let user = temp_dir("scene-backfill");
    let store = MemoryStore::open(&user);
    store.put_contact(contact(1, Scene::Dating)).unwrap();
    let old = format!(r#"{{"scene":"dating","contact_id":"{}"}}"#, id(1));
    std::fs::write(user.join("memory").join("state.json"), old).unwrap();
    let state = store.state();
    assert!(state.last.is_empty(), "旧文件没有 last");
    let state = sanitized_scope(state, &store.contacts());
    assert_eq!(state.last.get(&Scene::Dating), Some(&id(1)), "读时补上");

    // 旧文件直接切走再切回来，也回到原来的人
    std::fs::write(
        user.join("memory").join("state.json"),
        format!(r#"{{"scene":"dating","contact_id":"{}"}}"#, id(1)),
    )
    .unwrap();
    store
        .update_scope(Scene::Work, &ContactPick::Last, 0)
        .unwrap();
    let state = store
        .update_scope(Scene::Dating, &ContactPick::Last, 0)
        .unwrap();
    assert_eq!(state.contact_id, Some(id(1)));
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn forgotten_contacts_leave_last() {
    let user = temp_dir("scene-forget");
    let store = MemoryStore::open(&user);
    store.put_contact(contact(1, Scene::Dating)).unwrap();
    store.put_contact(contact(2, Scene::Daily)).unwrap();
    store.update_scope(Scene::Dating, &pick(1), 0).unwrap();
    store.update_scope(Scene::Daily, &pick(2), 0).unwrap();

    // App 忘掉恋爱的那个人：整份写回时 state 跟着理顺
    let mut snapshot = store.snapshot().unwrap();
    snapshot.contacts.retain(|c| c.id != id(1));
    snapshot.cards.remove(&id(1));
    store.write_snapshot(&snapshot).unwrap();
    let state = store.state();
    assert_eq!(state.last.get(&Scene::Dating), None, "忘掉的人从 last 清掉");
    assert_eq!(state.contact_id, Some(id(2)), "当前日常的人不受影响");

    // 键盘这边：名单上没了的人，切场景时也不会回到它
    store.forget_contact(&id(2)).unwrap();
    let state = store
        .update_scope(Scene::Daily, &ContactPick::Last, 0)
        .unwrap();
    assert_eq!(state.contact_id, None);
    assert!(state.last.is_empty());
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn a_contact_from_another_scene_is_sanitized() {
    let contacts = vec![contact(1, Scene::Dating), contact(2, Scene::Work)];
    let state = ScopeState {
        scene: Scene::Daily,
        contact_id: Some(id(1)),
        last: [(Scene::Daily, id(2)), (Scene::Work, id(2))].into(),
        used: [(id(2), 1), (id(9), 2)].into(),
    };
    let state = sanitized_scope(state, &contacts);
    assert_eq!(state.contact_id, None, "恋爱的人不能在日常里选");
    assert_eq!(
        state.last,
        [(Scene::Work, id(2))].into(),
        "last 里场景对不上的也去掉"
    );
    assert_eq!(state.used, [(id(2), 1)].into(), "used 里不在名单上的去掉");

    let user = temp_dir("scene-mismatch");
    let store = MemoryStore::open(&user);
    store.put_contact(contact(1, Scene::Dating)).unwrap();
    let state = store.update_scope(Scene::Work, &pick(1), 0).unwrap();
    assert_eq!(state.contact_id, None);
    assert_eq!(state.scene, Scene::Work);
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn picking_someone_records_when_and_forgetting_clears_it() {
    let user = temp_dir("scene-used");
    let store = MemoryStore::open(&user);
    store.put_contact(contact(1, Scene::Dating)).unwrap();
    store.put_contact(contact(2, Scene::Daily)).unwrap();
    store.update_scope(Scene::Dating, &pick(1), 100).unwrap();
    store.update_scope(Scene::Daily, &pick(2), 200).unwrap();
    let state = store
        .update_scope(Scene::Dating, &ContactPick::Last, 300)
        .unwrap();
    assert_eq!(state.used.get(&id(1)), Some(&300), "切回上次的人也算选中");
    assert_eq!(state.used.get(&id(2)), Some(&200));
    let state = store
        .update_scope(Scene::Dating, &ContactPick::Nobody, 400)
        .unwrap();
    assert_eq!(state.used.get(&id(1)), Some(&300), "不指定不记时间");

    let old: ScopeState = serde_json::from_str(r#"{"scene":"daily"}"#).unwrap();
    assert!(old.used.is_empty(), "旧文件没有 used");

    store.forget_contact(&id(1)).unwrap();
    let state = store
        .update_scope(Scene::Daily, &ContactPick::Last, 500)
        .unwrap();
    assert_eq!(state.used.get(&id(1)), None, "忘掉的人一并清掉");
    assert_eq!(state.used.get(&id(2)), Some(&500));
    std::fs::remove_dir_all(&user).ok();
}
