//! 服务器上关了剪贴板（403）：队列清空、Disabled 期间不入队；401 时队列保留。

mod support;

use std::time::{Duration, Instant};

use qingjian_cloud_client::{ClipboardSync, Status, SyncConfig};
use support::fake_server;

fn start(server: &str) -> (ClipboardSync, std::path::PathBuf) {
    let dir = std::env::temp_dir().join(format!("qjc-clip-{}", uuid_like()));
    std::fs::create_dir_all(&dir).unwrap();
    let sync = ClipboardSync::start(SyncConfig {
        server: server.to_owned(),
        token: "tok".to_owned(),
        state_dir: dir.clone(),
    })
    .unwrap();
    (sync, dir)
}

fn uuid_like() -> String {
    format!(
        "{}-{:?}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    )
}

fn wait_for(sync: &ClipboardSync, want: impl Fn(&Status) -> bool) -> bool {
    let deadline = Instant::now() + Duration::from_secs(2);
    while Instant::now() < deadline {
        if want(&sync.status()) {
            return true;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    false
}

#[test]
fn forbidden_clears_queue_and_stops_enqueueing() {
    let (url, _rx) = fake_server("403 Forbidden");
    let (sync, dir) = start(&url);
    sync.copy("x".to_owned());
    assert!(wait_for(&sync, |s| *s == Status::Disabled));
    // 上传线程在状态变 Disabled 之后清队列，给它一点时间
    let deadline = Instant::now() + Duration::from_secs(2);
    while sync.pending() > 0 && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
    }
    assert_eq!(sync.pending(), 0);
    sync.copy("y".to_owned());
    assert_eq!(sync.pending(), 0);
    drop(sync);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn unauthorized_keeps_queue() {
    let (url, _rx) = fake_server("401 Unauthorized");
    let (sync, dir) = start(&url);
    sync.copy("x".to_owned());
    assert!(wait_for(&sync, |s| *s == Status::Unauthorized));
    std::thread::sleep(Duration::from_millis(200));
    assert_eq!(sync.pending(), 1);
    drop(sync);
    let _ = std::fs::remove_dir_all(dir);
}
