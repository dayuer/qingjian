//! 账号 C 接口的烟测：按 C 签名直接调，参数无效不崩、返回值都能用 `qj_string_free` 释放。

use std::ffi::{CStr, CString, c_char};
use std::ptr;

use qingjian_cloud_bridge as _;

unsafe extern "C" {
    fn qj_account_status(path: *const c_char) -> *mut c_char;
    fn qj_account_set_consent(
        path: *const c_char,
        feature: *const c_char,
        enabled: bool,
    ) -> *mut c_char;
    fn qj_account_email_start(path: *const c_char, email: *const c_char) -> *mut c_char;
    fn qj_string_free(text: *mut c_char);
}

fn take(raw: *mut c_char) -> Option<String> {
    if raw.is_null() {
        return None;
    }
    let text = unsafe { CStr::from_ptr(raw) }
        .to_string_lossy()
        .into_owned();
    unsafe { qj_string_free(raw) };
    Some(text)
}

fn temp_toml(name: &str, text: &str) -> CString {
    let dir = std::env::temp_dir().join(format!("qj-ffi-{name}-{}", std::process::id()));
    std::fs::remove_dir_all(&dir).ok();
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("cloud.toml");
    std::fs::write(&path, text).unwrap();
    CString::new(path.to_str().unwrap()).unwrap()
}

#[test]
fn set_consent_with_unknown_feature_reports_invalid_argument() {
    let path = temp_toml("bogus", "server = \"http://127.0.0.1:1\"\n");
    let feature = CString::new("bogus").unwrap();
    let reason = take(unsafe { qj_account_set_consent(path.as_ptr(), feature.as_ptr(), true) });
    assert_eq!(reason.as_deref(), Some("参数无效"));
}

#[test]
fn status_with_null_path_returns_null() {
    assert!(unsafe { qj_account_status(ptr::null()) }.is_null());
}

#[test]
fn status_without_login_is_json() {
    let path = temp_toml("status", "server = \"http://127.0.0.1:1\"\n");
    let json = take(unsafe { qj_account_status(path.as_ptr()) }).unwrap();
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(value["signed_in"], false);
}

#[test]
fn email_start_with_empty_email_does_not_crash() {
    let path = temp_toml("email", "server = \"http://127.0.0.1:1\"\n");
    let email = CString::new("").unwrap();
    let reason = take(unsafe { qj_account_email_start(path.as_ptr(), email.as_ptr()) });
    assert!(reason.is_some_and(|text| !text.is_empty()));
    assert!(take(unsafe { qj_account_email_start(path.as_ptr(), ptr::null()) }).is_some());
}
