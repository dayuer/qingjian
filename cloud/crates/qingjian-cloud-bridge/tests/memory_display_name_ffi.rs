//! 键盘上的代号经 C 接口：旧文件读出来没有、App 读写来回一趟不丢、键盘新建对象不写、日子提醒里的名字用代号。

mod memory_support;

use std::path::Path;
use std::ptr;

use qingjian_cloud_bridge::{LocalDate, qj_session_free};
use serde_json::{Value, json};

use memory_support::{
    CONTACT, c, dirs, json_of, open, qj_memory_add_contact, qj_memory_hint, qj_memory_read,
    qj_memory_write, seed, set_scope, take,
};

fn read(user: &Path) -> Value {
    let dir = c(user.to_str().unwrap());
    json_of(unsafe { qj_memory_read(dir.as_ptr()) })
}

fn write(user: &Path, snapshot: &Value) -> Option<String> {
    let dir = c(user.to_str().unwrap());
    let text = c(&snapshot.to_string());
    take(unsafe { qj_memory_write(dir.as_ptr(), text.as_ptr()) })
}

#[test]
fn old_file_reads_without_display_name() {
    let (_, user) = dirs("display-old");
    seed(&user);
    let snapshot = read(&user);
    assert!(snapshot["contacts"][0].get("display_name").is_none());
}

#[test]
fn app_read_write_keeps_display_name() {
    let (_, user) = dirs("display-app");
    seed(&user);
    let mut snapshot = read(&user);
    snapshot["contacts"][0]["display_name"] = json!("阿美");
    assert_eq!(write(&user, &snapshot), None);
    let again = read(&user);
    assert_eq!(again["contacts"][0]["display_name"], "阿美");
    assert_eq!(write(&user, &again), None);
    assert_eq!(read(&user)["contacts"][0]["display_name"], "阿美");

    let mut long = read(&user);
    long["contacts"][0]["display_name"] = json!("一二三四五六七八九十一二三");
    let failure: Value = serde_json::from_str(&write(&user, &long).expect("超长应当拒绝")).unwrap();
    assert_eq!(failure["code"], "invalid");
    assert_eq!(
        read(&user)["contacts"][0]["display_name"],
        "阿美",
        "拒了就不动"
    );
}

#[test]
fn keyboard_added_contact_has_no_display_name() {
    let (data, user) = dirs("display-add");
    seed(&user);
    let session = open(&data, Some(&user));
    let name = c("阿杰");
    let added =
        json_of(unsafe { qj_memory_add_contact(session, name.as_ptr(), ptr::null(), ptr::null()) });
    unsafe { qj_session_free(session) };
    let id = added["id"].as_str().expect("建好了");
    let snapshot = read(&user);
    let person = snapshot["contacts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"] == id)
        .unwrap();
    assert!(person.get("display_name").is_none(), "{person}");
}

#[test]
fn reminder_uses_display_name_instead_of_name() {
    let (data, user) = dirs("display-hint");
    let tomorrow = LocalDate::today().add_days(1).to_string();
    let mut cards = serde_json::Map::new();
    cards.insert(
        CONTACT.to_owned(),
        json!([{
            "id": "fedcba9876543210fedcba9876543210", "kind": "date", "text": "生日", "keywords": [],
            "when": tomorrow, "source": "manual", "confirmed": true,
            "created_at": 1_791_043_200, "touched_at": 1_791_043_200
        }]),
    );
    let snapshot = json!({
        "contacts": [{"id": CONTACT, "name": "小美", "display_name": "阿美", "pronoun": "name",
                      "created_at": 1_791_043_200}],
        "cards": cards,
    });
    assert_eq!(write(&user, &snapshot), None);
    let session = open(&data, Some(&user));
    set_scope(session, "dating", Some(CONTACT));
    let hint = json_of(unsafe { qj_memory_hint(session) });
    unsafe { qj_session_free(session) };
    assert_eq!(hint["reason"], "today");
    assert_eq!(hint["text"], "明天是阿美的生日");
}
