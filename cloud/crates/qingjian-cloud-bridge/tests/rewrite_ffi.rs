//! 改写的 C 接口：技能列表、默认技能、每个人自己的技能。

mod memory_support;

use std::ptr;

use qingjian_cloud_bridge::{
    qj_rewrite_default, qj_rewrite_default_set, qj_rewrite_skills, qj_session_free,
    qj_settings_read, qj_settings_write,
};
use serde_json::{Value, json};

use memory_support::{
    CONTACT, c, dirs, json_of, open, qj_memory_contact_skill, qj_memory_contact_skill_set, seed,
    take,
};

#[test]
fn skills_come_from_the_bundled_packs() {
    let (data, user) = dirs("rewrite-skills");
    let session = open(&data, Some(&user));
    let skills: Value = json_of(unsafe { qj_rewrite_skills(session) });
    let ids: Vec<&str> = skills
        .as_array()
        .expect("技能列表")
        .iter()
        .map(|skill| skill["id"].as_str().unwrap())
        .collect();
    assert!(ids.contains(&"polish"), "{ids:?}");
    assert_eq!(skills[0]["id"], "polish", "按 order 排，润色在前");
    assert_eq!(skills[0]["name"], "润色");
    assert_eq!(skills[0]["summary"], "改通顺，意思不变");
    assert_eq!(skills[0].get("prompt"), None, "提示词不下发到壳里");
    unsafe { qj_session_free(session) };
}

/// 技能包没打进包（数据目录里没有 `skills/`）：列表为空指针，改写整块不出现。
#[test]
fn no_skill_packs_means_no_list() {
    let (data, user) = dirs("rewrite-no-skills");
    std::fs::remove_dir_all(data.join("skills")).unwrap();
    let session = open(&data, Some(&user));
    assert!(unsafe { qj_rewrite_skills(session) }.is_null());
    unsafe { qj_session_free(session) };
}

/// 默认技能记在设置里（`config.toml` 的 `[rewrite] skill`）；老配置文件没有这一项时按缺省。
#[test]
fn the_default_skill_lives_in_settings() {
    let (data, user) = dirs("rewrite-setting");
    let config = user.join("config.toml");
    let config_arg = c(config.to_str().unwrap());
    let dicts_arg = c(data.join("dicts").to_str().unwrap());
    let read = || json_of(unsafe { qj_settings_read(config_arg.as_ptr(), dicts_arg.as_ptr()) });

    assert_eq!(read()["rewrite_skill"], "polish", "缺省是润色");

    let mut settings = read();
    settings["rewrite_skill"] = json!("tactful");
    let text = c(&settings.to_string());
    assert_eq!(
        take(unsafe { qj_settings_write(config_arg.as_ptr(), text.as_ptr()) }),
        None
    );
    assert_eq!(read()["rewrite_skill"], "tactful");

    // 老配置文件（这一版之前写的）里没有 `[rewrite]`，读出来还是缺省
    std::fs::write(&config, "[general]\nscheme = \"xiaohe\"\n").unwrap();
    let settings = read();
    assert_eq!(settings["scheme"], "xiaohe", "别的设置照读");
    assert_eq!(settings["rewrite_skill"], "polish");
    std::fs::remove_dir_all(&user).ok();
}

/// 键盘侧读写默认技能：读 `qj_rewrite_default`、写 `qj_rewrite_default_set`，
/// 只碰 `config.toml` 里的 `[rewrite] skill`，别的内容与注释原样。
#[test]
fn the_keyboard_reads_and_writes_the_default_skill() {
    let (data, user) = dirs("rewrite-default");
    let config = user.join("config.toml");
    std::fs::write(&config, "# 我的注释\n[general]\nscheme = \"xiaohe\"\n").unwrap();
    let session = open(&data, Some(&user));

    let read = || json_of(unsafe { qj_rewrite_default(session) });
    assert_eq!(read()["skill"], "polish", "没配过时是缺省");

    let tactful = c("tactful");
    assert_eq!(
        take(unsafe { qj_rewrite_default_set(session, tactful.as_ptr()) }),
        None
    );
    assert_eq!(read()["skill"], "tactful");
    let text = std::fs::read_to_string(&config).unwrap();
    assert!(text.contains("# 我的注释"), "注释与别的内容原样：{text}");
    assert!(text.contains("scheme = \"xiaohe\""), "别的设置原样：{text}");
    assert!(text.contains("[rewrite]\nskill = \"tactful\""), "{text}");

    // 空指针 = 回到缺省
    assert_eq!(
        take(unsafe { qj_rewrite_default_set(session, ptr::null()) }),
        None
    );
    assert_eq!(read()["skill"], "polish");

    // 形状不合法的编号不收（存不存在不管，格式要管）
    let bad = c("X Y");
    assert_eq!(
        json_of(unsafe { qj_rewrite_default_set(session, bad.as_ptr()) })["code"],
        "invalid"
    );
    assert_eq!(read()["skill"], "polish", "没写进去");

    unsafe { qj_session_free(session) };
    std::fs::remove_dir_all(&user).ok();
}

/// 文件里的默认技能被别处写坏（不是合法编号）：读出来报 `invalid`，不当成缺省静默过去；会话无效时读是空指针。
#[test]
fn a_broken_default_skill_is_reported() {
    let (data, user) = dirs("rewrite-default-broken");
    std::fs::write(user.join("config.toml"), "[rewrite]\nskill = \"X Y\"\n").unwrap();
    let session = open(&data, Some(&user));
    assert_eq!(
        json_of(unsafe { qj_rewrite_default(session) })["code"],
        "invalid"
    );
    assert!(unsafe { qj_rewrite_default(ptr::null_mut()) }.is_null());
    unsafe { qj_session_free(session) };
    std::fs::remove_dir_all(&user).ok();
}

/// 每个人可以指定自己的技能；不指定（或清掉）就用设置里的默认。
#[test]
fn a_contact_can_pick_their_own_skill() {
    let (_, user) = dirs("contact-skill");
    seed(&user);
    let dir = c(user.to_str().unwrap());
    let contact = c(CONTACT);

    let read = || json_of(unsafe { qj_memory_contact_skill(dir.as_ptr(), contact.as_ptr()) });
    assert_eq!(read()["skill"], Value::Null, "一开始没指定");

    let tactful = c("tactful");
    assert_eq!(
        take(unsafe {
            qj_memory_contact_skill_set(dir.as_ptr(), contact.as_ptr(), tactful.as_ptr())
        }),
        None
    );
    assert_eq!(read()["skill"], "tactful");

    // 清掉：空指针 = 回到默认
    assert_eq!(
        take(unsafe { qj_memory_contact_skill_set(dir.as_ptr(), contact.as_ptr(), ptr::null()) }),
        None
    );
    assert_eq!(read()["skill"], Value::Null);

    // 不合法的技能 id 不收
    let bad = c("X Y");
    let failure = json_of(unsafe {
        qj_memory_contact_skill_set(dir.as_ptr(), contact.as_ptr(), bad.as_ptr())
    });
    assert_eq!(failure["code"], "invalid");

    // 名单上没有这个人
    let ghost = c("ffffffffffffffffffffffffffffffff");
    assert!(unsafe { qj_memory_contact_skill(dir.as_ptr(), ghost.as_ptr()) }.is_null());
    let failure = json_of(unsafe {
        qj_memory_contact_skill_set(dir.as_ptr(), ghost.as_ptr(), tactful.as_ptr())
    });
    assert_eq!(failure["code"], "invalid");
    std::fs::remove_dir_all(&user).ok();
}
