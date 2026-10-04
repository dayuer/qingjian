//! 错误文案的映射与「关掉 / 打开同步」时只清学习数据与配置进度。

use std::path::PathBuf;

use qingjian_cloud_client::ClientError;

use qingjian_cloud_proto::{Consents, Feature};

use super::{apply_server_consents, message, reset_after_sync_toggle, should_reset, sync_toggled};
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
fn forbidden_shows_server_text_when_present() {
    assert_eq!(
        message(&ClientError::Forbidden("账号已停用".into())),
        "账号已停用"
    );
    assert_eq!(
        message(&ClientError::Forbidden(String::new())),
        "服务器不允许这个操作"
    );
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
