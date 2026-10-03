//! JSON 形状是协议的一部分，客户端可能不是 Rust 写的，改字段名要在这里看得见。

use qingjian_cloud_proto::{Event, EventKind};

#[test]
fn clip_added_is_flat_with_type_tag() {
    let event = Event {
        seq: 7,
        device: "mac".to_owned(),
        at: 1_700_000_000_000,
        kind: EventKind::ClipAdded {
            client_id: "c1".to_owned(),
            text: "你好".to_owned(),
        },
    };
    let json = serde_json::to_value(&event).unwrap();
    assert_eq!(
        json,
        serde_json::json!({
            "seq": 7, "device": "mac", "at": 1_700_000_000_000_i64,
            "type": "clip_added", "client_id": "c1", "text": "你好"
        })
    );
    assert_eq!(serde_json::from_value::<Event>(json).unwrap(), event);
}

#[test]
fn clip_deleted_round_trips() {
    let json = r#"{"seq":8,"device":"iphone","at":1,"type":"clip_deleted","target":7}"#;
    let event: Event = serde_json::from_str(json).unwrap();
    assert_eq!(event.kind, EventKind::ClipDeleted { target: 7 });
}
