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
    Client::anonymous(&url).email_start("a@b.c", "v1").unwrap();
    let head = rx.recv().unwrap();
    assert_eq!(request_line(&head), "POST /v1/auth/email/start HTTP/1.1");
    assert!(!has_authorization(&head), "{head}");
    let body = head.split_once("\r\n\r\n").map_or("", |(_, body)| body);
    let body: serde_json::Value = serde_json::from_str(body).unwrap();
    assert_eq!(body["cross_border_consent"], "v1");
    assert_eq!(body["email"], "a@b.c");
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

mod reply_mapping {
    use qingjian_cloud_client::{Client, ClientError};
    use qingjian_cloud_proto::{
        AppleClient, AppleSignIn, Device, EmailVerify, HandoffExchange, Platform,
    };

    use crate::support::fake_server_with_body;

    fn device() -> Device {
        Device {
            name: "t".to_owned(),
            platform: Platform::Macos,
        }
    }

    fn apple() -> AppleSignIn {
        AppleSignIn {
            identity_token: String::new(),
            authorization_code: String::new(),
            nonce: String::new(),
            client: AppleClient::Ios,
            device: device(),
            challenge: None,
            cross_border_consent: String::new(),
        }
    }

    fn handoff() -> HandoffExchange {
        HandoffExchange {
            handoff: String::new(),
            verifier: String::new(),
            device: device(),
        }
    }

    fn verify(client: &Client) -> Result<(), ClientError> {
        client
            .email_verify(&EmailVerify {
                email: "a@b.c".to_owned(),
                code: "000000".to_owned(),
                device: device(),
                challenge: None,
                cross_border_consent: "v1".to_owned(),
            })
            .map(|_| ())
    }

    #[test]
    fn login_401_is_auth_failed_with_message() {
        let (url, _rx) = fake_server_with_body("401 Unauthorized", r#"{"error":"wrong code"}"#);
        let error = verify(&Client::anonymous(&url)).unwrap_err();
        assert!(matches!(error, ClientError::AuthFailed(m) if m == "wrong code"));
    }

    #[test]
    fn login_503_is_not_configured_but_other_methods_503_is_unreachable() {
        let (url, _rx) = fake_server_with_body("503 Service Unavailable", r#"{"error":"smtp"}"#);
        let client = Client::anonymous(&url);
        assert!(matches!(
            client.email_start("a@b.c", "v1").unwrap_err(),
            ClientError::NotConfigured(m) if m == "smtp"
        ));
        assert!(matches!(
            client.sign_in_apple(&apple()).unwrap_err(),
            ClientError::NotConfigured(_)
        ));
        assert!(matches!(
            client.exchange_handoff(&handoff()).unwrap_err(),
            ClientError::NotConfigured(_)
        ));
        // 同步线程用的方法保持原映射
        assert!(matches!(
            client.whoami().unwrap_err(),
            ClientError::Unreachable(_)
        ));
    }

    #[test]
    fn login_502_is_retryable_unreachable() {
        let (url, _rx) = fake_server_with_body("502 Bad Gateway", "");
        let error = verify(&Client::anonymous(&url)).unwrap_err();
        assert!(matches!(error, ClientError::Unreachable(_)));
        assert!(error.is_retryable());
    }

    const LOCKED: &str = r#"{"error":"too many failed attempts today","code":"locked_today"}"#;

    fn login_429_results(body: &'static str) -> [ClientError; 2] {
        let (url, _rx) = fake_server_with_body("429 Too Many Requests", body);
        let start = Client::anonymous(&url)
            .email_start("a@b.c", "v1")
            .unwrap_err();
        let (url, _rx) = fake_server_with_body("429 Too Many Requests", body);
        let verified = verify(&Client::anonymous(&url)).unwrap_err();
        [start, verified]
    }

    #[test]
    fn login_429_with_locked_code_is_locked_today() {
        for error in login_429_results(LOCKED) {
            assert!(matches!(
                error,
                ClientError::LockedToday(m) if m == "too many failed attempts today"
            ));
        }
    }

    #[test]
    fn login_429_without_or_with_other_code_is_rate_limited() {
        for body in [
            r#"{"error":"too many requests, try again later"}"#,
            r#"{"error":"x","code":"something_else"}"#,
            "<html>busy</html>",
        ] {
            for error in login_429_results(body) {
                assert!(matches!(error, ClientError::RateLimited), "{body}");
            }
        }
    }

    #[test]
    fn login_429_is_rate_limited() {
        let (url, _rx) = fake_server_with_body("429 Too Many Requests", r#"{"error":"slow"}"#);
        assert!(matches!(
            Client::anonymous(&url)
                .email_start("a@b.c", "v1")
                .unwrap_err(),
            ClientError::RateLimited
        ));
    }

    #[test]
    fn login_other_4xx_is_rejected_with_message() {
        let (url, _rx) = fake_server_with_body("400 Bad Request", r#"{"error":"bad email"}"#);
        assert!(matches!(
            Client::anonymous(&url).email_start("x", "v1").unwrap_err(),
            ClientError::Rejected { status: 400, message } if message == "bad email"
        ));
    }

    const CONSENT_REQUIRED: &str =
        r#"{"error":"cross-border consent required","code":"consent_required"}"#;

    #[test]
    fn login_400_consent_required_is_its_own_error() {
        let (url, _rx) = fake_server_with_body("400 Bad Request", CONSENT_REQUIRED);
        assert!(matches!(
            Client::anonymous(&url).email_start("a@b.c", "").unwrap_err(),
            ClientError::ConsentRequired(m) if m == "cross-border consent required"
        ));
        let (url, _rx) = fake_server_with_body("400 Bad Request", CONSENT_REQUIRED);
        assert!(matches!(
            Client::anonymous(&url).sign_in_apple(&apple()).unwrap_err(),
            ClientError::ConsentRequired(_)
        ));
        let (url, _rx) = fake_server_with_body("400 Bad Request", CONSENT_REQUIRED);
        assert!(matches!(
            verify(&Client::anonymous(&url)).unwrap_err(),
            ClientError::ConsentRequired(_)
        ));
    }

    #[test]
    fn login_400_without_or_with_other_code_is_rejected() {
        for body in [
            r#"{"error":"bad email"}"#,
            r#"{"error":"x","code":"other_code"}"#,
            "<html>bad</html>",
        ] {
            let (url, _rx) = fake_server_with_body("400 Bad Request", body);
            assert!(
                matches!(
                    Client::anonymous(&url)
                        .email_start("a@b.c", "v1")
                        .unwrap_err(),
                    ClientError::Rejected { status: 400, .. }
                ),
                "{body}"
            );
        }
    }

    #[test]
    fn account_400_consent_required_stays_rejected() {
        let (url, _rx) = fake_server_with_body("400 Bad Request", CONSENT_REQUIRED);
        assert!(matches!(
            Client::new(&url, "t")
                .put_consent(qingjian_cloud_proto::Feature::Sync, true)
                .unwrap_err(),
            ClientError::Rejected { status: 400, .. }
        ));
    }

    #[test]
    fn account_403_carries_message() {
        let (url, _rx) = fake_server_with_body(
            "403 Forbidden",
            r#"{"error":"feature not enabled: clipboard"}"#,
        );
        assert!(matches!(
            Client::new(&url, "t").account().unwrap_err(),
            ClientError::Forbidden(m) if m == "feature not enabled: clipboard"
        ));
    }

    #[test]
    fn account_401_is_unauthorized() {
        let (url, _rx) = fake_server_with_body("401 Unauthorized", r#"{"error":"x"}"#);
        assert!(matches!(
            Client::new(&url, "t").sign_out().unwrap_err(),
            ClientError::Unauthorized
        ));
    }

    #[test]
    fn account_404_is_rejected_with_message() {
        let (url, _rx) = fake_server_with_body("404 Not Found", r#"{"error":"no such session"}"#);
        assert!(matches!(
            Client::new(&url, "t").revoke_session(9).unwrap_err(),
            ClientError::Rejected { status: 404, message } if message == "no such session"
        ));
    }

    #[test]
    fn account_5xx_is_unreachable() {
        let (url, _rx) = fake_server_with_body("503 Service Unavailable", "");
        assert!(matches!(
            Client::new(&url, "t").delete_account().unwrap_err(),
            ClientError::Unreachable(_)
        ));
    }

    #[test]
    fn non_json_body_gives_empty_message() {
        let (url, _rx) = fake_server_with_body("403 Forbidden", "<html>nope</html>");
        assert!(matches!(
            Client::new(&url, "t").account().unwrap_err(),
            ClientError::Forbidden(m) if m.is_empty()
        ));
    }
}
