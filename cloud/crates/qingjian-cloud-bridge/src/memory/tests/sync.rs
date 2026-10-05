//! App 与键盘两个进程同时写：文件锁、修订号冲突与重读合并、读不了时不覆盖、忘掉的人不复活、「知道了」的记录。
//! 键盘记一笔现在写素材（`materials.jsonl`），卡片的修订号冲突改由测试里的另一个实例直接改卡片来制造。
//! 键盘侧只等 200 毫秒的锁：别的进程占着时返回 `LockTimeout`，不卡 2 秒。
//! 「两个进程」用两个线程各持一个 `MemoryStore` 模拟：flock 锁的是打开的文件，同一进程里两个实例也互斥。

use std::collections::{HashMap, HashSet};
use std::fs::OpenOptions;
use std::os::unix::fs::PermissionsExt;
use std::sync::{Arc, Barrier, mpsc};
use std::time::{Duration, Instant};

use qingjian_cloud_proto::CardKind;

use super::{card, contact, id, pick, temp_dir};
use crate::memory::{LocalDate, MaterialSource, MemoryError, MemorySnapshot, MemoryStore};

/// 另一个进程往卡片末尾加一张（读与写各拿一次锁，只在不并发的地方用）；记一笔现在写素材，不再碰卡片。
fn append_card(store: &MemoryStore, n: u32, text: &str, touched_at: i64) {
    let mut cards = store.try_cards(&id(1)).unwrap();
    cards.push(card(n, CardKind::Other, text, &[], None, touched_at));
    store.put_cards(&id(1), &cards).unwrap();
}

/// 测试里的 App：在快照上把第一张卡改成 `text`。
fn edit_first_card(snapshot: &mut MemorySnapshot, text: &str) {
    if let Some(first) = snapshot
        .cards
        .get_mut(&id(1))
        .and_then(|cards| cards.first_mut())
    {
        first.text = text.to_owned();
    }
}

#[test]
fn stale_snapshot_conflicts_and_merges() {
    let user = temp_dir("conflict");
    let store = MemoryStore::open(&user);
    store.put_contact(contact(1)).unwrap();
    store
        .put_cards(&id(1), &[card(1, CardKind::Other, "原来的", &[], None, 1)])
        .unwrap();
    let mut stale = store.snapshot().unwrap();
    append_card(&store, 2, "另一处加的", 2);
    edit_first_card(&mut stale, "App 改的");
    assert!(matches!(
        store.write_snapshot(&stale),
        Err(MemoryError::Conflict)
    ));
    let mut fresh = store.snapshot().unwrap();
    edit_first_card(&mut fresh, "App 改的");
    store.write_snapshot(&fresh).unwrap();
    let texts: Vec<String> = store
        .cards(&id(1))
        .into_iter()
        .map(|card| card.text)
        .collect();
    assert_eq!(texts, vec!["App 改的", "另一处加的"]);
    std::fs::remove_dir_all(&user).ok();
}

/// 键盘记一笔写 `materials.jsonl`、App 整份写回卡片与名单，两边在同一把锁里交错，谁的都不丢。
#[test]
fn two_processes_lose_no_notes() {
    let user = temp_dir("two-processes");
    MemoryStore::open(&user).put_contact(contact(1)).unwrap();
    MemoryStore::open(&user)
        .put_cards(&id(1), &[card(1, CardKind::Other, "第一张", &[], None, 1)])
        .unwrap();
    let keyboard_dir = user.clone();
    let keyboard = std::thread::spawn(move || {
        let store = MemoryStore::open(&keyboard_dir);
        for i in 0..40 {
            store
                .add_material(&id(1), &format!("记一笔 {i}"), MaterialSource::Typed, i)
                .unwrap();
        }
    });
    let app_dir = user.clone();
    let app = std::thread::spawn(move || {
        let store = MemoryStore::open(&app_dir);
        for i in 0..40 {
            let mut snapshot = store.snapshot().unwrap();
            edit_first_card(&mut snapshot, &format!("App 改 {i}"));
            store.write_snapshot(&snapshot).unwrap();
        }
    });
    keyboard.join().unwrap();
    app.join().unwrap();
    let store = MemoryStore::open(&user);
    let texts: HashSet<String> = store
        .materials(&id(1), 40)
        .unwrap()
        .into_iter()
        .map(|m| m.text)
        .collect();
    for i in 0..40 {
        assert!(texts.contains(&format!("记一笔 {i}")), "第 {i} 条丢了");
    }
    assert_eq!(store.cards(&id(1))[0].text, "App 改 39");
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn unreadable_files_abort_writes() {
    let user = temp_dir("unreadable");
    let store = MemoryStore::open(&user);
    store.put_contact(contact(1)).unwrap();
    store
        .put_cards(&id(1), &[card(1, CardKind::Other, "真文件", &[], None, 1)])
        .unwrap();
    let memory = user.join("memory");
    let cards = memory.join(id(1)).join("cards.json");
    let before = std::fs::read_to_string(&cards).unwrap();
    let deny = |path: &std::path::Path, mode: u32| {
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).unwrap();
    };

    deny(&cards, 0o000);
    assert!(matches!(
        store.put_cards(&id(1), &[card(2, CardKind::Other, "新卡", &[], None, 2)]),
        Err(MemoryError::Io(_))
    ));
    assert!(store.try_cards(&id(1)).is_err());
    assert!(store.snapshot().is_err(), "整份读不全就不给 App");
    deny(&cards, 0o600);
    assert_eq!(
        std::fs::read_to_string(&cards).unwrap(),
        before,
        "没被空表覆盖"
    );

    let contacts = memory.join("contacts.json");
    deny(&contacts, 0o000);
    assert!(matches!(
        store.put_contact(contact(2)),
        Err(MemoryError::Io(_))
    ));
    assert!(matches!(
        store.update_contact(&pick(1), 0),
        Err(MemoryError::Io(_))
    ));
    assert!(store.contacts().is_empty(), "只读的接口读不了给空");
    deny(&contacts, 0o600);
    assert_eq!(store.contacts().len(), 1);
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn forgotten_contact_is_not_recreated() {
    let user = temp_dir("forgotten");
    let store = MemoryStore::open(&user);
    store.put_contact(contact(1)).unwrap();
    store
        .put_cards(&id(1), &[card(1, CardKind::Other, "喜欢猫", &[], None, 1)])
        .unwrap();
    store.forget_contact(&id(1)).unwrap();
    let dir = user.join("memory").join(id(1));
    assert!(matches!(
        store.add_material(&id(1), "又记一笔", MaterialSource::Typed, 2),
        Err(MemoryError::Invalid(_))
    ));
    assert!(matches!(
        store.put_cards(&id(1), &[]),
        Err(MemoryError::Invalid(_))
    ));
    assert!(!dir.exists(), "目录没被重新建出来");
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn dismissed_records_drop_old_and_unknown_cards() {
    let user = temp_dir("dismissed");
    let store = MemoryStore::open(&user);
    let today = LocalDate::parse("2026-10-04").unwrap();
    let mut records = HashMap::new();
    records.insert(id(1001), today);
    records.insert(id(1002), today.add_days(-31));
    records.insert(id(1003), today);
    store.put_dismissed(&records).unwrap();
    let known: HashSet<String> = [id(1001), id(1002)].into_iter().collect();
    let loaded = store.dismissed(today, &known);
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[&id(1001)], today);
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn keyboard_lock_wait_times_out_quickly() {
    let user = temp_dir("lock-timeout");
    let keyboard = MemoryStore::open_with_lock_timeout(&user, Duration::from_millis(200));
    keyboard.put_contact(contact(1)).unwrap();

    let lock_path = user.join("memory").join(".lock");
    let (held_tx, held_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel::<()>();
    let holder = std::thread::spawn(move || {
        let file = OpenOptions::new().write(true).open(lock_path).unwrap();
        file.lock().unwrap();
        held_tx.send(()).unwrap();
        release_rx.recv_timeout(Duration::from_secs(10)).ok();
    });
    held_rx.recv_timeout(Duration::from_secs(5)).unwrap();

    let started = Instant::now();
    let result = keyboard.add_material(&id(1), "拿不到锁", MaterialSource::Typed, 1);
    let waited = started.elapsed();
    assert!(
        matches!(result, Err(MemoryError::LockTimeout)),
        "{result:?}"
    );
    assert!(waited >= Duration::from_millis(150), "{waited:?}");
    assert!(waited < Duration::from_millis(500), "{waited:?}");
    assert_eq!(MemoryError::LockTimeout.code(), "lock_timeout");
    eprintln!("200ms 超时实测 {waited:?}");

    release_tx.send(()).unwrap();
    holder.join().unwrap();
    keyboard
        .add_material(&id(1), "锁放了就能写", MaterialSource::Typed, 2)
        .unwrap();
    assert_eq!(keyboard.materials(&id(1), 2).unwrap().len(), 1);

    assert_eq!(
        MemoryStore::open(&user).lock_timeout(),
        Duration::from_secs(2),
        "缺省 2 秒"
    );
    std::fs::remove_dir_all(&user).ok();
}

/// 强制交错：A（App）读完快照 → B（键盘）写一次 → A 用旧快照写回。返回 A 收到的第一个结果与最终的卡片。
/// `b_write` 是 B 在自己的实例上做的那次写；`a_edit` 是 A 在快照上的改动；`merge` 在冲突后把 A 的改动并进新快照。
fn forced_interleaving(
    name: &str,
    b_write: impl FnOnce(&MemoryStore) + Send + 'static,
    a_edit: impl Fn(&mut MemorySnapshot),
    merge: impl Fn(&mut MemorySnapshot, &MemorySnapshot),
) -> (Result<(), MemoryError>, usize, Vec<crate::memory::Card>) {
    let user = temp_dir(name);
    let setup = MemoryStore::open(&user);
    setup.put_contact(contact(1)).unwrap();
    setup
        .put_cards(
            &id(1),
            &[
                card(1, CardKind::Other, "第一张", &[], None, 1),
                card(2, CardKind::Other, "第二张", &[], None, 1),
            ],
        )
        .unwrap();

    let barrier = Arc::new(Barrier::new(2));
    let keyboard_barrier = Arc::clone(&barrier);
    let keyboard_dir = user.clone();
    let keyboard = std::thread::spawn(move || {
        let store = MemoryStore::open(&keyboard_dir);
        keyboard_barrier.wait();
        b_write(&store);
        keyboard_barrier.wait();
    });

    let app = MemoryStore::open(&user);
    let mut snapshot = app.snapshot().unwrap();
    barrier.wait();
    barrier.wait();
    a_edit(&mut snapshot);
    let first = app.write_snapshot(&snapshot);
    assert!(first.is_ok() || matches!(first, Err(MemoryError::Conflict)));
    let mut conflicts = 0;
    let mut result = Ok(());
    if matches!(first, Err(MemoryError::Conflict)) {
        conflicts += 1;
        let fresh = app.snapshot().unwrap();
        let mut merged = fresh.clone();
        merge(&mut merged, &snapshot);
        result = app.write_snapshot(&merged);
    }
    keyboard.join().unwrap();
    let cards = app.cards(&id(1));
    std::fs::remove_dir_all(&user).ok();
    (result, conflicts, cards)
}

#[test]
fn forced_interleaving_conflicts_then_merges() {
    let (result, conflicts, cards) = forced_interleaving(
        "forced",
        |store| append_card(store, 3, "另一处加的", 2),
        |snapshot| {
            snapshot.cards.get_mut(&id(1)).unwrap()[1].text = "App 改的第二张".to_owned();
        },
        |fresh, stale| {
            let edited = stale.cards[&id(1)][1].clone();
            let card = fresh
                .cards
                .get_mut(&id(1))
                .unwrap()
                .iter_mut()
                .find(|c| c.id == edited.id)
                .unwrap();
            card.text = edited.text;
        },
    );
    assert!(result.is_ok(), "{result:?}");
    assert!(conflicts >= 1, "旧快照写回必须收到 Conflict");
    let texts: Vec<&str> = cards.iter().map(|c| c.text.as_str()).collect();
    assert_eq!(texts, vec!["第一张", "App 改的第二张", "另一处加的"]);
}

/// 同一张卡两边都改：合并时 `touched_at` 新者为准。
fn same_card_case(name: &str, app_touched: i64, keyboard_touched: i64) -> String {
    let (result, conflicts, cards) = forced_interleaving(
        name,
        move |store| {
            let mut cards = store.try_cards(&id(1)).unwrap();
            cards[0].text = "键盘改的".to_owned();
            cards[0].touched_at = keyboard_touched;
            store.put_cards(&id(1), &cards).unwrap();
        },
        move |snapshot| {
            let first = &mut snapshot.cards.get_mut(&id(1)).unwrap()[0];
            first.text = "App 改的".to_owned();
            first.touched_at = app_touched;
        },
        |fresh, stale| {
            let mine = stale.cards[&id(1)][0].clone();
            let theirs = &mut fresh.cards.get_mut(&id(1)).unwrap()[0];
            if mine.touched_at > theirs.touched_at {
                *theirs = mine;
            }
        },
    );
    assert!(result.is_ok(), "{result:?}");
    assert!(conflicts >= 1);
    assert_eq!(cards.len(), 2, "另一张卡不丢");
    cards[0].text.clone()
}

#[test]
fn forced_interleaving_same_card_newest_wins() {
    assert_eq!(same_card_case("same-keyboard", 10, 20), "键盘改的");
    assert_eq!(same_card_case("same-app", 30, 20), "App 改的");
}
