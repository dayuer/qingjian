//! 键盘待办队列：「记一笔」按顺序、有上限、落盘，切场景只留最新一次。拿不到文件锁的端到端行为见 `tests/memory_ffi.rs`。

use qingjian_cloud_proto::Scene;

use std::path::PathBuf;

use super::pending::{MAX_PENDING_NOTES, PENDING_FILE, PendingNote, PendingWrites};

fn note(n: usize) -> PendingNote {
    PendingNote {
        contact_id: "0123456789abcdef0123456789abcdef".to_owned(),
        text: format!("第 {n} 条"),
        at: i64::try_from(n).unwrap(),
    }
}

#[test]
fn pending_notes_keep_order_and_drop_the_oldest_over_the_cap() {
    let mut pending = PendingWrites::default();
    assert!(pending.is_empty());
    for n in 0..MAX_PENDING_NOTES + 5 {
        pending.push_note(note(n));
    }
    assert_eq!(pending.note_count(), MAX_PENDING_NOTES);
    assert_eq!(
        pending.front_note().unwrap().text,
        "第 5 条",
        "丢的是最旧的 5 条"
    );
    let mut texts = Vec::new();
    while let Some(next) = pending.pop_note() {
        texts.push(next.at);
    }
    let expected: Vec<i64> = (5..i64::try_from(MAX_PENDING_NOTES).unwrap() + 5).collect();
    assert_eq!(texts, expected);
    assert!(pending.is_empty());
}

#[test]
fn pending_scope_keeps_only_the_latest() {
    let mut pending = PendingWrites::default();
    pending.set_scope(Scene::Work, None);
    pending.set_scope(Scene::Dating, Some("a".repeat(32)));
    assert!(pending.has_scope());
    assert_eq!(
        pending.take_scope(),
        Some((Scene::Dating, Some("a".repeat(32))))
    );
    assert!(!pending.has_scope());

    // 重试失败放回时，不压过期间新来的一次
    pending.set_scope(Scene::Daily, None);
    pending.restore_scope((Scene::Work, None));
    assert_eq!(pending.take_scope(), Some((Scene::Daily, None)));
}

fn memory_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("qj-pending-{name}-{}", std::process::id()));
    std::fs::remove_dir_all(&dir).ok();
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn pending_notes_survive_a_restart_and_the_file_follows_the_queue() {
    let dir = memory_dir("restart");
    let mut pending = PendingWrites::open(&dir);
    pending.push_note(note(1));
    pending.push_note(note(2));
    let reopened = PendingWrites::open(&dir);
    assert_eq!(reopened.note_count(), 2);
    assert_eq!(reopened.front_note(), Some(&note(1)));

    pending.pop_note();
    assert_eq!(PendingWrites::open(&dir).front_note(), Some(&note(2)));
    pending.pop_note();
    assert!(!dir.join(PENDING_FILE).exists(), "队列空了就删文件");
}

#[test]
fn a_corrupt_line_is_skipped_and_the_rest_restored() {
    let dir = memory_dir("corrupt");
    let good = |n| serde_json::to_string(&note(n)).unwrap();
    let text = format!("{}\n{{坏了\n\n{}\n", good(1), good(3));
    std::fs::write(dir.join(PENDING_FILE), text).unwrap();
    let mut pending = PendingWrites::open(&dir);
    assert_eq!(pending.pop_note(), Some(note(1)));
    assert_eq!(pending.pop_note(), Some(note(3)));
    assert_eq!(pending.pop_note(), None);
}

#[test]
fn the_file_obeys_the_same_cap_as_memory() {
    let dir = memory_dir("cap");
    let mut pending = PendingWrites::open(&dir);
    for n in 0..MAX_PENDING_NOTES + 5 {
        pending.push_note(note(n));
    }
    let lines = std::fs::read_to_string(dir.join(PENDING_FILE)).unwrap();
    assert_eq!(lines.lines().count(), MAX_PENDING_NOTES);
    let mut reopened = PendingWrites::open(&dir);
    assert_eq!(reopened.pop_note(), Some(note(5)), "文件里丢的也是最旧的");

    // 手工写了超过上限的文件，读回来也只留最后 32 条
    let many: String = (0..MAX_PENDING_NOTES + 3)
        .map(|n| format!("{}\n", serde_json::to_string(&note(n)).unwrap()))
        .collect();
    std::fs::write(dir.join(PENDING_FILE), many).unwrap();
    let reopened = PendingWrites::open(&dir);
    assert_eq!(reopened.note_count(), MAX_PENDING_NOTES);
    assert_eq!(reopened.front_note(), Some(&note(3)));
}

#[test]
fn a_missing_memory_dir_is_neither_created_nor_an_error() {
    let dir = memory_dir("nodir").join("memory");
    let mut pending = PendingWrites::open(&dir);
    pending.push_note(note(1));
    pending.pop_note();
    assert!(!dir.exists());
}
