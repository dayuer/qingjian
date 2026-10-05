//! 改写的 C 接口：技能列表与默认技能（每个人自己的技能在 Task 6 补上）。

mod memory_support;

use qingjian_cloud_bridge::{
    qj_rewrite_skills, qj_session_free, qj_settings_read, qj_settings_write,
};
use serde_json::{Value, json};

use memory_support::{c, dirs, json_of, open, take};

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
