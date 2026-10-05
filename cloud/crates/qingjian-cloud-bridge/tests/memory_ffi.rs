//! 本地记忆的 C 接口：按 C 签名直接调，不需要产品数据。最后一个测试核对头文件与导出符号逐个一致。
//! 「记一笔」素材在 `memory_materials_ffi.rs`。

mod memory_support;

use std::collections::BTreeSet;
use std::path::Path;
use std::ptr;
use std::time::Instant;

use qingjian_cloud_bridge::{qj_flush, qj_poll, qj_push, qj_session_free, qj_set_private};
use serde_json::{Value, json};

use memory_support::{
    CARD, CONTACT, KEYBOARD_BUDGET, c, dirs, hold_lock, json_of, note, open, qj_memory_add_contact,
    qj_memory_cards, qj_memory_dismiss, qj_memory_hint, qj_memory_material_delete,
    qj_memory_materials, qj_memory_note, qj_memory_read, qj_memory_write, qj_reset_context,
    qj_scope_get, qj_scope_set, seed, set_contact, take, type_and_commit,
};

#[test]
fn scope_set_round_trips_without_scenes() {
    let (data, user) = dirs("scope");
    seed(&user);
    let session = open(&data, Some(&user));
    let scope = json_of(unsafe { qj_scope_get(session) });
    assert_eq!(scope["contact_id"], Value::Null);
    assert!(scope.get("scene").is_none(), "没有场景了：{scope}");
    assert!(scope.get("last").is_none(), "没有 last 了：{scope}");

    set_contact(session, Some(CONTACT));
    assert_eq!(
        json_of(unsafe { qj_scope_get(session) })["contact_id"],
        CONTACT
    );

    // 空指针 = 保持现在选的人不变（幂等）
    set_contact(session, None);
    assert_eq!(
        json_of(unsafe { qj_scope_get(session) })["contact_id"],
        CONTACT
    );

    // 空字符串 = 明确不指定
    set_contact(session, Some(""));
    assert_eq!(
        json_of(unsafe { qj_scope_get(session) })["contact_id"],
        Value::Null
    );

    // 名单上没有的对象当不指定
    set_contact(session, Some("ffffffffffffffffffffffffffffffff"));
    assert_eq!(
        json_of(unsafe { qj_scope_get(session) })["contact_id"],
        Value::Null,
        "名单上没有的对象当不指定"
    );
    unsafe { qj_session_free(session) };
}

#[test]
fn hint_shows_on_match_and_hides_when_private() {
    let (data, user) = dirs("hint");
    seed(&user);
    let session = open(&data, Some(&user));
    assert_eq!(type_and_commit(session, "shengri"), "生日");
    assert!(
        take(unsafe { qj_memory_hint(session) }).is_none(),
        "还没选对象，不出提示"
    );

    set_contact(session, Some(CONTACT));
    assert!(
        take(unsafe { qj_memory_hint(session) }).is_none(),
        "换对象时清了最近的字"
    );
    assert_eq!(type_and_commit(session, "shengri"), "生日");
    let hint = json_of(unsafe { qj_memory_hint(session) });
    assert_eq!(hint["card_id"], CARD);
    assert_eq!(hint["reason"], "match");
    assert_eq!(hint["text"], "想要一个生日蛋糕");

    unsafe { qj_set_private(session, true) };
    assert!(
        take(unsafe { qj_memory_hint(session) }).is_none(),
        "私密输入不出提示"
    );
    unsafe { qj_set_private(session, false) };
    assert!(take(unsafe { qj_memory_hint(session) }).is_some());

    let card = c(CARD);
    unsafe { qj_memory_dismiss(session, card.as_ptr(), true) };
    assert!(take(unsafe { qj_memory_hint(session) }).is_none());
    assert_eq!(type_and_commit(session, "shengri"), "生日");
    assert!(
        take(unsafe { qj_memory_hint(session) }).is_none(),
        "「知道了」当天不再出"
    );
    unsafe { qj_session_free(session) };

    // 记进了 dismissed.json：键盘重开也记得
    let session = open(&data, Some(&user));
    assert_eq!(type_and_commit(session, "shengri"), "生日");
    assert!(take(unsafe { qj_memory_hint(session) }).is_none());
    assert!(user.join("memory/dismissed.json").is_file());
    unsafe { qj_session_free(session) };
}

#[test]
fn flush_and_new_field_clear_recent_text() {
    let (data, user) = dirs("reset");
    seed(&user);
    let session = open(&data, Some(&user));
    set_contact(session, Some(CONTACT));
    assert_eq!(type_and_commit(session, "shengri"), "生日");
    assert!(take(unsafe { qj_memory_hint(session) }).is_some());
    unsafe { qj_flush(session) };
    assert!(
        take(unsafe { qj_memory_hint(session) }).is_none(),
        "键盘收起清掉"
    );
    assert_eq!(type_and_commit(session, "dianying"), "电影");
    assert!(
        take(unsafe { qj_memory_hint(session) }).is_none(),
        "旧的「生日」不再触发"
    );

    let (data, user) = dirs("reset-field");
    seed(&user);
    let other = open(&data, Some(&user));
    set_contact(other, Some(CONTACT));
    assert_eq!(type_and_commit(other, "shengri"), "生日");
    unsafe { qj_reset_context(other) };
    assert_eq!(type_and_commit(other, "dianying"), "电影");
    assert!(
        take(unsafe { qj_memory_hint(other) }).is_none(),
        "换了输入框，旧的字不算"
    );
    unsafe { qj_session_free(session) };
    unsafe { qj_session_free(other) };
}

#[test]
fn hint_switch_is_per_contact() {
    let (data, user) = dirs("hint-off");
    seed(&user);
    let dir = c(user.to_str().unwrap());
    let mut snapshot = json_of(unsafe { qj_memory_read(dir.as_ptr()) });
    snapshot["contacts"][0]["hint_on"] = json!(false);
    let text = c(&snapshot.to_string());
    assert_eq!(
        take(unsafe { qj_memory_write(dir.as_ptr(), text.as_ptr()) }),
        None
    );
    let session = open(&data, Some(&user));
    set_contact(session, Some(CONTACT));
    assert_eq!(type_and_commit(session, "shengri"), "生日");
    assert!(
        take(unsafe { qj_memory_hint(session) }).is_none(),
        "这个人关了打字时提示"
    );
    unsafe { qj_session_free(session) };
}

#[test]
fn forgotten_contact_stays_forgotten() {
    let (data, user) = dirs("forgotten");
    seed(&user);
    let session = open(&data, Some(&user));
    set_contact(session, Some(CONTACT));
    let dir = c(user.to_str().unwrap());
    let empty = c(&json!({"contacts": [], "cards": {}}).to_string());
    assert_eq!(
        take(unsafe { qj_memory_write(dir.as_ptr(), empty.as_ptr()) }),
        None
    );
    let failure: Value =
        serde_json::from_str(&note(session, CONTACT, "又记一笔").unwrap()).unwrap();
    assert_eq!(
        failure["code"], "invalid",
        "键盘内存里的名单还没刷新，按磁盘判断"
    );
    unsafe { qj_flush(session) };
    assert!(
        !user.join("memory").join(CONTACT).exists(),
        "对象目录没被重新建出来"
    );
    unsafe { qj_session_free(session) };
}

#[test]
fn the_roster_holds_any_number_of_people() {
    let (_, user) = dirs("limit");
    let contacts: Vec<Value> = (0..12)
        .map(|n| {
            json!({"id": format!("{n:032x}"), "name": format!("人{n}"), "pronoun": "ta", "created_at": 0})
        })
        .collect();
    let dir = c(user.to_str().unwrap());
    let many = c(&json!({"contacts": contacts, "cards": {}, "state": {}}).to_string());
    assert_eq!(
        take(unsafe { qj_memory_write(dir.as_ptr(), many.as_ptr()) }),
        None
    );
    let snapshot = json_of(unsafe { qj_memory_read(dir.as_ptr()) });
    assert_eq!(
        snapshot["contacts"].as_array().unwrap().len(),
        12,
        "人数不限"
    );
}

#[test]
fn bad_arguments_do_not_crash() {
    assert!(take(unsafe { qj_scope_get(ptr::null_mut()) }).is_none());
    unsafe { qj_scope_set(ptr::null_mut(), ptr::null()) };
    assert!(take(unsafe { qj_memory_hint(ptr::null_mut()) }).is_none());
    unsafe { qj_memory_dismiss(ptr::null_mut(), ptr::null(), true) };
    assert!(take(unsafe { qj_memory_cards(ptr::null_mut(), ptr::null()) }).is_none());
    assert!(take(unsafe { qj_memory_read(ptr::null()) }).is_none());
    assert!(take(unsafe { qj_memory_materials(ptr::null(), ptr::null()) }).is_none());
    let failure =
        json_of(unsafe { qj_memory_material_delete(ptr::null(), ptr::null(), ptr::null()) });
    assert_eq!(failure["code"], "invalid");
    let failure = json_of(unsafe { qj_memory_write(ptr::null(), ptr::null()) });
    assert_eq!(failure["code"], "invalid");
    let failure =
        json_of(unsafe { qj_memory_note(ptr::null_mut(), ptr::null(), ptr::null(), ptr::null()) });
    assert_eq!(failure["code"], "invalid");

    let (data, user) = dirs("bad-args");
    let dir = c(user.to_str().unwrap());
    let broken = c("{");
    let failure = json_of(unsafe { qj_memory_write(dir.as_ptr(), broken.as_ptr()) });
    assert_eq!(failure["code"], "invalid");

    // 没有学习数据目录的会话：没有记忆
    let session = open(&data, None);
    assert!(take(unsafe { qj_scope_get(session) }).is_none());
    set_contact(session, Some(CONTACT));
    assert!(take(unsafe { qj_memory_hint(session) }).is_none());
    let failure: Value = serde_json::from_str(&note(session, CONTACT, "x").unwrap()).unwrap();
    assert_eq!(failure["code"], "invalid");
    unsafe { qj_session_free(session) };
}

#[test]
fn keyboard_scope_switch_is_deferred_and_keeps_only_the_latest() {
    let (data, user) = dirs("scope-locked");
    seed(&user);
    let session = open(&data, Some(&user));

    let lock = hold_lock(&user);
    let started = Instant::now();
    set_contact(session, None);
    set_contact(session, Some(CONTACT));
    let elapsed = started.elapsed();
    eprintln!("锁被占着时两次 qj_scope_set 用了 {elapsed:?}");
    assert!(
        elapsed < KEYBOARD_BUDGET * 2,
        "每次最多等 200 毫秒：{elapsed:?}"
    );
    let scope = json_of(unsafe { qj_scope_get(session) });
    assert_eq!(scope["contact_id"], CONTACT, "内存里照切");
    // 磁盘上还没写，轮询也不能把刚切的读回旧的
    assert!(
        !user.join("memory/state.json").exists() || {
            !std::fs::read_to_string(user.join("memory/state.json"))
                .unwrap()
                .contains(CONTACT)
        }
    );
    unsafe { qj_poll(session) };
    assert_eq!(
        json_of(unsafe { qj_scope_get(session) })["contact_id"],
        CONTACT
    );

    drop(lock);
    unsafe { qj_push(session, 's' as u32) };
    let state: Value =
        serde_json::from_str(&std::fs::read_to_string(user.join("memory/state.json")).unwrap())
            .unwrap();
    assert_eq!(state["contact_id"], CONTACT, "待办只留最后一条，补写成功");
    unsafe { qj_session_free(session) };
}

#[test]
fn keyboard_adds_contacts_without_a_cap() {
    let (data, user) = dirs("add-limit");
    seed(&user);
    let session = open(&data, Some(&user));
    for n in 0..12 {
        let name = c(&format!("日常{n}"));
        let added = json_of(unsafe { qj_memory_add_contact(session, name.as_ptr(), ptr::null()) });
        assert!(added["id"].is_string(), "第 {n} 个应当建得了：{added}");
    }
    unsafe { qj_session_free(session) };
    let dir = c(user.to_str().unwrap());
    let snapshot = json_of(unsafe { qj_memory_read(dir.as_ptr()) });
    assert_eq!(
        snapshot["contacts"].as_array().unwrap().len(),
        13,
        "12 个新的 + 种子那 1 个"
    );
}

/// 学习数据目录或某层目录下的 `user.tsv` 里有没有这个词。
fn learned(dir: &Path, word: &str) -> bool {
    std::fs::read_to_string(dir.join("user.tsv")).is_ok_and(|text| text.contains(word))
}

#[test]
fn learning_is_isolated_per_person() {
    let (data, user) = dirs("learn-per-person");
    seed(&user);
    let session = open(&data, Some(&user));
    let name = c("妈妈");
    let second =
        json_of(unsafe { qj_memory_add_contact(session, name.as_ptr(), ptr::null()) })["id"]
            .as_str()
            .expect("建好返回 id")
            .to_owned();
    let memory = user.join("memory");

    // 选了人：只写这个人的对象层，不碰全局
    set_contact(session, Some(CONTACT));
    assert_eq!(type_and_commit(session, "shengri"), "生日");
    unsafe { qj_flush(session) };
    assert!(
        learned(&memory.join(CONTACT).join("learning"), "生日"),
        "写对象层"
    );
    assert!(!learned(&user, "生日"), "选了人不写全局");

    // 换个人：他的层还是空的，上一个人学的没串过来
    set_contact(session, Some(&second));
    assert!(
        !learned(&memory.join(&second).join("learning"), "生日"),
        "别人学的不串过来"
    );
    unsafe { qj_session_free(session) };

    // 不指定：写全局
    let session = open(&data, Some(&user));
    set_contact(session, Some(""));
    assert_eq!(type_and_commit(session, "shengri"), "生日");
    unsafe { qj_flush(session) };
    assert!(learned(&user, "生日"), "不指定写全局");
    unsafe { qj_session_free(session) };
}

#[test]
fn header_declares_every_export() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let header = std::fs::read_to_string(root.join("include/qingjian_bridge.h")).unwrap();
    let declared: BTreeSet<String> = header
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .flat_map(called_names)
        .collect();
    let mut exported = BTreeSet::new();
    collect_exports(&root.join("src"), &mut exported);
    assert!(exported.contains("qj_memory_write"), "{exported:?}");
    assert_eq!(declared, exported);
}

/// 一行里所有紧跟 `(` 的 `qj_xxx`。
fn called_names(line: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut rest = line;
    while let Some(start) = rest.find("qj_") {
        let tail = &rest[start..];
        let end = tail
            .find(|c: char| !(c.is_ascii_lowercase() || c == '_'))
            .unwrap_or(tail.len());
        if tail[end..].starts_with('(') {
            names.push(tail[..end].to_owned());
        }
        rest = &tail[end..];
    }
    names
}

/// `src/` 下所有 `extern "C" fn qj_xxx(` 的名字。
fn collect_exports(dir: &Path, out: &mut BTreeSet<String>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            collect_exports(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            for line in std::fs::read_to_string(&path).unwrap().lines() {
                if let Some((_, tail)) = line.split_once("extern \"C\" fn ")
                    && let Some((name, _)) = tail.split_once('(')
                {
                    out.insert(name.trim().to_owned());
                }
            }
        }
    }
}
