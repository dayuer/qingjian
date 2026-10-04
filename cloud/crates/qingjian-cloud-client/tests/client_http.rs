//! Client 对各状态码的映射，以及账号接口的方法、路径与请求头。

mod support;

use qingjian_cloud_client::{Client, ClientError};
use qingjian_cloud_proto::Feature;
use support::{fake_server, has_authorization, request_line};

#[test]
fn status_codes_map_to_errors() {
    for (line, check) in [
        (
            "403 Forbidden",
            (|e| matches!(e, ClientError::Forbidden(_))) as fn(&ClientError) -> bool,
        ),
        ("401 Unauthorized", |e| {
            matches!(e, ClientError::Unauthorized)
        }),
        ("429 Too Many Requests", |e| {
            matches!(e, ClientError::RateLimited)
        }),
    ] {
        let (url, _rx) = fake_server(line);
        let error = Client::new(&url, "tok").account().unwrap_err();
        assert!(check(&error), "{line} -> {error:?}");
    }
}

#[test]
fn email_start_accepts_204_and_sends_no_authorization() {
    let (url, rx) = fake_server("204 No Content");
    Client::anonymous(&url).email_start("a@b.c").unwrap();
    let head = rx.recv().unwrap();
    assert_eq!(request_line(&head), "POST /v1/auth/email/start HTTP/1.1");
    assert!(!has_authorization(&head), "{head}");
}

#[test]
fn account_methods_use_expected_method_and_path() {
    let (url, rx) = fake_server("204 No Content");
    let client = Client::new(&url, "tok");

    client.sign_out().unwrap();
    let head = rx.recv().unwrap();
    assert_eq!(request_line(&head), "DELETE /v1/sessions/current HTTP/1.1");
    assert!(head.contains("Bearer tok"), "{head}");

    client.delete_account().unwrap();
    assert_eq!(
        request_line(&rx.recv().unwrap()),
        "DELETE /v1/account HTTP/1.1"
    );

    client.revoke_session(7).unwrap();
    assert_eq!(
        request_line(&rx.recv().unwrap()),
        "DELETE /v1/sessions/7 HTTP/1.1"
    );
}

#[test]
fn put_consent_targets_feature_path() {
    // 204 没有响应体，解析会失败，这里只看发出去的请求
    let (url, rx) = fake_server("204 No Content");
    let _ = Client::new(&url, "tok").put_consent(Feature::Clipboard, true);
    let head = rx.recv().unwrap();
    assert_eq!(request_line(&head), "PUT /v1/consents/clipboard HTTP/1.1");
    assert!(head.contains("Bearer tok"), "{head}");
}
