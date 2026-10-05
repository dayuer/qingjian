//! 建空间与匹配码加设备的六个客户端方法：路径、请求头（哪些不要令牌）、请求体与错误映射。

mod support;

use qingjian_cloud_client::{Client, ClientError};
use qingjian_cloud_proto::{Device, PairDecision, PairJoin, PairPoll, Platform, SpaceCreate};
use support::{fake_server, fake_server_with_body, has_authorization, request_line};

const CONSENT: &str = "2026-10-04";

fn device() -> Device {
    Device {
        name: "小美的 iPhone".to_owned(),
        platform: Platform::Ios,
    }
}

fn join() -> PairJoin {
    PairJoin {
        code: "K7P2-9QXM".to_owned(),
        device: device(),
    }
}

fn body_of(head: &str) -> serde_json::Value {
    serde_json::from_str(head.split_once("\r\n\r\n").map_or("", |(_, body)| body)).unwrap()
}

#[test]
fn create_space_posts_without_authorization_and_reads_the_grant() {
    let (url, rx) = fake_server_with_body(
        "200 OK",
        r#"{"token":"sjt_a","user_id":7,"session_id":9,"new_user":true}"#,
    );
    let grant = Client::anonymous(&url)
        .create_space(&SpaceCreate {
            device: device(),
            cross_border_consent: CONSENT.to_owned(),
        })
        .unwrap();
    assert_eq!(grant.token, "sjt_a");
    assert!(grant.new_user);

    let head = rx.recv().unwrap();
    assert_eq!(request_line(&head), "POST /v1/space HTTP/1.1");
    assert!(!has_authorization(&head), "{head}");
    let body = body_of(&head);
    assert_eq!(body["device"]["name"], "小美的 iPhone");
    assert_eq!(body["device"]["platform"], "ios");
    assert_eq!(body["cross_border_consent"], CONSENT);
}

#[test]
fn create_space_without_consent_is_consent_required() {
    let (url, _rx) = fake_server_with_body(
        "400 Bad Request",
        r#"{"error":"cross-border consent required","code":"consent_required"}"#,
    );
    let error = Client::anonymous(&url)
        .create_space(&SpaceCreate {
            device: device(),
            cross_border_consent: String::new(),
        })
        .unwrap_err();
    assert!(
        matches!(error, ClientError::ConsentRequired(_)),
        "{error:?}"
    );
}

#[test]
fn pair_code_posts_with_the_token() {
    let (url, rx) = fake_server_with_body(
        "200 OK",
        r#"{"code":"K7P2-9QXM","expires_at":1791043200000}"#,
    );
    let code = Client::new(&url, "tok").pair_code().unwrap();
    assert_eq!(code.code, "K7P2-9QXM");
    assert_eq!(code.expires_at, 1791043200000);

    let head = rx.recv().unwrap();
    assert_eq!(request_line(&head), "POST /v1/pair/code HTTP/1.1");
    assert!(head.contains("Bearer tok"), "{head}");
}

#[test]
fn pair_join_posts_the_code_without_authorization() {
    let (url, rx) = fake_server_with_body(
        "200 OK",
        r#"{"request_id":"r1","secret":"s1","expires_at":1791043200000}"#,
    );
    let grant = Client::anonymous(&url).pair_join(&join()).unwrap();
    assert_eq!(grant.request_id, "r1");
    assert_eq!(grant.secret, "s1");

    let head = rx.recv().unwrap();
    assert_eq!(request_line(&head), "POST /v1/pair/join HTTP/1.1");
    assert!(!has_authorization(&head), "{head}");
    assert_eq!(body_of(&head)["code"], "K7P2-9QXM");
}

#[test]
fn a_bad_pair_code_is_its_own_error() {
    let (url, _rx) = fake_server_with_body(
        "404 Not Found",
        r#"{"error":"pair code not found or expired","code":"bad_code"}"#,
    );
    let error = Client::anonymous(&url).pair_join(&join()).unwrap_err();
    assert!(
        matches!(&error, ClientError::BadCode(message) if message == "pair code not found or expired"),
        "{error:?}"
    );
}

#[test]
fn a_full_space_is_its_own_error() {
    let (url, _rx) = fake_server_with_body(
        "409 Conflict",
        r#"{"error":"space already has 5 devices","code":"device_limit"}"#,
    );
    let error = Client::anonymous(&url).pair_join(&join()).unwrap_err();
    assert!(
        matches!(&error, ClientError::DeviceLimit(message) if message == "space already has 5 devices"),
        "{error:?}"
    );
}

#[test]
fn pair_poll_sends_the_secret_header_and_no_authorization() {
    let (url, rx) = fake_server_with_body("200 OK", r#"{"state":"pending"}"#);
    assert_eq!(
        Client::anonymous(&url).pair_poll("r1", "s1").unwrap(),
        PairPoll::Pending
    );

    let head = rx.recv().unwrap();
    assert_eq!(request_line(&head), "GET /v1/pair/join/r1 HTTP/1.1");
    assert!(!has_authorization(&head), "{head}");
    assert!(
        head.to_ascii_lowercase().contains("x-pair-secret: s1"),
        "{head}"
    );
}

#[test]
fn pair_poll_reads_denied_and_approved() {
    let (url, _rx) = fake_server_with_body("200 OK", r#"{"state":"denied"}"#);
    assert_eq!(
        Client::anonymous(&url).pair_poll("r1", "s1").unwrap(),
        PairPoll::Denied
    );

    let (url, _rx) = fake_server_with_body(
        "200 OK",
        r#"{"state":"approved","token":"sjt_b","user_id":7,"session_id":9,"new_user":false}"#,
    );
    let PairPoll::Approved(grant) = Client::anonymous(&url).pair_poll("r1", "s1").unwrap() else {
        panic!("应该是 approved");
    };
    assert_eq!(grant.token, "sjt_b");
    assert!(!grant.new_user);
}

#[test]
fn a_consumed_pair_request_is_a_plane_404() {
    // 取过令牌之后再轮询是 404，服务端不带 `bad_code`：调用方按状态码判「这次申请没了」
    let (url, _rx) = fake_server_with_body("404 Not Found", r#"{"error":"no such request"}"#);
    let error = Client::anonymous(&url).pair_poll("r1", "s1").unwrap_err();
    assert!(
        matches!(error, ClientError::Rejected { status: 404, .. }),
        "{error:?}"
    );
}

#[test]
fn pair_requests_lists_pending_with_the_token() {
    let (url, rx) = fake_server_with_body(
        "200 OK",
        r#"[{"id":"r1","name":"小美的 iPhone","platform":"ios","at":1791043200000}]"#,
    );
    let requests = Client::new(&url, "tok").pair_requests().unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].id, "r1");
    assert_eq!(requests[0].name, "小美的 iPhone");
    assert_eq!(requests[0].platform, Platform::Ios);

    let head = rx.recv().unwrap();
    assert_eq!(request_line(&head), "GET /v1/pair/requests HTTP/1.1");
    assert!(head.contains("Bearer tok"), "{head}");
}

#[test]
fn pair_decide_posts_the_decision_with_the_token() {
    let (url, rx) = fake_server("204 No Content");
    Client::new(&url, "tok")
        .pair_decide("r1", PairDecision { allow: true })
        .unwrap();

    let head = rx.recv().unwrap();
    assert_eq!(request_line(&head), "POST /v1/pair/requests/r1 HTTP/1.1");
    assert!(head.contains("Bearer tok"), "{head}");
    assert_eq!(body_of(&head)["allow"], true);
}
