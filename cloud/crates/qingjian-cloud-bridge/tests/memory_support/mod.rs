//! 本地记忆 C 接口测试共用：按 C 签名声明导出函数、临时目录（只有样例词库 `assets/sample/dict.tsv`，按内容认格式，起名 dict.qj 也能读）、
//! 样例对象与卡片、会话与按键的小工具。`memory_ffi.rs`、`memory_scene_ffi.rs` 与 `memory_materials_ffi.rs` 各用一部分，所以关掉未使用的告警。
#![allow(dead_code)]

use std::ffi::{CStr, CString, c_char};
use std::path::{Path, PathBuf};
use std::ptr;
use std::time::Duration;

use qingjian_cloud_bridge::{Session, qj_commit, qj_push, qj_session_open, qj_string_free};
use serde_json::{Value, json};

// Session 在 C 侧是不透明指针，这里只传地址
#[allow(improper_ctypes)]
unsafe extern "C" {
    pub fn qj_scope_set(session: *mut Session, contact_id: *const c_char);
    pub fn qj_scope_get(session: *mut Session) -> *mut c_char;
    pub fn qj_reset_context(session: *mut Session);
    pub fn qj_memory_hint(session: *mut Session) -> *mut c_char;
    pub fn qj_memory_dismiss(session: *mut Session, card_id: *const c_char, today: bool);
    pub fn qj_memory_cards(session: *mut Session, contact_id: *const c_char) -> *mut c_char;
    pub fn qj_memory_note(
        session: *mut Session,
        contact_id: *const c_char,
        text: *const c_char,
        source: *const c_char,
    ) -> *mut c_char;
    pub fn qj_memory_add_contact(
        session: *mut Session,
        name: *const c_char,
        pronoun: *const c_char,
    ) -> *mut c_char;
    pub fn qj_memory_dropped(session: *mut Session) -> *mut c_char;
    pub fn qj_memory_read(user_dir: *const c_char) -> *mut c_char;
    pub fn qj_memory_write(user_dir: *const c_char, json: *const c_char) -> *mut c_char;
    pub fn qj_memory_materials(user_dir: *const c_char, contact_id: *const c_char) -> *mut c_char;
    pub fn qj_memory_material_delete(
        user_dir: *const c_char,
        contact_id: *const c_char,
        client_id: *const c_char,
    ) -> *mut c_char;
    pub fn qj_memory_unassigned_materials(user_dir: *const c_char) -> *mut c_char;
    pub fn qj_memory_unassigned_note(
        user_dir: *const c_char,
        text: *const c_char,
        source: *const c_char,
    ) -> *mut c_char;
    pub fn qj_memory_assign_material(
        user_dir: *const c_char,
        client_id: *const c_char,
        contact_id: *const c_char,
    ) -> *mut c_char;
    pub fn qj_memory_contact_skill(
        user_dir: *const c_char,
        contact_id: *const c_char,
    ) -> *mut c_char;
    pub fn qj_memory_contact_skill_set(
        user_dir: *const c_char,
        contact_id: *const c_char,
        skill_id: *const c_char,
    ) -> *mut c_char;
}

pub const CONTACT: &str = "0123456789abcdef0123456789abcdef";

pub const CARD: &str = "fedcba9876543210fedcba9876543210";

pub fn take(raw: *mut c_char) -> Option<String> {
    if raw.is_null() {
        return None;
    }
    let text = unsafe { CStr::from_ptr(raw) }
        .to_string_lossy()
        .into_owned();
    unsafe { qj_string_free(raw) };
    Some(text)
}

pub fn json_of(raw: *mut c_char) -> Value {
    serde_json::from_str(&take(raw).expect("应当返回 JSON")).unwrap()
}

pub fn c(text: &str) -> CString {
    CString::new(text).unwrap()
}

/// 临时的数据目录（样例词库与随包技能包）与学习数据目录。
pub fn dirs(name: &str) -> (PathBuf, PathBuf) {
    let root = std::env::temp_dir().join(format!("qj-memory-ffi-{name}-{}", std::process::id()));
    std::fs::remove_dir_all(&root).ok();
    let data = root.join("data");
    let user = root.join("user");
    std::fs::create_dir_all(&data).unwrap();
    std::fs::create_dir_all(&user).unwrap();
    let assets = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../assets");
    std::fs::copy(assets.join("sample/dict.tsv"), data.join("dict.qj")).unwrap();
    // 技能包：桥从 data_dir/skills 读，打包时由 scripts/build-bridge.sh 拷进去
    let skills = data.join("skills");
    std::fs::create_dir_all(&skills).unwrap();
    for entry in std::fs::read_dir(assets.join("skills")).unwrap().flatten() {
        std::fs::copy(entry.path(), skills.join(entry.file_name())).unwrap();
    }
    (data, user)
}

/// 经 `qj_memory_write` 放一个对象与一张带关键词「生日」的卡。
pub fn seed(user: &Path) {
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
        "contacts": [{"id": CONTACT, "name": "小美", "pronoun": "ta_f", "created_at": 1_791_043_200}],
        "cards": cards,
    });
    let dir = c(user.to_str().unwrap());
    let text = c(&snapshot.to_string());
    assert_eq!(
        take(unsafe { qj_memory_write(dir.as_ptr(), text.as_ptr()) }),
        None
    );
}

/// 键盘「记一笔」（手写来源）；NULL 即成功，这里给 `None`。
pub fn note(session: *mut Session, contact: &str, text: &str) -> Option<String> {
    let contact = c(contact);
    let text = c(text);
    take(unsafe { qj_memory_note(session, contact.as_ptr(), text.as_ptr(), ptr::null()) })
}

/// 模拟 App 占着 `memory/.lock`：持有返回的文件就是持有锁，丢掉即释放。
pub fn hold_lock(user: &Path) -> std::fs::File {
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
pub const KEYBOARD_BUDGET: Duration = Duration::from_millis(900);

pub fn open(data: &Path, user: Option<&Path>) -> *mut Session {
    let data = c(data.to_str().unwrap());
    let user = user.map(|dir| c(dir.to_str().unwrap()));
    let user_ptr = user.as_ref().map_or(ptr::null(), |dir| dir.as_ptr());
    let session = unsafe { qj_session_open(data.as_ptr(), user_ptr, ptr::null(), ptr::null()) };
    assert!(!session.is_null());
    session
}

/// 切当前对象；`None` 是空指针（保持现在选的人不变），`Some("")` 是不指定，其余是对象 id。
pub fn set_contact(session: *mut Session, contact: Option<&str>) {
    let contact = contact.map(c);
    let contact_ptr = contact.as_ref().map_or(ptr::null(), |id| id.as_ptr());
    unsafe { qj_scope_set(session, contact_ptr) };
}

pub fn type_and_commit(session: *mut Session, keys: &str) -> String {
    for key in keys.chars() {
        unsafe { qj_push(session, key as u32) };
    }
    take(unsafe { qj_commit(session, 0) }).unwrap()
}
