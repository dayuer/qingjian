//! 上一版的场景数据怎么清：`scenes.json` 与没改过名的 `scene-*/` 删掉，
//! 但 `scene-*.migrated-<日期>` 是上一版承诺保留 30 天的备份，不能顺手删。

use super::{id, temp_dir};
use crate::memory::{LocalDate, MemoryStore};

/// 造一份上一版留下的数据：场景文件、场景目录、改名过的备份、带 scene 字段的人与状态。
fn seed_legacy(user: &std::path::Path) {
    let memory = user.join("memory");
    std::fs::create_dir_all(memory.join("scene-dating").join("learning")).unwrap();
    std::fs::create_dir_all(
        memory
            .join("scene-dating.migrated-2026-10-05")
            .join("learning"),
    )
    .unwrap();
    std::fs::write(
        memory
            .join("scene-dating.migrated-2026-10-05")
            .join("learning")
            .join("user.tsv"),
        "宝贝\t10\n",
    )
    .unwrap();
    std::fs::write(
        memory.join("scenes.json"),
        r#"[{"id":"daily","name":"日常","created_at":1}]"#,
    )
    .unwrap();
    std::fs::write(
        memory.join("contacts.json"),
        format!(
            r#"[{{"id":"{}","name":"小美","scene":"dating","created_at":1}}]"#,
            id(1)
        ),
    )
    .unwrap();
    std::fs::create_dir_all(memory.join(id(1))).unwrap();
    std::fs::write(
        memory.join(id(1)).join("cards.json"),
        r#"{"rev":1,"cards":[]}"#,
    )
    .unwrap();
    std::fs::write(
        memory.join("state.json"),
        format!(
            r#"{{"scene":"dating","contact_id":"{}","last":{{"dating":"{}"}}}}"#,
            id(1),
            id(1)
        ),
    )
    .unwrap();
}

#[test]
fn clearing_scenes_keeps_the_migrated_backup() {
    let user = temp_dir("clear-scenes");
    seed_legacy(&user);
    let store = MemoryStore::open(&user);
    let _ = store.contacts(); // 任何一次读写都会走一遍清场

    let memory = user.join("memory");
    assert!(!memory.join("scenes.json").exists(), "场景文件删掉");
    assert!(
        !memory.join("scene-dating").exists(),
        "没改过名的场景目录删掉"
    );
    let backup = memory.join("scene-dating.migrated-2026-10-05");
    assert!(
        backup.join("learning").join("user.tsv").is_file(),
        "改名过的备份留着：{backup:?}"
    );

    assert_eq!(store.contacts().len(), 1, "人一个不少");
    let state = store.state();
    assert_eq!(state.contact_id, Some(id(1)), "当前对象留着");
    let json = std::fs::read_to_string(memory.join("state.json")).unwrap();
    assert!(!json.contains("\"scene\""), "state.json 里不再写场景");
    assert!(!json.contains("\"last\""), "state.json 里不再写 last");
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn an_expired_migrated_backup_is_swept() {
    let user = temp_dir("sweep-migrated");
    let memory = user.join("memory");
    std::fs::create_dir_all(memory.join("scene-dating.migrated-2026-08-01")).unwrap();
    let store = MemoryStore::open(&user);
    store.sweep_migrated_dirs(LocalDate::today());
    assert!(
        !memory.join("scene-dating.migrated-2026-08-01").exists(),
        "改名满 30 天的备份清掉"
    );
    std::fs::remove_dir_all(&user).ok();
}
