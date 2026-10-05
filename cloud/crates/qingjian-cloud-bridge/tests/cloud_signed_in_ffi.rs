//! `qj_cloud_signed_in`：只看 `cloud.toml` 里的令牌前缀，不联网。

use std::ffi::{CString, c_char};

use qingjian_cloud_bridge as _;

unsafe extern "C" {
    fn qj_cloud_signed_in(path: *const c_char) -> bool;
}

fn cloud_toml(name: &str, text: &str) -> CString {
    let dir = std::env::temp_dir().join(format!("qj-signed-{name}-{}", std::process::id()));
    std::fs::remove_dir_all(&dir).ok();
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("cloud.toml");
    std::fs::write(&path, text).unwrap();
    CString::new(path.to_str().unwrap()).unwrap()
}

#[test]
fn a_session_token_means_signed_in() {
    let path = cloud_toml(
        "token",
        "server = \"http://127.0.0.1:1\"\ntoken = \"sjt_t\"\n",
    );
    assert!(unsafe { qj_cloud_signed_in(path.as_ptr()) });
}

#[test]
fn no_token_a_legacy_token_or_a_broken_file_means_signed_out() {
    for (name, text) in [
        ("none", "server = \"s\"\n"),
        ("legacy", "server = \"s\"\ntoken = \"qjc_old\"\n"),
        ("broken", "这不是 toml ]"),
    ] {
        let path = cloud_toml(name, text);
        assert!(!unsafe { qj_cloud_signed_in(path.as_ptr()) }, "{name}");
    }
}

#[test]
fn a_missing_file_or_a_null_path_is_signed_out_not_a_crash() {
    let missing = CString::new("/nonexistent-qj-signed/cloud.toml").unwrap();
    assert!(!unsafe { qj_cloud_signed_in(missing.as_ptr()) });
    assert!(!unsafe { qj_cloud_signed_in(std::ptr::null()) });
}
