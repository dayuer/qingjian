//! `cloud.toml`：没登录、旧令牌、格式不对都当离线；登录、改开关、退出登录只动该动的字段。

use std::path::{Path, PathBuf};

use qingjian_cloud_bridge::{AccountStatus, CloudConfig, DEFAULT_SERVER};
use qingjian_cloud_proto::Consents;

fn temp_file(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("qj-cloud-{name}-{}", std::process::id()));
    std::fs::remove_dir_all(&dir).ok();
    std::fs::create_dir_all(&dir).unwrap();
    dir.join("cloud.toml")
}

fn load(name: &str, text: &str) -> Option<CloudConfig> {
    let path = temp_file(name);
    std::fs::write(&path, text).unwrap();
    CloudConfig::load(&path)
}

#[test]
fn switches_default_to_off() {
    let config = load(
        "defaults",
        "server = \"https://pinyin.synon.ai/\"\ntoken = \"sjt_t\"\n",
    )
    .unwrap();
    assert!(!config.llm && !config.sync && !config.logs && !config.clipboard);
    assert_eq!(config.llm_base_url(), "https://pinyin.synon.ai/v1");
    assert!(!CloudConfig::default().clipboard);
}

#[test]
fn switches_are_respected() {
    let config = load(
        "switches",
        "server = \"s\"\ntoken = \"sjt_t\"\nllm = true\n",
    )
    .unwrap();
    assert!(config.llm && !config.sync);
    assert_eq!(
        config.consents(),
        Consents {
            llm: true,
            ..Consents::default()
        }
    );
}

#[test]
fn missing_or_old_token_or_bad_toml_means_offline() {
    assert!(load("no-token", "server = \"s\"\n").is_none());
    assert!(load("blank", "server = \"s\"\ntoken = \"  \"\n").is_none());
    assert!(load("old", "server = \"s\"\ntoken = \"qjc_old\"\nsync = true\n").is_none());
    assert!(load("bad", "server = ").is_none());
    assert!(CloudConfig::load(Path::new("/nonexistent/cloud.toml")).is_none());
}

#[test]
fn store_session_writes_server_token_and_consents() {
    let path = temp_file("session");
    std::fs::write(&path, "server = \"https://example.com\"\n").unwrap();
    let consents = Consents {
        sync: true,
        ..Consents::default()
    };
    CloudConfig::store_session(&path, "https://example.com", "sjt_abc", 7, consents).unwrap();
    let config = CloudConfig::load(&path).unwrap();
    assert_eq!(config.server, "https://example.com");
    assert_eq!(config.token, "sjt_abc");
    assert_eq!(config.user_id, Some(7));
    assert!(config.sync && !config.clipboard && !config.logs && !config.llm);
}

#[test]
fn store_consents_keeps_server_and_token() {
    let path = temp_file("consents");
    CloudConfig::store_session(
        &path,
        "https://example.com",
        "sjt_abc",
        7,
        Consents::default(),
    )
    .unwrap();
    let consents = Consents {
        clipboard: true,
        input_log: true,
        ..Consents::default()
    };
    CloudConfig::store_consents(&path, consents).unwrap();
    let config = CloudConfig::read(&path).unwrap();
    assert_eq!(config.token, "sjt_abc");
    assert_eq!(config.server, "https://example.com");
    assert!(config.clipboard && config.logs && !config.sync);
}

#[test]
fn clear_session_drops_token_and_switches_but_keeps_server() {
    let path = temp_file("clear");
    let all = Consents {
        clipboard: true,
        sync: true,
        input_log: true,
        llm: true,
        memory: true,
    };
    CloudConfig::store_session(&path, "https://example.com", "sjt_abc", 7, all).unwrap();
    assert_eq!(CloudConfig::read(&path).unwrap().consents(), all);
    CloudConfig::clear_session(&path).unwrap();
    let config = CloudConfig::read(&path).unwrap();
    assert!(config.token.is_empty());
    assert_eq!(config.consents(), Consents::default());
    assert_eq!(config.server, "https://example.com");
    // 退出登录留着 user_id：同一账号再登录不用清同步进度
    assert_eq!(config.user_id, Some(7));
    assert!(CloudConfig::load(&path).is_none());
    // 没有文件时退出登录也算成功
    assert!(CloudConfig::clear_session(&temp_file("clear-missing")).is_ok());
}

#[test]
fn status_without_token_stays_offline() {
    // 不写文件：没登录时只读文件、不联网
    let status = AccountStatus::load(&temp_file("status"));
    assert!(!status.signed_in);
    assert_eq!(status.server, DEFAULT_SERVER);
    assert!(status.error.is_none());
    assert!(status.sessions.is_empty() && status.identities.is_empty());
    let json = serde_json::to_value(&status).unwrap();
    assert_eq!(json["consents"]["input_log"], false);
    assert_eq!(json["signed_in"], false);
    assert!(json.get("token").is_none());
    assert!(json.get("error_code").is_none() && json.get("error").is_some_and(|v| v.is_null()));
}

#[test]
fn clear_account_also_forgets_user_id() {
    let path = temp_file("clear-account");
    CloudConfig::store_session(
        &path,
        "https://example.com",
        "sjt_abc",
        7,
        Consents::default(),
    )
    .unwrap();
    CloudConfig::clear_account(&path).unwrap();
    let config = CloudConfig::read(&path).unwrap();
    assert!(config.token.is_empty());
    assert_eq!(config.user_id, None);
    assert_eq!(config.server, "https://example.com");
    assert!(CloudConfig::clear_account(&temp_file("clear-account-missing")).is_ok());
}

#[test]
fn user_id_defaults_to_none_for_old_files() {
    let config = load("old-shape", "server = \"s\"\ntoken = \"sjt_t\"\n").unwrap();
    assert_eq!(config.user_id, None);
}

#[test]
fn store_consents_without_a_readable_file_fails_and_writes_nothing() {
    let path = temp_file("consents-missing");
    assert!(CloudConfig::store_consents(&path, Consents::default()).is_err());
    assert!(!path.exists());
    std::fs::write(&path, "server = ").unwrap();
    assert!(CloudConfig::store_consents(&path, Consents::default()).is_err());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "server = ");
}

#[cfg(unix)]
#[test]
fn save_is_private_atomic_and_complete() {
    use std::os::unix::fs::PermissionsExt;

    let path = temp_file("save");
    let long = CloudConfig {
        server: "https://example.com/".repeat(50),
        token: "sjt_".to_owned() + &"a".repeat(200),
        ..CloudConfig::default()
    };
    long.save(&path).unwrap();
    let short = CloudConfig {
        server: "s".into(),
        token: "sjt_b".into(),
        user_id: Some(3),
        ..CloudConfig::default()
    };
    short.save(&path).unwrap();
    let mode = std::fs::metadata(&path).unwrap().permissions().mode();
    assert_eq!(mode & 0o777, 0o600);
    assert_eq!(CloudConfig::read(&path).unwrap(), short);
    let names: Vec<_> = std::fs::read_dir(path.parent().unwrap())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names, ["cloud.toml"]);
}

#[test]
fn parse_failure_note_does_not_leak_the_line() {
    let text = "server = \"s\"\ntoken = \"sjt_abc\" oops\n";
    let error = toml::from_str::<CloudConfig>(text).unwrap_err();
    let note = qingjian_cloud_bridge::parse_failure_note(&error);
    assert!(!note.is_empty());
    assert!(!note.contains("sjt_abc"), "{note}");
}

#[cfg(unix)]
#[test]
fn concurrent_writes_stay_valid_private_and_leave_no_tmp() {
    use std::os::unix::fs::PermissionsExt;

    let path = temp_file("concurrent");
    CloudConfig::store_session(
        &path,
        "https://example.com",
        "sjt_abc",
        7,
        Consents::default(),
    )
    .unwrap();
    let handles: Vec<_> = (0..8)
        .map(|n| {
            let path = path.clone();
            std::thread::spawn(move || {
                for i in 0..25 {
                    let consents = Consents {
                        sync: (n + i) % 2 == 0,
                        ..Consents::default()
                    };
                    if i % 5 == 0 {
                        CloudConfig::store_session(
                            &path,
                            "https://example.com",
                            "sjt_abc",
                            7,
                            consents,
                        )
                        .unwrap();
                    } else {
                        CloudConfig::store_consents(&path, consents).unwrap();
                    }
                }
            })
        })
        .collect();
    for handle in handles {
        handle.join().unwrap();
    }
    let config = CloudConfig::read(&path).unwrap();
    assert_eq!(config.token, "sjt_abc");
    assert_eq!(config.user_id, Some(7));
    assert_eq!(
        std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
        0o600
    );
    let names: Vec<_> = std::fs::read_dir(path.parent().unwrap())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names, ["cloud.toml"]);
}

/// `qj_cloud_configured`：有地址与登录令牌才算开了素笺云（App 的「待整理」引导与键盘 toast 按它），不联网。
#[test]
fn cloud_configured_needs_server_and_session_token() {
    use std::ffi::CString;

    use qingjian_cloud_bridge::qj_cloud_configured;

    let check = |name: &str, text: Option<&str>| {
        let path = temp_file(name);
        if let Some(text) = text {
            std::fs::write(&path, text).unwrap();
        }
        let path = CString::new(path.to_str().unwrap()).unwrap();
        unsafe { qj_cloud_configured(path.as_ptr()) }
    };
    assert!(check(
        "configured",
        Some("server = \"s\"\ntoken = \"sjt_t\"\n")
    ));
    assert!(!check("configured-missing", None));
    assert!(
        !check("configured-seed", Some("server = \"s\"\n")),
        "随包的种子只有地址"
    );
    assert!(!check(
        "configured-old",
        Some("server = \"s\"\ntoken = \"qjc_t\"\n")
    ));
    assert!(!check("configured-broken", Some("server = ")));
    assert!(!unsafe { qj_cloud_configured(std::ptr::null()) });
}
