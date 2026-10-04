//! 账号相关的 JSON 形状：服务端（synon-ime）与各客户端按这里的字段名对接。

use qingjian_cloud_proto::{
    Account, AppleClient, AppleSignIn, CROSS_BORDER_CONSENT_VERSION, Consents, Device, EmailStart,
    EmailVerify, Feature, HandoffExchange, HandoffGrant, IdentityInfo, LOGIN_CALLBACK_SCHEME,
    PATH_ACCOUNT, PATH_AUTH_APPLE, PATH_AUTH_EMAIL_START, PATH_AUTH_EMAIL_VERIFY,
    PATH_AUTH_HANDOFF, PATH_CONSENTS, PATH_LOGIN, PATH_SESSIONS, Platform, PutConsent,
    SessionGrant, SessionInfo, TOKEN_PREFIX,
};
use serde_json::json;

fn iphone() -> Device {
    Device {
        name: "iPhone".to_owned(),
        platform: Platform::Ios,
    }
}

#[test]
fn feature_is_snake_case_and_parses_back() {
    assert_eq!(
        serde_json::to_value(Feature::InputLog).unwrap(),
        json!("input_log")
    );
    assert_eq!(
        serde_json::to_value(Feature::Clipboard).unwrap(),
        json!("clipboard")
    );
    for feature in Feature::ALL {
        assert_eq!(Feature::parse(feature.as_str()), Some(feature));
        assert_eq!(
            serde_json::to_value(feature).unwrap(),
            json!(feature.as_str())
        );
    }
    assert_eq!(Feature::parse("inputlog"), None);
}

#[test]
fn platform_and_client_are_lowercase() {
    assert_eq!(
        serde_json::to_value(Platform::Macos).unwrap(),
        json!("macos")
    );
    assert_eq!(serde_json::to_value(Platform::Web).unwrap(), json!("web"));
    assert_eq!(
        serde_json::to_value(AppleClient::Ios).unwrap(),
        json!("ios")
    );
    assert_eq!(
        serde_json::from_value::<Platform>(json!("ios")).unwrap(),
        Platform::Ios
    );
}

#[test]
fn apple_sign_in_without_challenge_omits_it() {
    let request = AppleSignIn {
        identity_token: "jwt".to_owned(),
        authorization_code: "code".to_owned(),
        nonce: "raw".to_owned(),
        client: AppleClient::Ios,
        device: iphone(),
        challenge: None,
        cross_border_consent: "v1".to_owned(),
    };
    let value = serde_json::to_value(&request).unwrap();
    assert_eq!(
        value,
        json!({
            "identity_token": "jwt", "authorization_code": "code", "nonce": "raw",
            "client": "ios", "device": { "name": "iPhone", "platform": "ios" },
            "cross_border_consent": "v1"
        })
    );
    assert_eq!(
        serde_json::from_value::<AppleSignIn>(value).unwrap(),
        request
    );
}

#[test]
fn email_verify_with_challenge_round_trips() {
    let request = EmailVerify {
        email: "a@b.c".to_owned(),
        code: "123456".to_owned(),
        device: Device {
            name: "web".to_owned(),
            platform: Platform::Web,
        },
        challenge: Some("S256".to_owned()),
        cross_border_consent: "v1".to_owned(),
    };
    let value = serde_json::to_value(&request).unwrap();
    assert_eq!(value["challenge"], json!("S256"));
    assert_eq!(
        serde_json::from_value::<EmailVerify>(value).unwrap(),
        request
    );
    let without: EmailVerify = serde_json::from_value(json!({
        "email": "a@b.c", "code": "1", "device": { "name": "n", "platform": "macos" }
    }))
    .unwrap();
    assert_eq!(without.challenge, None);
}

#[test]
fn grants_and_handoff_round_trip() {
    let grant = SessionGrant {
        token: format!("{TOKEN_PREFIX}abc"),
        user_id: 42,
        session_id: 7,
        new_user: true,
    };
    let value = serde_json::to_value(&grant).unwrap();
    assert_eq!(
        value,
        json!({ "token": "sjt_abc", "user_id": 42, "session_id": 7, "new_user": true })
    );
    assert_eq!(
        serde_json::from_value::<SessionGrant>(value).unwrap(),
        grant
    );
    assert_eq!(
        serde_json::to_value(HandoffGrant {
            handoff: "h".to_owned()
        })
        .unwrap(),
        json!({ "handoff": "h" })
    );
    let exchange = HandoffExchange {
        handoff: "h".to_owned(),
        verifier: "v".to_owned(),
        device: iphone(),
    };
    let value = serde_json::to_value(&exchange).unwrap();
    assert_eq!(
        serde_json::from_value::<HandoffExchange>(value).unwrap(),
        exchange
    );
}

#[test]
fn consents_default_off_and_follow_feature() {
    let mut consents = Consents::default();
    for feature in Feature::ALL {
        assert!(!consents.get(feature));
    }
    for feature in Feature::ALL {
        let mut only = Consents::default();
        only.set(feature, true);
        let value = serde_json::to_value(only).unwrap();
        for other in Feature::ALL {
            assert_eq!(value[other.as_str()], json!(other == feature));
            assert_eq!(only.get(other), other == feature);
        }
    }
    consents.set(Feature::InputLog, true);
    assert!(consents.input_log && consents.get(Feature::InputLog));
    assert_eq!(
        serde_json::to_value(consents).unwrap(),
        json!({ "clipboard": false, "sync": false, "input_log": true, "llm": false, "memory": false })
    );
    assert_eq!(
        serde_json::to_value(PutConsent { enabled: true }).unwrap(),
        json!({ "enabled": true })
    );
}

#[test]
fn account_round_trips_with_null_fields() {
    let json = json!({
        "identities": [{ "provider": "apple", "label": null }, { "provider": "email", "label": "a@b.c" }],
        "sessions": [{ "id": 7, "name": "iPhone", "platform": "ios", "created_at": 1, "last_seen": null, "current": true }],
        "consents": { "clipboard": true, "sync": false, "input_log": false, "llm": true, "memory": false }
    });
    let account: Account = serde_json::from_value(json.clone()).unwrap();
    assert_eq!(
        account.identities[0],
        IdentityInfo {
            provider: "apple".to_owned(),
            label: None
        }
    );
    assert_eq!(
        account.sessions[0],
        SessionInfo {
            id: 7,
            name: "iPhone".to_owned(),
            platform: Platform::Ios,
            created_at: 1,
            last_seen: None,
            current: true,
            joined_via: None
        }
    );
    assert!(account.consents.clipboard && account.consents.llm);
    assert_eq!(serde_json::to_value(&account).unwrap(), json);
}

#[test]
fn token_prefix_is_sujian() {
    assert_eq!(TOKEN_PREFIX, "sjt_");
}

#[test]
fn constants_match_the_contract() {
    assert_eq!(PATH_AUTH_APPLE, "/v1/auth/apple");
    assert_eq!(PATH_AUTH_EMAIL_START, "/v1/auth/email/start");
    assert_eq!(PATH_AUTH_EMAIL_VERIFY, "/v1/auth/email/verify");
    assert_eq!(PATH_AUTH_HANDOFF, "/v1/auth/handoff");
    assert_eq!(PATH_ACCOUNT, "/v1/account");
    assert_eq!(PATH_SESSIONS, "/v1/sessions");
    assert_eq!(PATH_CONSENTS, "/v1/consents");
    assert_eq!(PATH_LOGIN, "/login");
    assert_eq!(LOGIN_CALLBACK_SCHEME, "sujian");
}

#[test]
fn unknown_fields_from_a_newer_server_are_ignored() {
    let account: Account = serde_json::from_value(json!({
        "extra": 1,
        "identities": [],
        "sessions": [{
            "id": 1, "name": "n", "platform": "macos", "created_at": 1,
            "last_seen": null, "current": false, "os_version": "x"
        }],
        "consents": { "clipboard": true, "future": true }
    }))
    .unwrap();
    assert!(account.consents.clipboard);
    assert!(!account.consents.sync && !account.consents.input_log && !account.consents.llm);
    let partial: Consents = serde_json::from_value(json!({ "clipboard": true })).unwrap();
    assert!(partial.clipboard && !partial.sync && !partial.input_log && !partial.llm);
}

#[test]
fn cross_border_consent_is_serialized_and_defaults_to_empty() {
    assert_eq!(CROSS_BORDER_CONSENT_VERSION, "2026-10-04");
    let start = EmailStart {
        email: "a@b.c".to_owned(),
        cross_border_consent: CROSS_BORDER_CONSENT_VERSION.to_owned(),
    };
    assert_eq!(
        serde_json::to_value(&start).unwrap(),
        json!({ "email": "a@b.c", "cross_border_consent": "2026-10-04" })
    );
    let start: EmailStart = serde_json::from_value(json!({ "email": "a@b.c" })).unwrap();
    assert_eq!(start.cross_border_consent, "");
    let verify: EmailVerify = serde_json::from_value(json!({
        "email": "a@b.c", "code": "1", "device": { "name": "n", "platform": "ios" }
    }))
    .unwrap();
    assert_eq!(verify.cross_border_consent, "");
    let apple: AppleSignIn = serde_json::from_value(json!({
        "identity_token": "j", "authorization_code": "c", "nonce": "n", "client": "ios",
        "device": { "name": "n", "platform": "ios" }
    }))
    .unwrap();
    assert_eq!(apple.cross_border_consent, "");
    let value = serde_json::to_value(&verify).unwrap();
    assert_eq!(value["cross_border_consent"], json!(""));
}
