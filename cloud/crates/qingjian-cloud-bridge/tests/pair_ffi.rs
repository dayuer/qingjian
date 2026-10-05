//! 建空间与匹配码加设备的 C 接口，对着假服务走一遍：令牌只在 `cloud.toml` 里，
//! 成功返回的 JSON 里没有它；码不对、空间满各有各的 code 与中文。

mod support;

use std::ffi::c_char;
use std::path::PathBuf;
use std::ptr;

use qingjian_cloud_bridge::{CloudConfig, qj_string_free};
use qingjian_cloud_client::test_support::{fake_server_sequence, has_authorization, request_line};
use serde_json::Value;

use support::c_string;

unsafe extern "C" {
    fn qj_space_create(path: *const c_char, device: *const c_char, consented: bool) -> *mut c_char;
    fn qj_pair_code(path: *const c_char) -> *mut c_char;
    fn qj_pair_join(path: *const c_char, code: *const c_char, device: *const c_char)
    -> *mut c_char;
    fn qj_pair_poll(
        path: *const c_char,
        request_id: *const c_char,
        secret: *const c_char,
    ) -> *mut c_char;
    fn qj_pair_requests(path: *const c_char) -> *mut c_char;
    fn qj_pair_decide(path: *const c_char, request_id: *const c_char, allow: bool) -> *mut c_char;
}

/// 每次用不同的目录：这些接口按路径读写 `cloud.toml`，共用会互相干扰。
fn cloud_toml(name: &str, server: &str, token: Option<&str>) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("qj-pair-{name}-{}", std::process::id()));
    std::fs::remove_dir_all(&dir).ok();
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("cloud.toml");
    let token = token.map_or(String::new(), |t| format!("token = \"{t}\"\n"));
    std::fs::write(&path, format!("server = \"{server}\"\n{token}")).unwrap();
    path
}

/// 取走一个 `char *` 并释放；返回解析好的 JSON（成功返回 NULL 时给 `Value::Null`）。
fn take(raw: *mut std::ffi::c_char) -> Value {
    if raw.is_null() {
        return Value::Null;
    }
    let text = unsafe { std::ffi::CStr::from_ptr(raw) }
        .to_str()
        .unwrap()
        .to_owned();
    unsafe { qj_string_free(raw) };
    serde_json::from_str(&text).unwrap_or(Value::String(text))
}

fn account_json() -> &'static str {
    r#"{"identities":[],"sessions":[],"consents":{"clipboard":false,"sync":false,"input_log":false,"llm":false,"memory":false}}"#
}

const GRANT: &str = r#"{"token":"sjt_new","user_id":7,"session_id":9,"new_user":true}"#;

#[test]
fn create_space_writes_the_token_and_hands_back_nothing() {
    let (url, rx) = fake_server_sequence(vec![("200 OK", GRANT), ("200 OK", account_json())]);
    let path = cloud_toml("create", &url, None);

    let device = c_string("小美的 iPhone");
    let raw = unsafe {
        qj_space_create(
            path.to_str().unwrap().as_ptr().cast(),
            device.as_ptr(),
            true,
        )
    };
    assert!(raw.is_null(), "{:?}", take(raw));

    let head = rx.recv().unwrap();
    assert_eq!(request_line(&head), "POST /v1/space HTTP/1.1");
    assert!(!has_authorization(&head), "{head}");

    let config = CloudConfig::read(&path).unwrap();
    assert_eq!(config.token, "sjt_new");
    assert_eq!(config.user_id, Some(7));
}

#[test]
fn create_space_without_consent_never_reaches_the_server() {
    let (url, rx) = fake_server_sequence(vec![("200 OK", GRANT)]);
    let path = cloud_toml("consent", &url, None);

    let raw =
        unsafe { qj_space_create(path.to_str().unwrap().as_ptr().cast(), ptr::null(), false) };
    let failure = take(raw);
    assert_eq!(failure["code"], "consent_required");

    assert!(rx.try_recv().is_err(), "不该发请求");
    assert!(CloudConfig::read(&path).unwrap().token.is_empty());
}

#[test]
fn a_new_device_joins_polls_and_lands_signed_in() {
    let (url, rx) = fake_server_sequence(vec![
        (
            "200 OK",
            r#"{"request_id":"r1","secret":"s1","expires_at":1791043200000}"#,
        ),
        (
            "200 OK",
            r#"{"state":"approved","token":"sjt_new","user_id":7,"session_id":9,"new_user":true}"#,
        ),
        ("200 OK", account_json()),
    ]);
    let path = cloud_toml("join", &url, None);
    let code = c_string("K7P2-9QXM");

    let raw = unsafe {
        qj_pair_join(
            path.to_str().unwrap().as_ptr().cast(),
            code.as_ptr(),
            ptr::null(),
        )
    };
    let grant = take(raw);
    assert_eq!(grant["request_id"], "r1");
    assert_eq!(grant["secret"], "s1");
    let join_head = rx.recv().unwrap();
    assert_eq!(request_line(&join_head), "POST /v1/pair/join HTTP/1.1");
    assert!(!has_authorization(&join_head), "{join_head}");
    assert!(
        CloudConfig::read(&path).unwrap().token.is_empty(),
        "输码还没拿到会话"
    );

    let (request_id, secret) = (c_string("r1"), c_string("s1"));
    let raw = unsafe {
        qj_pair_poll(
            path.to_str().unwrap().as_ptr().cast(),
            request_id.as_ptr(),
            secret.as_ptr(),
        )
    };
    assert_eq!(take(raw)["state"], "approved");
    let poll_head = rx.recv().unwrap();
    assert_eq!(request_line(&poll_head), "GET /v1/pair/join/r1 HTTP/1.1");
    assert!(
        poll_head.to_ascii_lowercase().contains("x-pair-secret: s1"),
        "{poll_head}"
    );

    // 会话在轮询这一步就地落盘，返回值里没有令牌
    assert_eq!(CloudConfig::read(&path).unwrap().token, "sjt_new");
}

#[test]
fn a_denied_request_leaves_the_device_signed_out() {
    let (url, _rx) = fake_server_sequence(vec![("200 OK", r#"{"state":"denied"}"#)]);
    let path = cloud_toml("denied", &url, None);
    let (request_id, secret) = (c_string("r1"), c_string("s1"));

    let raw = unsafe {
        qj_pair_poll(
            path.to_str().unwrap().as_ptr().cast(),
            request_id.as_ptr(),
            secret.as_ptr(),
        )
    };
    assert_eq!(take(raw)["state"], "denied");
    assert!(CloudConfig::read(&path).unwrap().token.is_empty());
}

#[test]
fn a_bad_code_and_a_full_space_are_told_apart() {
    for (status, body, code, hint) in [
        (
            "404 Not Found",
            r#"{"error":"pair code not found or expired","code":"bad_code"}"#,
            "bad_code",
            "重新输一张",
        ),
        (
            "409 Conflict",
            r#"{"error":"space already has 5 devices","code":"device_limit"}"#,
            "device_limit",
            "删一台",
        ),
    ] {
        let (url, _rx) = fake_server_sequence(vec![(status, body)]);
        let path = cloud_toml("bad", &url, None);
        let code_in = c_string("K7P2-9QXM");
        let raw = unsafe {
            qj_pair_join(
                path.to_str().unwrap().as_ptr().cast(),
                code_in.as_ptr(),
                ptr::null(),
            )
        };
        let failure = take(raw);
        assert_eq!(failure["code"], code, "{failure}");
        assert!(
            failure["message"].as_str().unwrap().contains(hint),
            "{failure}"
        );
    }
}

#[test]
fn the_old_device_lists_and_decides() {
    let (url, rx) = fake_server_sequence(vec![
        (
            "200 OK",
            r#"[{"id":"r1","name":"新手机","platform":"ios","at":1791043200000}]"#,
        ),
        ("204 No Content", ""),
    ]);
    let path = cloud_toml("decide", &url, Some("sjt_old"));

    let raw = unsafe { qj_pair_requests(path.to_str().unwrap().as_ptr().cast()) };
    let requests = take(raw);
    assert_eq!(requests[0]["id"], "r1");
    assert_eq!(requests[0]["name"], "新手机");
    let list_head = rx.recv().unwrap();
    assert_eq!(request_line(&list_head), "GET /v1/pair/requests HTTP/1.1");
    assert!(list_head.contains("Bearer sjt_old"), "{list_head}");

    let request_id = c_string("r1");
    let raw = unsafe {
        qj_pair_decide(
            path.to_str().unwrap().as_ptr().cast(),
            request_id.as_ptr(),
            true,
        )
    };
    assert!(raw.is_null(), "{:?}", take(raw));
    let decide_head = rx.recv().unwrap();
    assert_eq!(
        request_line(&decide_head),
        "POST /v1/pair/requests/r1 HTTP/1.1"
    );
}

#[test]
fn the_old_device_side_needs_a_token() {
    let (url, rx) = fake_server_sequence(vec![("200 OK", "{}")]);
    let path = cloud_toml("no-token", &url, None);

    for failure in [
        take(unsafe { qj_pair_code(path.to_str().unwrap().as_ptr().cast()) }),
        take(unsafe { qj_pair_requests(path.to_str().unwrap().as_ptr().cast()) }),
    ] {
        assert_eq!(failure["code"], "not_signed_in", "{failure}");
    }
    assert!(rx.try_recv().is_err(), "没登录不该发请求");
}

#[test]
fn pair_code_hands_back_the_shown_form() {
    let (url, _rx) = fake_server_sequence(vec![(
        "200 OK",
        r#"{"code":"K7P2-9QXM","expires_at":1791043200000}"#,
    )]);
    let path = cloud_toml("code", &url, Some("sjt_old"));

    let raw = unsafe { qj_pair_code(path.to_str().unwrap().as_ptr().cast()) };
    let reply = take(raw);
    // 字段叫 pair_code 而不是 code：失败那个 JSON 里 code 是错误种类
    assert_eq!(reply["pair_code"], "K7P2-9QXM");
    assert_eq!(reply["expires_at"], 1791043200000i64);
    assert!(reply.get("message").is_none(), "{reply}");
}

#[test]
fn a_broken_path_is_an_invalid_argument() {
    assert_eq!(
        take(unsafe { qj_pair_code(ptr::null()) })["code"],
        "invalid_argument"
    );
    assert_eq!(
        take(unsafe { qj_pair_join(ptr::null(), ptr::null(), ptr::null()) })["code"],
        "invalid_argument"
    );
}
