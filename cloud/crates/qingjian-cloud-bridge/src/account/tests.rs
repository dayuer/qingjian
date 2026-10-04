//! 错误文案的映射与「关掉 / 打开同步」时只清学习数据与配置进度。

use std::path::PathBuf;

use qingjian_cloud_client::ClientError;

use super::{message, reset_sync_progress};

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
    reset_sync_progress(&dir.join("cloud.toml"));
    for name in gone {
        assert!(!state.join(name).exists(), "{name} 应该被删");
    }
    for name in kept {
        assert!(state.join(name).exists(), "{name} 应该还在");
    }
    std::fs::remove_dir_all(&dir).ok();
}
