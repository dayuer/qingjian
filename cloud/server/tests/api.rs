//! 接口层：鉴权、去重、删除、拉取与保留策略。

mod common;

use qingjian_cloud_client::{Client, ClientError};
use qingjian_cloud_proto::{EventKind, PushClip};

use common::TestServer;

fn clip(id: &str, text: &str) -> PushClip {
    PushClip {
        client_id: id.to_owned(),
        text: text.to_owned(),
    }
}

#[test]
fn rejects_missing_and_wrong_tokens() {
    let server = TestServer::start();
    let error = Client::new(&server.url, "qjc_wrong").whoami().unwrap_err();
    assert!(matches!(error, ClientError::Unauthorized), "{error:?}");
    let token = server.store.add_device("mac").unwrap();
    server.store.remove_device("mac").unwrap();
    assert!(matches!(
        Client::new(&server.url, &token).whoami(),
        Err(ClientError::Unauthorized)
    ));
}

#[test]
fn push_is_idempotent_per_client_id() {
    let server = TestServer::start();
    let client = Client::new(&server.url, &server.store.add_device("mac").unwrap());
    let first = client.push_clip(&clip("a", "你好")).unwrap();
    let again = client.push_clip(&clip("a", "你好")).unwrap();
    assert_eq!(first, again);
    assert_eq!(first.device, "mac");
    let page = client.events(0, 100).unwrap();
    assert_eq!(page.events.len(), 1);
    assert_eq!(page.latest, first.seq);
}

#[test]
fn rejects_empty_and_oversized_text() {
    let server = TestServer::start();
    let client = Client::new(&server.url, &server.store.add_device("mac").unwrap());
    assert!(matches!(
        client.push_clip(&clip("a", "")),
        Err(ClientError::Rejected { status: 400, .. })
    ));
    let big = "x".repeat(qingjian_cloud_proto::MAX_CLIP_BYTES + 1);
    assert!(matches!(
        client.push_clip(&clip("b", &big)),
        Err(ClientError::Rejected { status: 413, .. })
    ));
}

#[test]
fn delete_removes_text_and_records_event() {
    let server = TestServer::start();
    let mac = Client::new(&server.url, &server.store.add_device("mac").unwrap());
    let phone = Client::new(&server.url, &server.store.add_device("iphone").unwrap());
    let added = mac.push_clip(&clip("a", "密码别同步")).unwrap();
    let deleted = phone.delete_clip(added.seq).unwrap();
    assert_eq!(deleted.kind, EventKind::ClipDeleted { target: added.seq });
    let page = mac.events(0, 100).unwrap();
    assert_eq!(page.events, [deleted]);
    assert!(matches!(
        phone.delete_clip(added.seq),
        Err(ClientError::Rejected { status: 404, .. })
    ));
}

#[test]
fn paging_and_latest_seq_survive_pruning() {
    let server = TestServer::start();
    let client = Client::new(&server.url, &server.store.add_device("mac").unwrap());
    for i in 0..5 {
        client
            .push_clip(&clip(&i.to_string(), &format!("t{i}")))
            .unwrap();
    }
    let page = client.events(1, 2).unwrap();
    assert_eq!(
        page.events.iter().map(|e| e.seq).collect::<Vec<_>>(),
        [2, 3]
    );
    assert_eq!(page.latest, 5);
    assert_eq!(server.store.prune(2, 0).unwrap(), 3);
    let page = client.events(0, 100).unwrap();
    assert_eq!(
        page.events.iter().map(|e| e.seq).collect::<Vec<_>>(),
        [4, 5]
    );
    // 删掉的事件不让 latest 回退
    assert_eq!(page.latest, 5);
    assert_eq!(server.store.prune(10, i64::MAX).unwrap(), 2);
    assert_eq!(client.events(0, 100).unwrap().latest, 5);
}
