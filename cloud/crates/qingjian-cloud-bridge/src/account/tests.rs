//! 错误文案的映射与「关掉 / 打开同步」时只清学习数据与配置进度。

use std::path::PathBuf;

use qingjian_cloud_client::ClientError;

use qingjian_cloud_proto::{Consents, Feature};

use super::failure::{
    Failure, LOCKED_TODAY, apple_message, code_of, email_start_message, email_verify_message,
    message,
};
use super::reset::{reset_account_data, reset_after_sync_toggle, should_reset, sync_toggled};
use super::{apply_server_consents, forget_account, store_login};
use crate::cloud_config::CloudConfig;

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("qj-account-{name}-{}", std::process::id()));
    std::fs::remove_dir_all(&dir).ok();
    std::fs::create_dir_all(dir.join("cloud")).unwrap();
    dir
}

#[test]
fn messages_distinguish_login_failure_from_expired_session() {
    assert_eq!(
        message(&ClientError::Unauthorized),
        "登录已失效，请重新登录"
    );
    let auth_failed = message(&ClientError::AuthFailed("bad code".into()));
    assert_ne!(auth_failed, message(&ClientError::Unauthorized));
    assert!(!message(&ClientError::NotConfigured("no smtp".into())).is_empty());
    assert_eq!(message(&ClientError::RateLimited), "操作太频繁，请稍后再试");
}

#[test]
fn forbidden_never_shows_server_text() {
    assert_eq!(
        message(&ClientError::Forbidden("feature not enabled: sync".into())),
        "这项功能还没打开"
    );
    assert_eq!(
        message(&ClientError::Forbidden(String::new())),
        "这项功能还没打开"
    );
}

#[test]
fn client_errors_map_to_codes() {
    let cases = [
        (ClientError::AuthFailed("x".into()), "auth_failed"),
        (ClientError::LockedToday("x".into()), "locked_today"),
        (ClientError::Unauthorized, "unauthorized"),
        (ClientError::NotConfigured("x".into()), "not_configured"),
        (ClientError::RateLimited, "rate_limited"),
        (ClientError::Forbidden("x".into()), "forbidden"),
        (ClientError::Unreachable("x".into()), "unreachable"),
        (ClientError::Io(std::io::Error::other("x")), "unreachable"),
        (
            ClientError::Rejected {
                status: 400,
                message: String::new(),
            },
            "other",
        ),
        (ClientError::BadResponse("x".into()), "other"),
    ];
    for (error, code) in cases {
        assert_eq!(code_of(&error), code, "{error}");
    }
}

#[test]
fn failure_json_has_code_and_message() {
    let json = Failure::new("locked_today", LOCKED_TODAY.to_owned()).to_json();
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(value["code"], "locked_today");
    assert_eq!(value["message"], LOCKED_TODAY);
    assert_eq!(Failure::invalid_argument().code, "invalid_argument");
    assert_eq!(Failure::not_signed_in().code, "not_signed_in");
    assert_eq!(Failure::other("x").code, "other");
}

fn seed_account_a(name: &str) -> (PathBuf, PathBuf) {
    let dir = temp_dir(name);
    let path = dir.join("cloud.toml");
    CloudConfig::store_session(&path, "https://x", "sjt_a", 1, Consents::default()).unwrap();
    std::fs::write(dir.join("input-log.jsonl"), "A 的明文").unwrap();
    std::fs::write(dir.join("input-log.jsonl.1"), "A 的轮转").unwrap();
    std::fs::write(dir.join("cloud/input-log-state.json"), "{}").unwrap();
    std::fs::write(dir.join("cloud/outbox.jsonl"), "A 的离线事件").unwrap();
    std::fs::write(dir.join("user.tsv"), "学习数据").unwrap();
    (dir, path)
}

fn account_data_left(dir: &std::path::Path) -> bool {
    [
        "input-log.jsonl",
        "input-log.jsonl.1",
        "cloud/input-log-state.json",
        "cloud/outbox.jsonl",
    ]
    .iter()
    .any(|name| dir.join(name).exists())
}

#[test]
fn switching_account_deletes_input_log_and_progress() {
    let (dir, path) = seed_account_a("switch");
    store_login(&path, "https://x", "sjt_b", 2, Consents::default()).unwrap();
    assert!(!account_data_left(&dir));
    assert!(dir.join("user.tsv").exists(), "本机学习数据不是账号数据");
    assert_eq!(CloudConfig::read(&path).unwrap().user_id, Some(2));
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn same_account_relogin_keeps_input_log_and_progress() {
    let (dir, path) = seed_account_a("relogin");
    CloudConfig::clear_session(&path).unwrap();
    store_login(&path, "https://x", "sjt_a2", 1, Consents::default()).unwrap();
    assert!(dir.join("input-log.jsonl").exists());
    assert!(dir.join("cloud/input-log-state.json").exists());
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn deleting_account_removes_input_log_and_progress() {
    let (dir, path) = seed_account_a("delete");
    forget_account(&path).unwrap();
    assert!(!account_data_left(&dir));
    let config = CloudConfig::read(&path).unwrap();
    assert!(config.token.is_empty());
    assert_eq!(config.user_id, None);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn reset_account_data_tolerates_missing_files() {
    let dir = temp_dir("missing");
    reset_account_data(&dir.join("cloud.toml"));
    reset_account_data(&dir.join("nope/cloud.toml"));
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn reset_removes_learning_and_config_progress_but_keeps_the_rest() {
    let dir = temp_dir("reset");
    let state = dir.join("cloud");
    let gone = [
        "learning-base.json",
        "learning-state.json",
        "config-state.json",
    ];
    let kept = [
        "state.json",
        "outbox.jsonl",
        "input-log-state.json",
        "clipboard.json",
    ];
    for name in gone.iter().chain(&kept) {
        std::fs::write(state.join(name), "x").unwrap();
    }
    reset_after_sync_toggle(&dir.join("cloud.toml"));
    for name in gone {
        assert!(!state.join(name).exists(), "{name} 应该被删");
    }
    for name in kept {
        assert!(state.join(name).exists(), "{name} 应该还在");
    }
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn same_account_keeps_progress_other_account_clears_it() {
    assert!(!should_reset(Some(7), 7, true));
    assert!(should_reset(Some(7), 8, true));
    assert!(should_reset(Some(7), 8, false));
    // 没有旧 user_id：有旧进度就当换了账号，没有就无事可清
    assert!(should_reset(None, 7, true));
    assert!(!should_reset(None, 7, false));
}

#[test]
fn sign_out_then_same_account_does_not_reset_but_delete_does() {
    let dir = temp_dir("flow");
    let path = dir.join("cloud.toml");
    CloudConfig::store_session(&path, "https://x", "sjt_a", 7, Consents::default()).unwrap();
    let has_progress = dir.join("cloud").exists();
    assert!(has_progress);

    CloudConfig::clear_session(&path).unwrap();
    let previous = CloudConfig::read(&path).unwrap().user_id;
    assert!(!should_reset(previous, 7, has_progress), "同账号重登不清");
    assert!(should_reset(previous, 8, has_progress), "换账号要清");

    CloudConfig::clear_account(&path).unwrap();
    let previous = CloudConfig::read(&path).unwrap().user_id;
    assert!(
        should_reset(previous, 7, has_progress),
        "删号后重登按换账号"
    );
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn only_a_changed_sync_value_resets() {
    assert!(!sync_toggled(Feature::Sync, true, true));
    assert!(!sync_toggled(Feature::Sync, false, false));
    assert!(sync_toggled(Feature::Sync, true, false));
    assert!(sync_toggled(Feature::Sync, false, true));
    assert!(!sync_toggled(Feature::Clipboard, true, false));
    assert!(!sync_toggled(Feature::Llm, false, true));
}

fn signed_in_dir(name: &str, sync: bool, llm: bool) -> (PathBuf, PathBuf) {
    let dir = temp_dir(name);
    let path = dir.join("cloud.toml");
    let consents = Consents {
        sync,
        llm,
        ..Consents::default()
    };
    CloudConfig::store_session(&path, "https://x", "sjt_a", 7, consents).unwrap();
    std::fs::write(dir.join("cloud/learning-base.json"), "x").unwrap();
    std::fs::write(dir.join("cloud/outbox.jsonl"), "x").unwrap();
    (dir, path)
}

#[test]
fn server_side_sync_change_resets_progress_both_ways() {
    for (was, now) in [(false, true), (true, false)] {
        let (dir, path) = signed_in_dir("srv-sync", was, false);
        let server = Consents {
            sync: now,
            ..Consents::default()
        };
        assert!(apply_server_consents(&path, server));
        assert!(!dir.join("cloud/learning-base.json").exists());
        assert!(dir.join("cloud/outbox.jsonl").exists());
        assert_eq!(CloudConfig::read(&path).unwrap().consents(), server);
        std::fs::remove_dir_all(&dir).ok();
    }
}

#[test]
fn server_side_change_without_sync_keeps_progress() {
    let (dir, path) = signed_in_dir("srv-llm", true, false);
    let server = Consents {
        sync: true,
        llm: true,
        ..Consents::default()
    };
    assert!(!apply_server_consents(&path, server));
    assert!(dir.join("cloud/learning-base.json").exists());
    assert!(CloudConfig::read(&path).unwrap().llm);
    // 完全没变也不清
    assert!(!apply_server_consents(&path, server));
    assert!(dir.join("cloud/learning-base.json").exists());
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn server_consents_ignored_when_not_signed_in() {
    let dir = temp_dir("srv-none");
    let path = dir.join("cloud.toml");
    let all = Consents {
        sync: true,
        ..Consents::default()
    };
    assert!(!apply_server_consents(&path, all));
    assert!(!path.exists());
    CloudConfig::store_session(&path, "https://x", "sjt_a", 7, Consents::default()).unwrap();
    CloudConfig::clear_session(&path).unwrap();
    std::fs::write(dir.join("cloud/learning-base.json"), "x").unwrap();
    assert!(!apply_server_consents(&path, all));
    assert!(dir.join("cloud/learning-base.json").exists());
    assert!(!CloudConfig::read(&path).unwrap().sync);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn email_verify_messages() {
    assert_eq!(
        email_verify_message(&ClientError::AuthFailed("x".into())),
        "验证码不对，请重新输入"
    );
    assert_eq!(
        email_verify_message(&ClientError::LockedToday("x".into())),
        LOCKED_TODAY
    );
    assert_eq!(
        email_verify_message(&ClientError::RateLimited),
        "操作太频繁，请稍后再试"
    );
    assert_eq!(
        email_verify_message(&ClientError::NotConfigured("x".into())),
        "服务器还没配好邮件发送"
    );
}

#[test]
fn email_start_messages() {
    assert_eq!(
        email_start_message(&ClientError::LockedToday("x".into())),
        "今天验证失败次数过多，请明天再试，或改用 Apple 登录"
    );
    assert_eq!(
        email_start_message(&ClientError::RateLimited),
        "操作太频繁，请稍后再试"
    );
    assert_eq!(
        email_start_message(&ClientError::Rejected {
            status: 422,
            message: String::new()
        }),
        "邮箱地址不对，检查后再试"
    );
}

#[test]
fn apple_auth_failed_message_is_unchanged_and_generic_handles_locked() {
    assert_eq!(
        apple_message(&ClientError::AuthFailed("x".into())),
        "Apple 登录没有通过验证，请重试"
    );
    assert_eq!(message(&ClientError::LockedToday("x".into())), LOCKED_TODAY);
}
