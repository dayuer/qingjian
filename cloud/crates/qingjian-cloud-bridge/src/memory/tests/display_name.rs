//! 键盘上的代号 `display_name`：旧文件兼容、写盘前去空白、超长拒绝、键盘称呼的回退。

use super::{contact, open_with_scenes, temp_dir};
use crate::memory::{Contact, MemoryError, MemorySnapshot};

#[test]
fn old_contacts_without_display_name_read_as_none_and_stay_absent() {
    let old = r#"{"id":"00000000000000000000000000000001","name":"小美","pronoun":"ta_f","scene":"dating","created_at":1791043200}"#;
    let contact: Contact = serde_json::from_str(old).unwrap();
    assert_eq!(contact.display_name, None);
    assert_eq!(contact.chip_name(), "小美");
    let written = serde_json::to_string(&contact).unwrap();
    assert!(
        !written.contains("display_name"),
        "没有代号时不写这个字段：{written}"
    );
}

#[test]
fn display_name_round_trips_through_the_store() {
    let user = temp_dir("display-round-trip");
    let store = open_with_scenes(&user);
    let mut named = contact(1);
    named.display_name = Some(" 阿美 ".to_owned());
    store
        .write_snapshot(&MemorySnapshot {
            scenes: store.scenes(),
            contacts: vec![named],
            ..MemorySnapshot::default()
        })
        .unwrap();
    let read = store.snapshot().unwrap();
    assert_eq!(
        read.contacts[0].display_name.as_deref(),
        Some("阿美"),
        "写盘前去掉首尾空白"
    );
    store.write_snapshot(&read).unwrap();
    assert_eq!(
        store.contacts()[0].display_name.as_deref(),
        Some("阿美"),
        "再写一趟不丢"
    );
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn blank_display_name_counts_as_none() {
    let user = temp_dir("display-blank");
    let store = open_with_scenes(&user);
    let mut blank = contact(1);
    blank.display_name = Some(" \u{3000} ".to_owned());
    assert_eq!(blank.chip_name(), "人1", "全是空白时键盘显示名字");
    store.put_contact(blank).unwrap();
    assert_eq!(store.contacts()[0].display_name, None);
    let file = std::fs::read_to_string(user.join("memory/contacts.json")).unwrap();
    assert!(!file.contains("display_name"), "{file}");
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn display_name_over_twelve_chars_is_rejected() {
    let user = temp_dir("display-long");
    let store = open_with_scenes(&user);
    let mut ok = contact(1);
    ok.display_name = Some("一二三四五六七八九十一二".to_owned());
    store.put_contact(ok).unwrap();
    let mut long = contact(2);
    long.display_name = Some("一二三四五六七八九十一二三".to_owned());
    assert!(matches!(
        store.put_contact(long.clone()),
        Err(MemoryError::Invalid(_))
    ));
    assert!(matches!(
        store.write_snapshot(&MemorySnapshot {
            contacts: vec![long],
            ..MemorySnapshot::default()
        }),
        Err(MemoryError::Invalid(_))
    ));
    assert_eq!(store.contacts().len(), 1, "被拒的没写进去");
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn chip_name_prefers_display_name() {
    let mut person = contact(1);
    assert_eq!(person.chip_name(), "人1");
    person.display_name = Some("阿美".to_owned());
    assert_eq!(person.chip_name(), "阿美");
}
