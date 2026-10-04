//! 本地记忆的 C 接口：按 C 签名直接调。词库用仓库里的样例 `assets/sample/dict.tsv`（按内容认格式，起名 dict.qj 也能读），
//! 不需要产品数据。最后一个测试核对头文件与导出符号逐个一致。

use std::collections::BTreeSet;
use std::ffi::{CStr, CString, c_char};
use std::path::{Path, PathBuf};
use std::ptr;
use std::time::{Duration, Instant};

use qingjian_cloud_bridge::{
    Session, qj_commit, qj_flush, qj_poll, qj_push, qj_session_free, qj_session_open,
    qj_set_private, qj_string_free,
};
use serde_json::{Value, json};

// Session 在 C 侧是不透明指针，这里只传地址
#[allow(improper_ctypes)]
unsafe extern "C" {
    fn qj_scope_set(session: *mut Session, scene: *const c_char, contact_id: *const c_char);
    fn qj_scope_get(session: *mut Session) -> *mut c_char;
    fn qj_reset_context(session: *mut Session);
    fn qj_memory_hint(session: *mut Session) -> *mut c_char;
    fn qj_memory_dismiss(session: *mut Session, card_id: *const c_char, today: bool);
    fn qj_memory_cards(session: *mut Session, contact_id: *const c_char) -> *mut c_char;
    fn qj_memory_note(
        session: *mut Session,
        contact_id: *const c_char,
        text: *const c_char,
    ) -> *mut c_char;
    fn qj_memory_read(user_dir: *const c_char) -> *mut c_char;
    fn qj_memory_write(user_dir: *const c_char, json: *const c_char) -> *mut c_char;
}

const CONTACT: &str = "0123456789abcdef0123456789abcdef";

const CARD: &str = "fedcba9876543210fedcba9876543210";

fn take(raw: *mut c_char) -> Option<String> {
    if raw.is_null() {
        return None;
    }
    let text = unsafe { CStr::from_ptr(raw) }
        .to_string_lossy()
        .into_owned();
    unsafe { qj_string_free(raw) };
    Some(text)
}

fn json_of(raw: *mut c_char) -> Value {
    serde_json::from_str(&take(raw).expect("应当返回 JSON")).unwrap()
}

fn c(text: &str) -> CString {
    CString::new(text).unwrap()
}

/// 临时的数据目录（只有样例词库）与学习数据目录。
fn dirs(name: &str) -> (PathBuf, PathBuf) {
    let root = std::env::temp_dir().join(format!("qj-memory-ffi-{name}-{}", std::process::id()));
    std::fs::remove_dir_all(&root).ok();
    let data = root.join("data");
    let user = root.join("user");
    std::fs::create_dir_all(&data).unwrap();
    std::fs::create_dir_all(&user).unwrap();
    let sample = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../assets/sample/dict.tsv");
    std::fs::copy(sample, data.join("dict.qj")).unwrap();
    (data, user)
}

/// 经 `qj_memory_write` 放一个恋爱场景的对象与一张带关键词「生日」的卡。
fn seed(user: &Path) {
    let mut cards = serde_json::Map::new();
    cards.insert(
        CONTACT.to_owned(),
        json!([{
            "id": CARD, "kind": "other", "text": "想要一个生日蛋糕", "keywords": ["生日"],
            "when": null, "source": "manual", "confirmed": true,
            "created_at": 1_791_043_200, "touched_at": 1_791_043_200
        }]),
    );
    let snapshot = json!({
        "contacts": [{"id": CONTACT, "name": "小美", "pronoun": "ta_f", "scene": "dating", "created_at": 1_791_043_200}],
        "cards": cards,
        "state": {"scene": "daily", "contact_id": null}
    });
    let dir = c(user.to_str().unwrap());
    let text = c(&snapshot.to_string());
    assert_eq!(
        take(unsafe { qj_memory_write(dir.as_ptr(), text.as_ptr()) }),
        None
    );
}

fn open(data: &Path, user: Option<&Path>) -> *mut Session {
    let data = c(data.to_str().unwrap());
    let user = user.map(|dir| c(dir.to_str().unwrap()));
    let user_ptr = user.as_ref().map_or(ptr::null(), |dir| dir.as_ptr());
    let session = unsafe { qj_session_open(data.as_ptr(), user_ptr, ptr::null(), ptr::null()) };
    assert!(!session.is_null());
    session
}

fn set_scope(session: *mut Session, scene: &str, contact: Option<&str>) {
    let scene = c(scene);
    let contact = contact.map(c);
    let contact_ptr = contact.as_ref().map_or(ptr::null(), |id| id.as_ptr());
    unsafe { qj_scope_set(session, scene.as_ptr(), contact_ptr) };
}

fn type_and_commit(session: *mut Session, keys: &str) -> String {
    for key in keys.chars() {
        unsafe { qj_push(session, key as u32) };
    }
    take(unsafe { qj_commit(session, 0) }).unwrap()
}

#[test]
fn scope_set_round_trips() {
    let (data, user) = dirs("scope");
    seed(&user);
    let session = open(&data, Some(&user));
    let scope = json_of(unsafe { qj_scope_get(session) });
    assert_eq!(scope["scene"], "daily");
    assert_eq!(scope["contact_id"], Value::Null);

    set_scope(session, "dating", Some(CONTACT));
    let scope = json_of(unsafe { qj_scope_get(session) });
    assert_eq!(scope["scene"], "dating");
    assert_eq!(scope["contact_id"], CONTACT);
    unsafe { qj_session_free(session) };

    // 写进了 state.json，下次打开还在
    let session = open(&data, Some(&user));
    assert_eq!(
        json_of(unsafe { qj_scope_get(session) })["contact_id"],
        CONTACT
    );
    set_scope(session, "work", Some(CONTACT));
    let scope = json_of(unsafe { qj_scope_get(session) });
    assert_eq!(scope["scene"], "work");
    assert_eq!(scope["contact_id"], Value::Null, "非恋爱场景不带对象");
    set_scope(session, "party", None);
    assert_eq!(
        json_of(unsafe { qj_scope_get(session) })["scene"],
        "work",
        "不认识的场景不动"
    );
    set_scope(session, "dating", Some("ffffffffffffffffffffffffffffffff"));
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
        "日常场景不出提示"
    );

    set_scope(session, "dating", Some(CONTACT));
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
    set_scope(session, "dating", Some(CONTACT));
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
    set_scope(other, "dating", Some(CONTACT));
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
    set_scope(session, "dating", Some(CONTACT));
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
    set_scope(session, "dating", Some(CONTACT));
    let dir = c(user.to_str().unwrap());
    let empty = c(r#"{"contacts":[],"cards":{}}"#);
    assert_eq!(
        take(unsafe { qj_memory_write(dir.as_ptr(), empty.as_ptr()) }),
        None
    );
    let contact = c(CONTACT);
    let text = c("又记一笔");
    let failure = json_of(unsafe { qj_memory_note(session, contact.as_ptr(), text.as_ptr()) });
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
fn note_creates_a_manual_card() {
    let (data, user) = dirs("note");
    seed(&user);
    let session = open(&data, Some(&user));
    let contact = c(CONTACT);
    let text = c("  周末一起看电影 ");
    assert_eq!(
        take(unsafe { qj_memory_note(session, contact.as_ptr(), text.as_ptr()) }),
        None
    );

    let dir = c(user.to_str().unwrap());
    let snapshot = json_of(unsafe { qj_memory_read(dir.as_ptr()) });
    let cards = snapshot["cards"][CONTACT].as_array().unwrap();
    assert_eq!(cards.len(), 2);
    let note = &cards[1];
    assert_eq!(note["kind"], "other");
    assert_eq!(note["text"], "周末一起看电影");
    assert_eq!(note["source"], "manual");
    assert_eq!(note["confirmed"], true);
    assert_eq!(note["id"].as_str().unwrap().len(), 32);

    let panel = json_of(unsafe { qj_memory_cards(session, contact.as_ptr()) });
    assert_eq!(panel.as_array().unwrap().len(), 2);

    let stranger = c("ffffffffffffffffffffffffffffffff");
    let failure = json_of(unsafe { qj_memory_note(session, stranger.as_ptr(), text.as_ptr()) });
    assert_eq!(failure["code"], "invalid");
    let blank = c("   ");
    let failure = json_of(unsafe { qj_memory_note(session, contact.as_ptr(), blank.as_ptr()) });
    assert_eq!(failure["code"], "invalid");
    unsafe { qj_session_free(session) };
}

#[test]
fn ninth_dating_contact_is_rejected() {
    let (_, user) = dirs("limit");
    let people = |count: u32| -> Value {
        let contacts: Vec<Value> = (0..count)
            .map(|n| json!({"id": format!("{n:032x}"), "name": format!("人{n}"), "pronoun": "ta", "scene": "dating", "created_at": 0}))
            .collect();
        json!({"contacts": contacts, "cards": {}, "state": {}})
    };
    let dir = c(user.to_str().unwrap());
    let nine = c(&people(9).to_string());
    let failure = json_of(unsafe { qj_memory_write(dir.as_ptr(), nine.as_ptr()) });
    assert_eq!(failure["code"], "contact_limit");
    assert_eq!(failure["message"], "恋爱场景最多 8 个人");
    let eight = c(&people(8).to_string());
    assert_eq!(
        take(unsafe { qj_memory_write(dir.as_ptr(), eight.as_ptr()) }),
        None
    );
    let snapshot = json_of(unsafe { qj_memory_read(dir.as_ptr()) });
    assert_eq!(snapshot["contacts"].as_array().unwrap().len(), 8);
}

#[test]
fn bad_arguments_do_not_crash() {
    assert!(take(unsafe { qj_scope_get(ptr::null_mut()) }).is_none());
    unsafe { qj_scope_set(ptr::null_mut(), ptr::null(), ptr::null()) };
    assert!(take(unsafe { qj_memory_hint(ptr::null_mut()) }).is_none());
    unsafe { qj_memory_dismiss(ptr::null_mut(), ptr::null(), true) };
    assert!(take(unsafe { qj_memory_cards(ptr::null_mut(), ptr::null()) }).is_none());
    assert!(take(unsafe { qj_memory_read(ptr::null()) }).is_none());
    let failure = json_of(unsafe { qj_memory_write(ptr::null(), ptr::null()) });
    assert_eq!(failure["code"], "invalid");
    let failure = json_of(unsafe { qj_memory_note(ptr::null_mut(), ptr::null(), ptr::null()) });
    assert_eq!(failure["code"], "invalid");

    let (data, user) = dirs("bad-args");
    let dir = c(user.to_str().unwrap());
    let broken = c("{");
    let failure = json_of(unsafe { qj_memory_write(dir.as_ptr(), broken.as_ptr()) });
    assert_eq!(failure["code"], "invalid");

    // 没有学习数据目录的会话：没有记忆
    let session = open(&data, None);
    assert!(take(unsafe { qj_scope_get(session) }).is_none());
    set_scope(session, "dating", Some(CONTACT));
    assert!(take(unsafe { qj_memory_hint(session) }).is_none());
    let contact = c(CONTACT);
    let text = c("x");
    let failure = json_of(unsafe { qj_memory_note(session, contact.as_ptr(), text.as_ptr()) });
    assert_eq!(failure["code"], "invalid");
    unsafe { qj_session_free(session) };
}

/// 模拟 App 占着 `memory/.lock`：持有返回的文件就是持有锁，丢掉即释放。
fn hold_lock(user: &Path) -> std::fs::File {
    let dir = user.join("memory");
    std::fs::create_dir_all(&dir).unwrap();
    let file = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(dir.join(".lock"))
        .unwrap();
    file.lock().unwrap();
    file
}

/// 键盘等锁的上限是 200 毫秒；主线程上一次调用最多容忍这么久（加调度与慢机器的余量）。
const KEYBOARD_BUDGET: Duration = Duration::from_millis(900);

#[test]
fn keyboard_note_is_deferred_while_the_lock_is_held() {
    let (data, user) = dirs("note-locked");
    seed(&user);
    let session = open(&data, Some(&user));
    let contact = c(CONTACT);
    let cards_path = user.join("memory").join(CONTACT).join("cards.json");

    let lock = hold_lock(&user);
    for text in ["周末一起看电影", "她喜欢喝茶"] {
        let text = c(text);
        let started = Instant::now();
        let failure = take(unsafe { qj_memory_note(session, contact.as_ptr(), text.as_ptr()) });
        let elapsed = started.elapsed();
        eprintln!("锁被占着时 qj_memory_note 用了 {elapsed:?}");
        assert_eq!(failure, None, "拿不到锁视同已接受，不报错");
        assert!(elapsed >= Duration::from_millis(150), "应当等满键盘的超时");
        assert!(elapsed < KEYBOARD_BUDGET, "主线程不能卡 2 秒：{elapsed:?}");
    }
    let on_disk = std::fs::read_to_string(&cards_path).unwrap();
    assert!(!on_disk.contains("周末一起看电影"), "锁没放，还没写进磁盘");

    drop(lock);
    unsafe { qj_push(session, 'n' as u32) };
    let dir = c(user.to_str().unwrap());
    let snapshot = json_of(unsafe { qj_memory_read(dir.as_ptr()) });
    let texts: Vec<&str> = snapshot["cards"][CONTACT]
        .as_array()
        .unwrap()
        .iter()
        .map(|card| card["text"].as_str().unwrap())
        .collect();
    assert_eq!(
        texts,
        ["想要一个生日蛋糕", "周末一起看电影", "她喜欢喝茶"],
        "下一次按键补写成功，顺序不变"
    );
    unsafe { qj_session_free(session) };
}

#[test]
fn keyboard_note_retries_on_poll_and_flush_too() {
    let (data, user) = dirs("note-poll");
    seed(&user);
    let session = open(&data, Some(&user));
    let contact = c(CONTACT);
    let text = c("经 poll 补写");
    let lock = hold_lock(&user);
    assert_eq!(
        take(unsafe { qj_memory_note(session, contact.as_ptr(), text.as_ptr()) }),
        None
    );
    drop(lock);
    unsafe { qj_poll(session) };
    let dir = c(user.to_str().unwrap());
    let snapshot = json_of(unsafe { qj_memory_read(dir.as_ptr()) });
    assert_eq!(snapshot["cards"][CONTACT].as_array().unwrap().len(), 2);

    let lock = hold_lock(&user);
    let text = c("经 flush 补写");
    assert_eq!(
        take(unsafe { qj_memory_note(session, contact.as_ptr(), text.as_ptr()) }),
        None
    );
    drop(lock);
    unsafe { qj_flush(session) };
    let snapshot = json_of(unsafe { qj_memory_read(dir.as_ptr()) });
    assert_eq!(snapshot["cards"][CONTACT].as_array().unwrap().len(), 3);
    unsafe { qj_session_free(session) };
}

#[test]
fn keyboard_scope_switch_is_deferred_and_keeps_only_the_latest() {
    let (data, user) = dirs("scope-locked");
    seed(&user);
    let session = open(&data, Some(&user));

    let lock = hold_lock(&user);
    let started = Instant::now();
    set_scope(session, "work", None);
    set_scope(session, "dating", Some(CONTACT));
    let elapsed = started.elapsed();
    eprintln!("锁被占着时两次 qj_scope_set 用了 {elapsed:?}");
    assert!(
        elapsed < KEYBOARD_BUDGET * 2,
        "每次最多等 200 毫秒：{elapsed:?}"
    );
    let scope = json_of(unsafe { qj_scope_get(session) });
    assert_eq!(scope["scene"], "dating", "内存里照切");
    assert_eq!(scope["contact_id"], CONTACT);
    // 磁盘上还没写，轮询也不能把刚切的读回旧的
    assert!(
        !user.join("memory/state.json").exists() || {
            !std::fs::read_to_string(user.join("memory/state.json"))
                .unwrap()
                .contains("dating")
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
    assert_eq!(state["scene"], "dating", "待办只留最后一次，补写成功");
    assert_eq!(state["contact_id"], CONTACT);
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
