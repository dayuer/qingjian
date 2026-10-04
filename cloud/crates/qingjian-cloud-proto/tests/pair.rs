//! 子项目 1b（不要账号）的 JSON 形状：建空间、匹配码加设备、临时事件；服务端（synon-ime）按这里的字段名对接。

use qingjian_cloud_proto::{
    CODE_BAD_CODE, CODE_DEVICE_LIMIT, Device, Event, EventKind, HEADER_PAIR_SECRET, PAIR_CODE_LEN,
    PATH_PAIR_CODE, PATH_PAIR_JOIN, PATH_PAIR_REQUESTS, PATH_SPACE, PairCode, PairCodeError,
    PairDecision, PairJoin, PairJoinGrant, PairPoll, PairRequestInfo, Platform, SessionGrant,
    SessionInfo, SpaceCreate, format_pair_code, normalize_pair_code,
};
use serde_json::json;

fn mac() -> Device {
    Device {
        name: "MacBook".to_owned(),
        platform: Platform::Macos,
    }
}

#[test]
fn constants_match_the_server_contract() {
    assert_eq!(PATH_SPACE, "/v1/space");
    assert_eq!(PATH_PAIR_CODE, "/v1/pair/code");
    assert_eq!(PATH_PAIR_JOIN, "/v1/pair/join");
    assert_eq!(PATH_PAIR_REQUESTS, "/v1/pair/requests");
    assert_eq!(HEADER_PAIR_SECRET, "X-Pair-Secret");
    assert_eq!(CODE_BAD_CODE, "bad_code");
    assert_eq!(CODE_DEVICE_LIMIT, "device_limit");
    assert_eq!(PAIR_CODE_LEN, 8);
}

#[test]
fn space_create_round_trips_and_consent_defaults_to_empty() {
    let request = SpaceCreate {
        device: mac(),
        cross_border_consent: "2026-10-04".to_owned(),
    };
    let value = serde_json::to_value(&request).unwrap();
    assert_eq!(
        value,
        json!({
            "device": { "name": "MacBook", "platform": "macos" },
            "cross_border_consent": "2026-10-04"
        })
    );
    assert_eq!(
        serde_json::from_value::<SpaceCreate>(value).unwrap(),
        request
    );
    let bare: SpaceCreate =
        serde_json::from_value(json!({ "device": { "name": "n", "platform": "ios" } })).unwrap();
    assert_eq!(bare.cross_border_consent, "");
}

#[test]
fn pair_request_and_response_bodies_round_trip() {
    let code = PairCode {
        code: "K7P2-9QXM".to_owned(),
        expires_at: 1_700_000_600_000,
    };
    let value = serde_json::to_value(&code).unwrap();
    assert_eq!(
        value,
        json!({ "code": "K7P2-9QXM", "expires_at": 1_700_000_600_000_i64 })
    );
    assert_eq!(serde_json::from_value::<PairCode>(value).unwrap(), code);

    let join = PairJoin {
        code: "k7p2 9qxm".to_owned(),
        device: mac(),
    };
    let value = serde_json::to_value(&join).unwrap();
    assert_eq!(
        value,
        json!({ "code": "k7p2 9qxm", "device": { "name": "MacBook", "platform": "macos" } })
    );
    assert_eq!(serde_json::from_value::<PairJoin>(value).unwrap(), join);

    let grant = PairJoinGrant {
        request_id: "r1".to_owned(),
        secret: "ab".repeat(32),
        expires_at: 5,
    };
    let value = serde_json::to_value(&grant).unwrap();
    assert_eq!(
        value,
        json!({ "request_id": "r1", "secret": "ab".repeat(32), "expires_at": 5 })
    );
    assert_eq!(
        serde_json::from_value::<PairJoinGrant>(value).unwrap(),
        grant
    );

    let info = PairRequestInfo {
        id: "r1".to_owned(),
        name: "MacBook".to_owned(),
        platform: Platform::Macos,
        at: 9,
    };
    let value = serde_json::to_value(&info).unwrap();
    assert_eq!(
        value,
        json!({ "id": "r1", "name": "MacBook", "platform": "macos", "at": 9 })
    );
    assert_eq!(
        serde_json::from_value::<PairRequestInfo>(value).unwrap(),
        info
    );

    let decision = PairDecision { allow: false };
    let value = serde_json::to_value(decision).unwrap();
    assert_eq!(value, json!({ "allow": false }));
    assert_eq!(
        serde_json::from_value::<PairDecision>(value).unwrap(),
        decision
    );
}

#[test]
fn pair_poll_pending_and_denied_are_bare_state_tags() {
    assert_eq!(
        serde_json::to_value(PairPoll::Pending).unwrap(),
        json!({ "state": "pending" })
    );
    assert_eq!(
        serde_json::to_value(PairPoll::Denied).unwrap(),
        json!({ "state": "denied" })
    );
    assert_eq!(
        serde_json::from_value::<PairPoll>(json!({ "state": "pending" })).unwrap(),
        PairPoll::Pending
    );
    assert_eq!(
        serde_json::from_value::<PairPoll>(json!({ "state": "denied" })).unwrap(),
        PairPoll::Denied
    );
}

#[test]
fn pair_poll_approved_flattens_the_session_grant() {
    let poll = PairPoll::Approved(SessionGrant {
        token: "sjt_x".to_owned(),
        user_id: 3,
        session_id: 11,
        new_user: false,
    });
    let value = serde_json::to_value(&poll).unwrap();
    assert_eq!(
        value,
        json!({
            "state": "approved", "token": "sjt_x", "user_id": 3,
            "session_id": 11, "new_user": false
        })
    );
    assert_eq!(serde_json::from_value::<PairPoll>(value).unwrap(), poll);
    assert!(serde_json::from_value::<PairPoll>(json!({ "state": "approved" })).is_err());
    assert!(serde_json::from_value::<PairPoll>(json!({ "state": "expired" })).is_err());
}

#[test]
fn pair_request_event_is_flat_with_type_tag_and_seq_zero() {
    let event = Event {
        seq: 0,
        device: "iPhone".to_owned(),
        at: 1,
        kind: EventKind::PairRequest {
            request_id: "r1".to_owned(),
            name: "MacBook".to_owned(),
            platform: Platform::Macos,
        },
    };
    let value = serde_json::to_value(&event).unwrap();
    assert_eq!(
        value,
        json!({
            "seq": 0, "device": "iPhone", "at": 1,
            "type": "pair_request", "request_id": "r1", "name": "MacBook", "platform": "macos"
        })
    );
    assert_eq!(serde_json::from_value::<Event>(value).unwrap(), event);
}

#[test]
fn device_joined_event_round_trips() {
    let json = r#"{"seq":0,"device":"MacBook","at":2,"type":"device_joined","name":"MacBook","via":"email"}"#;
    let event: Event = serde_json::from_str(json).unwrap();
    assert_eq!(
        event.kind,
        EventKind::DeviceJoined {
            name: "MacBook".to_owned(),
            via: "email".to_owned()
        }
    );
    assert_eq!(
        serde_json::to_value(&event).unwrap(),
        serde_json::from_str::<serde_json::Value>(json).unwrap()
    );
}

#[test]
fn session_info_joined_via_is_optional_on_the_wire() {
    let old = json!({
        "id": 7, "name": "iPhone", "platform": "ios", "created_at": 1,
        "last_seen": null, "current": true
    });
    let first: SessionInfo = serde_json::from_value(old.clone()).unwrap();
    assert_eq!(first.joined_via, None);
    assert_eq!(serde_json::to_value(&first).unwrap(), old);

    let joined = SessionInfo {
        joined_via: Some("pair".to_owned()),
        ..first
    };
    let value = serde_json::to_value(&joined).unwrap();
    assert_eq!(
        value,
        json!({
            "id": 7, "name": "iPhone", "platform": "ios", "created_at": 1,
            "last_seen": null, "current": true, "joined_via": "pair"
        })
    );
    assert_eq!(
        serde_json::from_value::<SessionInfo>(value).unwrap(),
        joined
    );
}

#[test]
fn normalize_pair_code_accepts_what_people_type() {
    let cases = [
        ("K7P2-9QXM", "K7P29QXM"),
        ("k7p2-9qxm", "K7P29QXM"),
        ("K7P29QXM", "K7P29QXM"),
        ("  k7p2 9qxm \n", "K7P29QXM"),
        ("K7P2 - 9QXM", "K7P29QXM"),
        ("k7p2\t9qxm", "K7P29QXM"),
        // 容易看错的字母按 Crockford 规则归一：I / L → 1、O → 0。
        ("IlLi-OoO0", "11110000"),
        ("ABCD-EFGH", "ABCDEFGH"),
        ("JKMN-PQRS", "JKMNPQRS"),
        ("TVWX-YZ00", "TVWXYZ00"),
    ];
    for (input, expected) in cases {
        assert_eq!(
            normalize_pair_code(input).as_deref(),
            Ok(expected),
            "input {input:?}"
        );
    }
}

#[test]
fn normalize_pair_code_rejects_bad_input() {
    let cases = [
        ("", PairCodeError::Length(0)),
        ("K7P2-9QX", PairCodeError::Length(7)),
        ("K7P2-9QXMA", PairCodeError::Length(9)),
        ("K7P2-9QXU", PairCodeError::Char('U')),
        ("k7p2-9qxu", PairCodeError::Char('u')),
        ("K7P2_9QXM", PairCodeError::Char('_')),
        ("K7P2-9QX码", PairCodeError::Char('码')),
        ("K7P2-9QX*", PairCodeError::Char('*')),
    ];
    for (input, expected) in cases {
        assert_eq!(normalize_pair_code(input), Err(expected), "input {input:?}");
    }
}

#[test]
fn format_pair_code_inserts_the_hyphen() {
    assert_eq!(format_pair_code("K7P29QXM"), "K7P2-9QXM");
    assert_eq!(
        format_pair_code(&normalize_pair_code("k7p2 9qxm").unwrap()),
        "K7P2-9QXM"
    );
}
