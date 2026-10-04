//! 键盘待办队列：「记一笔」按顺序、有上限，切场景只留最新一次。拿不到文件锁的端到端行为见 `tests/memory_ffi.rs`。

use qingjian_cloud_proto::Scene;

use super::pending::{MAX_PENDING_NOTES, PendingNote, PendingWrites};

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
