//! 磁盘上有一张不合格的卡（旧版本或别的写入路径留下的）时：真要整份写或写这个人的卡，报错里指得出是谁的哪张卡。

use qingjian_cloud_proto::CardKind;

use super::{card, contact, id, temp_dir};
use crate::memory::{MemoryError, MemoryStore};

/// 人 1，卡片文件里写一张关键词只有 1 个字的卡（绕过校验直接落盘）。
fn store_with_bad_card(name: &str) -> (std::path::PathBuf, MemoryStore) {
    let user = temp_dir(name);
    let store = MemoryStore::open(&user);
    store.put_contact(contact(1)).unwrap();
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
fn a_bad_card_error_names_the_person_and_the_card() {
    let (user, store) = store_with_bad_card("bad-card-message");
    let snapshot = store.snapshot().unwrap();
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
