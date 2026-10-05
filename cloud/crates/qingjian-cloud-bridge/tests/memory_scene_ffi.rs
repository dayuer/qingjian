//! 场景（C 接口）：键盘按场景新建、人数不限；切场景回到上次选的人；
//! 每个场景一样——都出提示、都写全局与对象层（场景自 2026-10-05 起只是用户自建的分组）。

mod memory_support;

use std::path::Path;
use std::ptr;

use qingjian_cloud_bridge::{qj_flush, qj_session_free};
use serde_json::{Value, json};

use memory_support::{
    c, dirs, json_of, note, open, qj_memory_add_contact, qj_memory_hint, qj_memory_read,
    qj_memory_write, qj_scope_get, scenes, seed, set_scope, take, type_and_commit,
};

const DAILY: &str = "11111111111111111111111111111111";

const WORK: &str = "22222222222222222222222222222222";

const DATING: &str = "0123456789abcdef0123456789abcdef";

/// 每个场景各一个人，各有一张带关键词「生日」的卡。
fn seed_scenes(user: &Path) {
    let card = |n: u32| {
        json!([{
            "id": format!("{:032x}", 1000 + n), "kind": "other", "text": "想要一个生日蛋糕", "keywords": ["生日"],
            "when": null, "source": "manual", "confirmed": true,
            "created_at": 1_791_043_200, "touched_at": 1_791_043_200
        }])
    };
    let person = |id: &str, name: &str, scene: &str| json!({"id": id, "name": name, "pronoun": "ta", "scene": scene, "created_at": 1_791_043_200});
    let snapshot = json!({
        "scenes": scenes(),
        "contacts": [person(DATING, "小美", "dating"), person(DAILY, "妈妈", "daily"), person(WORK, "老板", "work")],
        "cards": {DATING: card(1), DAILY: card(2), WORK: card(3)},
    });
    let dir = c(user.to_str().unwrap());
    let text = c(&snapshot.to_string());
    assert_eq!(
        take(unsafe { qj_memory_write(dir.as_ptr(), text.as_ptr()) }),
        None
    );
}

fn add(session: *mut qingjian_cloud_bridge::Session, name: &str, scene: Option<&str>) -> Value {
    let name = c(name);
    let scene = scene.map(c);
    let scene_ptr = scene.as_ref().map_or(ptr::null(), |s| s.as_ptr());
    json_of(unsafe { qj_memory_add_contact(session, name.as_ptr(), ptr::null(), scene_ptr) })
}

fn scene_of(user: &Path, id: &str) -> String {
    let dir = c(user.to_str().unwrap());
    let snapshot = json_of(unsafe { qj_memory_read(dir.as_ptr()) });
    snapshot["contacts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == id)
        .expect("名单里有这个人")["scene"]
        .as_str()
        .unwrap()
        .to_owned()
}

#[test]
fn keyboard_adds_contacts_into_the_given_scene() {
    let (data, user) = dirs("scene-add");
    seed(&user);
    let session = open(&data, Some(&user));
    let name = c("  阿杰 ");
    let pronoun = c("ta_m");
    let daily = c("daily");
    let added = json_of(unsafe {
        qj_memory_add_contact(session, name.as_ptr(), pronoun.as_ptr(), daily.as_ptr())
    });
    let id = added["id"].as_str().expect("建好返回 id").to_owned();
    assert_eq!(id.len(), 32);
    assert_eq!(scene_of(&user, &id), "daily");
    assert!(user.join("memory").join(&id).is_dir(), "对象目录建好了");
    set_scope(session, "daily", Some(&id));
    assert_eq!(
        json_of(unsafe { qj_scope_get(session) })["contact_id"],
        id.as_str(),
        "新建的人马上能在这个场景选"
    );
    let dir = c(user.to_str().unwrap());
    let snapshot = json_of(unsafe { qj_memory_read(dir.as_ptr()) });
    let contact = snapshot["contacts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == id.as_str())
        .unwrap()
        .clone();
    assert_eq!(contact["name"], "阿杰", "名字去掉首尾空白");
    assert_eq!(contact["pronoun"], "ta_m");

    // 场景传空指针或不合格式：用会话当前的场景
    set_scope(session, "work", None);
    let id = add(session, "老王", None)["id"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_eq!(scene_of(&user, &id), "work");
    let id = add(session, "老李", Some("Party"))["id"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_eq!(scene_of(&user, &id), "work");

    assert_eq!(add(session, "   ", Some("daily"))["code"], "invalid");
    assert_eq!(
        add(ptr::null_mut(), "阿杰", Some("daily"))["code"],
        "invalid"
    );
    unsafe { qj_session_free(session) };
}

#[test]
fn keyboard_adds_contacts_without_a_cap() {
    let (data, user) = dirs("scene-add-limit");
    seed(&user);
    let session = open(&data, Some(&user));
    for n in 0..12 {
        let added = add(session, &format!("日常{n}"), Some("daily"));
        assert!(added["id"].is_string(), "第 {n} 个应当建得了：{added}");
    }
    assert!(add(session, "恋爱0", Some("dating"))["id"].is_string());
    unsafe { qj_session_free(session) };
    let dir = c(user.to_str().unwrap());
    let snapshot = json_of(unsafe { qj_memory_read(dir.as_ptr()) });
    assert_eq!(
        snapshot["contacts"].as_array().unwrap().len(),
        14,
        "12 个新的 + 种子那 1 个 + 恋爱 1 个"
    );
}

#[test]
fn switching_scene_without_a_contact_returns_to_the_last_pick() {
    let (data, user) = dirs("scene-last");
    seed_scenes(&user);
    let session = open(&data, Some(&user));
    set_scope(session, "dating", Some(DATING));
    set_scope(session, "daily", Some(DAILY));
    set_scope(session, "work", None);
    let scope = json_of(unsafe { qj_scope_get(session) });
    assert_eq!(scope["contact_id"], Value::Null, "工作还没选过人");
    assert_eq!(scope["last"]["dating"], DATING);
    assert_eq!(scope["last"]["daily"], DAILY);
    assert!(
        scope["used"][DAILY]
            .as_i64()
            .is_some_and(|at| at > 1_700_000_000),
        "选中时记下了时间：{scope}"
    );
    set_scope(session, "dating", None);
    assert_eq!(
        json_of(unsafe { qj_scope_get(session) })["contact_id"],
        DATING
    );

    // 空字符串是明确不指定，也记成这个场景的上次
    set_scope(session, "dating", Some(""));
    assert_eq!(
        json_of(unsafe { qj_scope_get(session) })["contact_id"],
        Value::Null
    );
    set_scope(session, "daily", None);
    assert_eq!(
        json_of(unsafe { qj_scope_get(session) })["contact_id"],
        DAILY
    );
    set_scope(session, "dating", None);
    assert_eq!(
        json_of(unsafe { qj_scope_get(session) })["contact_id"],
        Value::Null
    );
    unsafe { qj_session_free(session) };

    // 重开会话还记得
    let session = open(&data, Some(&user));
    set_scope(session, "daily", None);
    assert_eq!(
        json_of(unsafe { qj_scope_get(session) })["contact_id"],
        DAILY
    );
    unsafe { qj_session_free(session) };
}

#[test]
fn every_scene_shows_hints_and_takes_notes() {
    let (data, user) = dirs("scene-hints");
    seed_scenes(&user);
    let session = open(&data, Some(&user));
    for (scene, who) in [("daily", DAILY), ("work", WORK)] {
        set_scope(session, scene, Some(who));
        assert_eq!(json_of(unsafe { qj_scope_get(session) })["contact_id"], who);
        assert_eq!(type_and_commit(session, "shengri"), "生日");
        assert_eq!(
            json_of(unsafe { qj_memory_hint(session) })["text"],
            "想要一个生日蛋糕",
            "{scene} 也出提示"
        );
        assert_eq!(note(session, who, "周五前交方案"), None);
        let materials =
            std::fs::read_to_string(user.join("memory").join(who).join("materials.jsonl")).unwrap();
        assert!(materials.contains("周五前交方案"), "{scene} 也能记一笔");
    }
    unsafe { qj_session_free(session) };
}

/// 学习数据目录或某层目录下的 `user.tsv` 里有没有这个词。
fn learned(dir: &Path, word: &str) -> bool {
    std::fs::read_to_string(dir.join("user.tsv")).is_ok_and(|text| text.contains(word))
}

#[test]
fn every_scene_learns_into_global_and_contact_layers() {
    for (name, scene, who) in [("daily", "daily", DAILY), ("dating", "dating", DATING)] {
        let (data, user) = dirs(&format!("scene-learn-{name}"));
        seed_scenes(&user);
        let session = open(&data, Some(&user));
        set_scope(session, scene, Some(who));
        assert_eq!(type_and_commit(session, "shengri"), "生日");
        unsafe { qj_flush(session) };
        let memory = user.join("memory");
        assert!(learned(&user, "生日"), "{scene} 写全局");
        assert!(
            learned(&memory.join(who).join("learning"), "生日"),
            "{scene} 写对象层"
        );
        assert!(!memory.join("scene-daily").exists(), "不再有场景层");
        assert!(!memory.join("scene-dating").exists(), "不再有场景层");
        unsafe { qj_session_free(session) };
    }
}
