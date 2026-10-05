//! `Contact` 与 `ScopeState` 去场景之后的形状：老文件里的 `scene` / `last` 读得进、写出去就没有了。

use super::{id, temp_dir};
use crate::memory::{Contact, MAX_PINNED, MemoryError, MemoryStore};
use crate::scope::ScopeState;

#[test]
fn an_old_contact_with_a_scene_reads_and_writes_without_it() {
    let user = temp_dir("contact-no-scene");
    let store = MemoryStore::open(&user);
    store.put_contact(super::contact(1)).unwrap();

    let path = user.join("memory").join("contacts.json");
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(!text.contains("\"scene\""), "写出去不该再有 scene：{text}");

    // 老文件（带 scene）照旧读得进来
    std::fs::write(
        &path,
        format!(
            r#"[{{"id":"{}","name":"小美","scene":"dating","created_at":1}}]"#,
            id(1)
        ),
    )
    .unwrap();
    let read: Vec<Contact> = store.contacts();
    assert_eq!(read.len(), 1);
    assert_eq!(read[0].name, "小美");
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn an_old_state_with_scene_and_last_reads_without_them() {
    let user = temp_dir("state-no-scene");
    let store = MemoryStore::open(&user);
    std::fs::create_dir_all(user.join("memory")).unwrap();
    std::fs::write(
        user.join("memory").join("state.json"),
        format!(
            r#"{{"scene":"dating","contact_id":"{}","last":{{"dating":"{}"}}}}"#,
            id(1),
            id(1)
        ),
    )
    .unwrap();
    let state: ScopeState = store.state();
    assert_eq!(
        state.contact_id,
        Some(id(1)),
        "多出来的 scene / last 忽略，contact_id 留着"
    );
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn pinning_is_counted_globally() {
    let user = temp_dir("pin-global");
    let store = MemoryStore::open(&user);
    for n in 0..MAX_PINNED as u32 {
        let mut who = super::contact(n);
        who.pinned_at = Some(i64::from(n));
        store.put_contact(who).unwrap();
    }
    let mut fifth = super::contact(9);
    fifth.pinned_at = Some(9);
    assert!(matches!(
        store.put_contact(fifth),
        Err(MemoryError::PinLimit)
    ));
    let mut sixth = super::contact(8);
    sixth.pinned_at = Some(8);
    assert_eq!(
        store.put_contact(sixth).unwrap_err().message(),
        "最多置顶 4 个人"
    );
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn the_snapshot_has_no_scenes() {
    let user = temp_dir("snapshot-no-scenes");
    let store = MemoryStore::open(&user);
    store.put_contact(super::contact(1)).unwrap();
    let json = serde_json::to_string(&store.snapshot().unwrap()).unwrap();
    assert!(
        !json.contains("scenes"),
        "整份读写的 JSON 里不该再有 scenes：{json}"
    );
    std::fs::remove_dir_all(&user).ok();
}
