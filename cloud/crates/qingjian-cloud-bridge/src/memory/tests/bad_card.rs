//! 磁盘上有一张不合格的卡（旧版本或别的写入路径留下的）时：场景的增删改照样成功（它们不碰卡片），
//! 真要整份写或写这个人的卡时，报错里指得出是谁的哪张卡。

use std::ffi::{CStr, CString};

use qingjian_cloud_proto::CardKind;

use super::{card, contact, id, open_with_scenes, temp_dir};
use crate::memory::ffi::{qj_memory_delete_scene, qj_memory_put_scene};
use crate::memory::{DEFAULT_SCENE_ID, MemoryError, MemoryStore, Scene};
use crate::scope::ContactPick;

/// 人 1 在「work」场景，卡片文件里写一张关键词只有 1 个字的卡（绕过校验直接落盘）。
fn store_with_bad_card(name: &str) -> (std::path::PathBuf, MemoryStore) {
    let user = temp_dir(name);
    let store = open_with_scenes(&user);
    store.put_contact(contact(1, "work")).unwrap();
    let bad = card(
        1,
        CardKind::Preference,
        "周末想去看海边的日落",
        &["海"],
        None,
        1,
    );
    let json = serde_json::json!({ "rev": 3, "cards": [bad] });
    std::fs::write(
        user.join("memory").join(id(1)).join("cards.json"),
        serde_json::to_vec(&json).unwrap(),
    )
    .unwrap();
    (user, store)
}

#[test]
fn scene_edits_succeed_while_a_bad_card_sits_on_disk() {
    let (user, store) = store_with_bad_card("bad-card-scenes");

    store
        .put_scene(Scene::new("a1b2".to_owned(), "家人".to_owned(), 7))
        .unwrap();
    store
        .put_scene(Scene::new("work".to_owned(), "同事".to_owned(), 99))
        .unwrap();
    let work = store.scenes().into_iter().find(|s| s.id == "work").unwrap();
    assert_eq!(work.name, "同事", "改名成了");
    assert_eq!(work.created_at, 0, "改名不动建的时间");

    store.delete_scene("work").unwrap();
    assert_eq!(
        store.contacts()[0].scene,
        DEFAULT_SCENE_ID,
        "人挪到默认场景"
    );
    assert_eq!(store.try_cards(&id(1)).unwrap().len(), 1, "坏卡原样留着");
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn scene_ffi_skips_card_validation() {
    let (user, store) = store_with_bad_card("bad-card-ffi");
    let dir = CString::new(user.to_str().unwrap()).unwrap();
    let scene = CString::new("work").unwrap();
    let name = CString::new("同事").unwrap();
    let renamed = unsafe { qj_memory_put_scene(dir.as_ptr(), scene.as_ptr(), name.as_ptr()) };
    assert!(renamed.is_null(), "改名成功返回空指针");
    let deleted = unsafe { qj_memory_delete_scene(dir.as_ptr(), scene.as_ptr()) };
    assert!(deleted.is_null(), "删场景成功返回空指针");
    assert!(store.scenes().iter().all(|s| s.id != "work"));

    let bad_id = CString::new("../x").unwrap();
    let refused = unsafe { qj_memory_put_scene(dir.as_ptr(), bad_id.as_ptr(), name.as_ptr()) };
    let text = unsafe { CStr::from_ptr(refused) }
        .to_str()
        .unwrap()
        .to_owned();
    unsafe { crate::qj_string_free(refused) };
    assert!(text.contains("\"invalid\""), "{text}");
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn deleting_the_current_scene_moves_the_keyboard_state() {
    let (user, store) = store_with_bad_card("bad-card-state");
    store
        .update_scope("work", &ContactPick::Contact(id(1)), 5)
        .unwrap();
    store.delete_scene("work").unwrap();
    let state = store.state();
    assert_eq!(state.scene, DEFAULT_SCENE_ID);
    assert_eq!(
        state.contact_id, None,
        "人换了场景，当前对象按场景对不上处理"
    );
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn a_bad_card_error_names_the_person_and_the_card() {
    let (user, store) = store_with_bad_card("bad-card-message");
    let mut snapshot = store.snapshot().unwrap();
    snapshot.scenes[0].name = "家里".to_owned();
    let error = store.write_snapshot(&snapshot).unwrap_err();
    assert_eq!(error.code(), "invalid");
    assert_eq!(
        error.message(),
        "人1的卡「周末想去看海边的…」：每个关键词要 2 到 8 个字"
    );

    let short = card(2, CardKind::Other, "爱吃辣", &["辣"], None, 1);
    let error = store.put_cards(&id(1), &[short]).unwrap_err();
    assert!(matches!(
        error,
        MemoryError::InvalidCard { ref contact, ref card, reason: "每个关键词要 2 到 8 个字" }
            if contact == "人1" && card == "爱吃辣"
    ));
    assert!(error.to_json().contains("人1的卡「爱吃辣」"));
    std::fs::remove_dir_all(&user).ok();
}
