//! 端到端：两台设备经 ClipboardSync 互传剪贴板，覆盖实时推送、离线补发与断线补拉。

mod common;

use std::sync::Arc;
use std::time::{Duration, Instant};

use qingjian_cloud_client::{ClipboardSync, Incoming, Status, SyncConfig};
use qingjian_cloud_proto::EventKind;

use common::{TestServer, free_addr, wait_for};

const TIMEOUT: Duration = Duration::from_secs(10);

fn start(url: &str, token: &str) -> (ClipboardSync, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let sync = ClipboardSync::start(SyncConfig {
        server: url.to_owned(),
        token: token.to_owned(),
        state_dir: dir.path().to_owned(),
    })
    .unwrap();
    (sync, dir)
}

/// 等到收到一条别的设备复制的文本。
fn next_text(sync: &ClipboardSync) -> Option<String> {
    wait_for(TIMEOUT, || {
        while let Some(Incoming { event, mine }) = sync.try_recv() {
            if let (false, EventKind::ClipAdded { text, .. }) = (mine, event.kind) {
                return Some(text);
            }
        }
        None
    })
}

fn wait_online(sync: &ClipboardSync) {
    wait_for(TIMEOUT, || (sync.status() == Status::Online).then_some(()))
        .expect("never came online");
}

#[test]
fn copy_on_one_device_arrives_on_the_other_quickly() {
    let server = TestServer::start();
    let (mac, _a) = start(&server.url, &server.store.add_device("mac").unwrap());
    let (phone, _b) = start(&server.url, &server.store.add_device("iphone").unwrap());
    wait_online(&mac);
    wait_online(&phone);
    // 两边都挂上 SSE 之后才计时
    std::thread::sleep(Duration::from_millis(200));
    let started = Instant::now();
    mac.copy("从 Mac 复制".to_owned());
    assert_eq!(next_text(&phone).as_deref(), Some("从 Mac 复制"));
    let elapsed = started.elapsed();
    assert!(elapsed < Duration::from_secs(1), "took {elapsed:?}");
    // 自己发的也会回来，但标着 mine
    let own = wait_for(TIMEOUT, || mac.try_recv()).unwrap();
    assert!(own.mine);
}

#[test]
fn offline_copies_are_queued_and_sent_when_server_returns() {
    let addr = free_addr();
    let store = Arc::new(qingjian_cloud_server::Store::in_memory().unwrap());
    let mac_token = store.add_device("mac").unwrap();
    let phone_token = store.add_device("iphone").unwrap();
    let url = format!("http://{addr}");

    // 服务器还没起：复制的东西进离线队列，状态是离线
    let (mac, _a) = start(&url, &mac_token);
    mac.copy("离线时复制的".to_owned());
    wait_for(TIMEOUT, || {
        matches!(mac.status(), Status::Offline(_)).then_some(())
    })
    .expect("should report offline");
    assert_eq!(mac.pending(), 1);

    let server = TestServer::start_on(addr, store);
    let (phone, _b) = start(&server.url, &phone_token);
    assert_eq!(next_text(&phone).as_deref(), Some("离线时复制的"));
    wait_for(TIMEOUT, || (mac.pending() == 0).then_some(())).expect("outbox drained");
}

#[test]
fn listener_catches_up_after_server_restart() {
    let server = TestServer::start();
    let addr = server.addr();
    let mac_token = server.store.add_device("mac").unwrap();
    let phone_token = server.store.add_device("iphone").unwrap();
    let (phone, _b) = start(&server.url, &phone_token);
    wait_online(&phone);

    // 服务器重启期间别的设备发了一条：手机重连后按 seq 补到
    let store = server.stop();
    let mac = qingjian_cloud_client::Client::new(&format!("http://{addr}"), &mac_token);
    let server = TestServer::start_on(addr, store);
    mac.push_clip(&qingjian_cloud_proto::PushClip {
        client_id: "x".to_owned(),
        text: "重启期间".to_owned(),
    })
    .unwrap();
    assert_eq!(next_text(&phone).as_deref(), Some("重启期间"));
    drop(server);
}

#[test]
fn progress_survives_restart_without_redelivery() {
    let server = TestServer::start();
    let mac =
        qingjian_cloud_client::Client::new(&server.url, &server.store.add_device("mac").unwrap());
    let phone_token = server.store.add_device("iphone").unwrap();
    mac.push_clip(&qingjian_cloud_proto::PushClip {
        client_id: "1".to_owned(),
        text: "第一条".to_owned(),
    })
    .unwrap();
    let dir = tempfile::tempdir().unwrap();
    let config = SyncConfig {
        server: server.url.clone(),
        token: phone_token,
        state_dir: dir.path().to_owned(),
    };
    {
        let phone = ClipboardSync::start(config.clone()).unwrap();
        assert_eq!(next_text(&phone).as_deref(), Some("第一条"));
    }
    mac.push_clip(&qingjian_cloud_proto::PushClip {
        client_id: "2".to_owned(),
        text: "第二条".to_owned(),
    })
    .unwrap();
    let phone = ClipboardSync::start(config).unwrap();
    assert_eq!(next_text(&phone).as_deref(), Some("第二条"));
}
