//! 「记一笔」素材的 C 接口：`qj_memory_note` 写进 `materials.jsonl` 而不是卡片，App 用 `qj_memory_materials` 读、
//! `qj_memory_material_delete` 删；拿不到锁时进键盘待办，之后补写成素材，补写被拒绝的条数用 `qj_memory_dropped` 取。
//! 按 C 签名直接调，不需要产品数据。

mod memory_support;

use std::path::Path;
use std::time::Instant;

use qingjian_cloud_bridge::{qj_flush, qj_poll, qj_push, qj_session_free};
use serde_json::{Value, json};

use memory_support::{
    CONTACT, KEYBOARD_BUDGET, c, dirs, hold_lock, json_of, note, open, qj_memory_dropped,
    qj_memory_material_delete, qj_memory_materials, qj_memory_note, qj_memory_read,
    qj_memory_write, seed, take,
};

/// App 读到的 `{"unprocessed_count","materials"}`。
fn materials(user: &Path) -> Value {
    let dir = c(user.to_str().unwrap());
    let contact = c(CONTACT);
    json_of(unsafe { qj_memory_materials(dir.as_ptr(), contact.as_ptr()) })
}

fn texts(listing: &Value) -> Vec<String> {
    listing["materials"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m["text"].as_str().unwrap().to_owned())
        .collect()
}

fn materials_file(user: &Path) -> std::path::PathBuf {
    user.join("memory").join(CONTACT).join("materials.jsonl")
}

#[test]
fn note_goes_to_materials_not_cards() {
    let (data, user) = dirs("materials-note");
    seed(&user);
    let session = open(&data, Some(&user));
    let contact = c(CONTACT);
    let text = c("  周末一起看电影 ");
    let source = c("clipboard");
    assert_eq!(
        take(unsafe { qj_memory_note(session, contact.as_ptr(), text.as_ptr(), source.as_ptr()) }),
        None
    );

    let dir = c(user.to_str().unwrap());
    let snapshot = json_of(unsafe { qj_memory_read(dir.as_ptr()) });
    assert_eq!(
        snapshot["cards"][CONTACT].as_array().unwrap().len(),
        1,
        "卡片还是只有原来那张"
    );
    let file = std::fs::read_to_string(materials_file(&user)).unwrap();
    assert_eq!(file.lines().count(), 1);

    let listing = materials(&user);
    assert_eq!(listing["unprocessed_count"], 1);
    let material = &listing["materials"][0];
    assert_eq!(material["text"], "周末一起看电影");
    assert_eq!(material["kind"], "note");
    assert_eq!(material["source"], "clipboard");
    assert_eq!(material["uploaded"], false);
    assert_eq!(material["processed"], false);
    assert_eq!(material["client_id"].as_str().unwrap().len(), 32);
    assert!(material["at"].as_i64().unwrap() > 0);

    assert_eq!(note(session, CONTACT, "手写的"), None);
    assert_eq!(
        materials(&user)["materials"][0]["source"],
        "typed",
        "空指针按手写"
    );

    let failure: Value =
        serde_json::from_str(&note(session, "ffffffffffffffffffffffffffffffff", "x").unwrap())
            .unwrap();
    assert_eq!(failure["code"], "invalid");
    let failure: Value = serde_json::from_str(&note(session, CONTACT, "   ").unwrap()).unwrap();
    assert_eq!(failure["code"], "invalid");
    unsafe { qj_session_free(session) };
}

#[test]
fn long_note_is_split_into_several_materials() {
    let (data, user) = dirs("materials-split");
    seed(&user);
    let session = open(&data, Some(&user));
    let paragraph = "字".repeat(500);
    let text = [paragraph.as_str(); 3].join("\n\n");
    assert_eq!(note(session, CONTACT, &text), None);
    let listing = materials(&user);
    assert_eq!(listing["unprocessed_count"], 3, "4500 字节切成三条");
    let joined: String = texts(&listing).concat();
    assert_eq!(
        joined.chars().filter(|c| *c == '字').count(),
        1500,
        "一个字不丢"
    );
    unsafe { qj_session_free(session) };
}

#[test]
fn two_hundred_unprocessed_materials_reject_the_next() {
    let (data, user) = dirs("materials-limit");
    seed(&user);
    let session = open(&data, Some(&user));
    for n in 0..200 {
        assert_eq!(note(session, CONTACT, &format!("第 {n} 条")), None);
    }
    let failure: Value =
        serde_json::from_str(&note(session, CONTACT, "第 201 条").unwrap()).unwrap();
    assert_eq!(failure["code"], "material_limit");
    assert_eq!(
        failure["message"],
        "这个人还有 200 条没整理，先去 App 里看看"
    );
    assert_eq!(failure["remaining"], 0);
    assert_eq!(failure["needed"], 1);
    assert_eq!(materials(&user)["unprocessed_count"], 200);
    unsafe { qj_session_free(session) };
}

#[test]
fn materials_are_listed_newest_first_and_can_be_deleted() {
    let (_, user) = dirs("materials-order");
    seed(&user);
    let lines: Vec<String> = [("早", 100), ("中", 300), ("晚", 200)]
        .iter()
        .enumerate()
        .map(|(n, (text, at))| {
            format!(
                r#"{{"client_id":"{:032x}","kind":"note","text":"{text}","at":{at},"source":"typed","uploaded":false,"processed":false}}"#,
                n + 1
            )
        })
        .collect();
    // 一条已整理（今天整理的，还在保留期内）：不出现在 App 的列表里
    let processed = format!(
        r#"{{"client_id":"{:032x}","kind":"note","text":"整理过的","at":50,"source":"typed","uploaded":true,"processed":true,"processed_at":{}}}"#,
        9,
        qingjian_cloud_bridge::now_unix()
    );
    let file = format!("{}\n{processed}\n", lines.join("\n"));
    std::fs::write(materials_file(&user), file).unwrap();

    let listing = materials(&user);
    assert_eq!(
        texts(&listing),
        ["中", "晚", "早"],
        "按时间倒序，只含未整理的"
    );
    assert_eq!(listing["unprocessed_count"], 3);

    let dir = c(user.to_str().unwrap());
    let contact = c(CONTACT);
    let middle = c(&format!("{:032x}", 2));
    assert_eq!(
        take(unsafe { qj_memory_material_delete(dir.as_ptr(), contact.as_ptr(), middle.as_ptr()) }),
        None
    );
    assert_eq!(
        take(unsafe { qj_memory_material_delete(dir.as_ptr(), contact.as_ptr(), middle.as_ptr()) }),
        None,
        "删过的再删也算成功"
    );
    let listing = materials(&user);
    assert_eq!(texts(&listing), ["晚", "早"]);
    let file = std::fs::read_to_string(materials_file(&user)).unwrap();
    assert!(file.contains("整理过的"), "已整理的在保留期内还留在文件里");

    let bad = c("../x");
    let failure =
        json_of(unsafe { qj_memory_material_delete(dir.as_ptr(), bad.as_ptr(), middle.as_ptr()) });
    assert_eq!(failure["code"], "invalid");
}

#[test]
fn forgetting_the_contact_deletes_materials() {
    let (data, user) = dirs("materials-forget");
    seed(&user);
    let session = open(&data, Some(&user));
    assert_eq!(note(session, CONTACT, "喜欢猫"), None);
    assert!(materials_file(&user).exists());
    let dir = c(user.to_str().unwrap());
    let empty = c(r#"{"contacts":[],"cards":{}}"#);
    assert_eq!(
        take(unsafe { qj_memory_write(dir.as_ptr(), empty.as_ptr()) }),
        None
    );
    assert!(!materials_file(&user).exists());
    let listing = materials(&user);
    assert_eq!(listing["unprocessed_count"], 0);
    assert!(listing["materials"].as_array().unwrap().is_empty());
    unsafe { qj_session_free(session) };
}

#[test]
fn locked_note_is_deferred_then_written_as_material() {
    let (data, user) = dirs("materials-locked");
    seed(&user);
    let session = open(&data, Some(&user));

    let lock = hold_lock(&user);
    for text in ["周末一起看电影", "她喜欢喝茶"] {
        let started = Instant::now();
        let failure = note(session, CONTACT, text);
        let elapsed = started.elapsed();
        assert_eq!(failure, None, "拿不到锁视同已接受，不报错");
        assert!(elapsed < KEYBOARD_BUDGET, "主线程不能卡 2 秒：{elapsed:?}");
    }
    assert!(!materials_file(&user).exists(), "锁没放，还没写进磁盘");

    drop(lock);
    unsafe { qj_push(session, 'n' as u32) };
    assert_eq!(
        texts(&materials(&user)),
        ["她喜欢喝茶", "周末一起看电影"],
        "下一次按键补写成素材，顺序不变（列表新的在上）"
    );
    let file = std::fs::read_to_string(materials_file(&user)).unwrap();
    let order: Vec<&str> = file
        .lines()
        .map(|line| {
            if line.contains("周末") {
                "周末"
            } else {
                "喝茶"
            }
        })
        .collect();
    assert_eq!(order, ["周末", "喝茶"], "文件里按记的顺序");

    // poll 与 flush 也会补写
    let lock = hold_lock(&user);
    assert_eq!(note(session, CONTACT, "经 poll 补写"), None);
    drop(lock);
    unsafe { qj_poll(session) };
    assert_eq!(materials(&user)["unprocessed_count"], 3);
    let lock = hold_lock(&user);
    assert_eq!(note(session, CONTACT, "经 flush 补写"), None);
    drop(lock);
    unsafe { qj_flush(session) };
    assert_eq!(materials(&user)["unprocessed_count"], 4);
    unsafe { qj_session_free(session) };
}

/// 键盘扩展被系统杀掉：Session 丢了，待办靠 `pending-keyboard.jsonl` 在下次启动时补写成素材（来源也带着）。
#[test]
fn pending_note_survives_the_session_being_dropped() {
    let (data, user) = dirs("materials-restart");
    seed(&user);
    let pending_file = user.join("memory/pending-keyboard.jsonl");
    let session = open(&data, Some(&user));
    let contact = c(CONTACT);
    let text = c("被杀前记的");
    let source = c("clipboard");
    let lock = hold_lock(&user);
    assert_eq!(
        take(unsafe { qj_memory_note(session, contact.as_ptr(), text.as_ptr(), source.as_ptr()) }),
        None
    );
    assert!(pending_file.exists(), "入队时就落盘");
    unsafe { qj_session_free(session) };
    drop(lock);

    let session = open(&data, Some(&user));
    unsafe { qj_poll(session) };
    let listing = materials(&user);
    assert_eq!(texts(&listing), ["被杀前记的"]);
    assert_eq!(listing["materials"][0]["source"], "clipboard");
    assert!(!pending_file.exists(), "补写成功后文件清掉");
    unsafe { qj_session_free(session) };
}

#[test]
fn restored_note_for_a_forgotten_contact_is_dropped() {
    let (data, user) = dirs("materials-forgotten");
    seed(&user);
    let session = open(&data, Some(&user));
    let lock = hold_lock(&user);
    assert_eq!(note(session, CONTACT, "忘掉的人的笔记"), None);
    unsafe { qj_session_free(session) };
    drop(lock);

    let dir = c(user.to_str().unwrap());
    let empty = c(r#"{"contacts":[],"cards":{}}"#);
    assert_eq!(
        take(unsafe { qj_memory_write(dir.as_ptr(), empty.as_ptr()) }),
        None
    );
    let session = open(&data, Some(&user));
    unsafe { qj_poll(session) };
    assert!(
        !user.join("memory").join(CONTACT).exists(),
        "对象目录没被复活"
    );
    assert!(
        !user.join("memory/pending-keyboard.jsonl").exists(),
        "被拒绝的待办出队"
    );
    let dropped = json_of(unsafe { qj_memory_dropped(session) });
    assert_eq!(
        dropped,
        json!({"material_limit": 0, "contact_gone": 1, "queue_full": 0})
    );
    assert_eq!(
        take(unsafe { qj_memory_dropped(session) }),
        None,
        "取过一次就清零"
    );
    unsafe { qj_session_free(session) };
}

/// 排队时还有空位、补写时已经满了：不悄悄丢，按切好的条数记成 material_limit；键盘重启也还在，取走后清零，原文不落进记录。
#[test]
fn deferred_note_rejected_for_a_full_contact_is_counted() {
    let (data, user) = dirs("materials-dropped-full");
    seed(&user);
    let session = open(&data, Some(&user));
    assert_eq!(take(unsafe { qj_memory_dropped(session) }), None);
    for n in 0..199 {
        assert_eq!(note(session, CONTACT, &format!("第 {n} 条")), None);
    }
    let lock = hold_lock(&user);
    let two = "很长的原话".repeat(140);
    assert_eq!(note(session, CONTACT, &two), None, "拿不到锁先接受");
    unsafe { qj_session_free(session) };
    drop(lock);

    let session = open(&data, Some(&user));
    unsafe { qj_poll(session) };
    assert_eq!(materials(&user)["unprocessed_count"], 199, "整次不写");
    let record = std::fs::read_to_string(user.join("memory/dropped-keyboard.json")).unwrap();
    assert!(!record.contains("原话"), "只记条数与原因");
    unsafe { qj_session_free(session) };

    let session = open(&data, Some(&user));
    let dropped = json_of(unsafe { qj_memory_dropped(session) });
    assert_eq!(
        dropped,
        json!({"material_limit": 2, "contact_gone": 0, "queue_full": 0}),
        "2100 字节切成两条，按两条算；重开会话还在"
    );
    assert!(!user.join("memory/dropped-keyboard.json").exists());
    assert_eq!(take(unsafe { qj_memory_dropped(session) }), None);
    unsafe { qj_session_free(session) };
}
