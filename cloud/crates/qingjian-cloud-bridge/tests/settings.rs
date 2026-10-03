//! 设置页读写 `config.toml`：改过的项写回后读出来一样，文件里别的内容（Mac 专属设置、注释）保留。

use std::path::PathBuf;

use qingjian_cloud_bridge::{CloudConfig, CloudStatus, CloudSwitches, Session, Settings};
use qingjian_core::CustomPhrase;

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("qj-settings-{name}-{}", std::process::id()));
    std::fs::remove_dir_all(&dir).ok();
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn round_trips_and_keeps_other_settings() {
    let dir = temp_dir("round");
    let config = dir.join("config.toml");
    std::fs::write(&config, "# 我的注释\n[general]\nmenu_bar_icon = false\n").unwrap();

    let mut settings = Settings::read(&config, &dir.join("dicts"));
    assert_eq!(settings.scheme, "pinyin");
    assert!(settings.schemes.iter().any(|s| s.key == "xiaohe"));
    settings.scheme = "xiaohe".into();
    settings.traditional = true;
    settings.fuzzy.z_zh = true;
    settings.phrases = vec![CustomPhrase {
        code: "dz".into(),
        text: "北京市海淀区".into(),
        position: 1,
        enabled: true,
    }];
    settings.write(&config).unwrap();

    let read = Settings::read(&config, &dir.join("dicts"));
    assert_eq!(read.scheme, "xiaohe");
    assert!(read.traditional);
    assert!(read.fuzzy.z_zh);
    assert_eq!(read.phrases, settings.phrases);
    let text = std::fs::read_to_string(&config).unwrap();
    assert!(text.contains("# 我的注释"));
    assert!(text.contains("menu_bar_icon = false"));
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn rejects_invalid_phrases_without_writing() {
    let dir = temp_dir("invalid");
    let config = dir.join("config.toml");
    let mut settings = Settings::read(&config, &dir);
    settings.traditional = true;
    settings.phrases = vec![CustomPhrase {
        code: "D Z".into(),
        text: "x".into(),
        position: 1,
        enabled: true,
    }];
    assert!(settings.write(&config).is_err());
    assert!(!Settings::read(&config, &dir).traditional);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn cloud_config_saves_incomplete_values() {
    let dir = temp_dir("cloud");
    let path = dir.join("cloud.toml");
    let config = CloudConfig {
        server: "https://example.com".into(),
        candidates: true,
        ..CloudConfig::default()
    };
    config.save(&path).unwrap();
    assert_eq!(CloudConfig::read(&path), Some(config));
    // 没填令牌：设置页能读回来，键盘按离线用
    assert!(CloudConfig::load(&path).is_none());
    std::fs::remove_dir_all(&dir).ok();
}

/// 用真实产品数据：键盘会话按 `config.toml` 套用繁体，主 App 改了文件后轮询重读。
#[test]
fn session_applies_config_changes() {
    let Some(data) = std::env::var_os("QINGJIAN_DATA").map(PathBuf::from) else {
        eprintln!("没有 QINGJIAN_DATA，跳过");
        return;
    };
    if !data.join("dict.qj").is_file() {
        return;
    }
    let user = temp_dir("session");
    let config = user.join("config.toml");
    let mut session = Session::open(&data, Some(&user), None, None).unwrap();
    let first = |session: &mut Session| {
        session.clear();
        "zhongguo".chars().for_each(|c| session.push(c));
        session.entries()[0].text().to_owned()
    };
    assert_eq!(first(&mut session), "中国");

    let mut settings = Settings::read(&config, &data.join("dicts"));
    settings.traditional = true;
    settings.write(&config).unwrap();
    session.poll();
    assert_eq!(first(&mut session), "中國");
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn switches_keep_server_and_token() {
    let dir = temp_dir("switches");
    let path = dir.join("cloud.toml");
    let config = CloudConfig {
        server: "https://example.com".into(),
        token: "secret".into(),
        ..CloudConfig::default()
    };
    config.save(&path).unwrap();
    let mut switches = CloudSwitches::of(&config);
    switches.clipboard = false;
    CloudConfig::save_switches(&path, switches).unwrap();
    let saved = CloudConfig::read(&path).unwrap();
    assert_eq!(saved.token, "secret");
    assert_eq!(saved.server, "https://example.com");
    assert!(!saved.clipboard);
    let status = serde_json::to_string(&CloudStatus::of(&saved)).unwrap();
    assert!(status.contains("\"connected\":true"));
    assert!(!status.contains("secret"));
    std::fs::remove_dir_all(&dir).ok();
}
