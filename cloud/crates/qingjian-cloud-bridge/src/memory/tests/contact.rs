//! `Contact` 与 `ScopeState` 去场景之后的形状：老文件里的 `scene` / `last` 读得进、写出去就没有了。

use super::{id, temp_dir};
use crate::memory::{Contact, MemoryStore};
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
