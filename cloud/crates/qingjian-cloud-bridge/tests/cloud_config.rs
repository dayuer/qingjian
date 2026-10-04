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
    CloudConfig::store_session(&path, "https://example.com", "sjt_abc", consents).unwrap();
    let config = CloudConfig::load(&path).unwrap();
    assert_eq!(config.server, "https://example.com");
    assert_eq!(config.token, "sjt_abc");
    assert!(config.sync && !config.clipboard && !config.logs && !config.llm);
}

#[test]
fn store_consents_keeps_server_and_token() {
    let path = temp_file("consents");
    CloudConfig::store_session(&path, "https://example.com", "sjt_abc", Consents::default())
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
    };
    CloudConfig::store_session(&path, "https://example.com", "sjt_abc", all).unwrap();
    CloudConfig::clear_session(&path).unwrap();
    let config = CloudConfig::read(&path).unwrap();
    assert!(config.token.is_empty());
    assert_eq!(config.consents(), Consents::default());
    assert_eq!(config.server, "https://example.com");
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
}
