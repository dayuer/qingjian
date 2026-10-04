//! 账号的 C 接口（主 App 用），与 `include/qingjian_bridge.h` 一一对应。都是阻塞的网络请求，Swift 在后台调。
//! `path` 是 App Group 里的 `cloud.toml`。返回值：状态返回 JSON（参数无效时为空）；操作成功返回空，失败返回给用户看的原因。
//! 都用 `qj_string_free` 释放。

use std::ffi::c_char;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::Path;
use std::ptr;

use qingjian_cloud_proto::Feature;

use super::AccountStatus;
use crate::{owned, path_arg};

/// 账号页的 JSON（[`AccountStatus`]）。没登录时不联网。
///
/// # Safety
/// `path` 是有效的 UTF-8 C 字符串。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qj_account_status(path: *const c_char) -> *mut c_char {
    let Some(path) = (unsafe { path_arg(path) }) else {
        return ptr::null_mut();
    };
    catch_unwind(AssertUnwindSafe(|| {
        serde_json::to_string(&AccountStatus::load(Path::new(path))).ok()
    }))
    .ok()
    .flatten()
    .map_or(ptr::null_mut(), |json| owned(&json))
}

/// Apple 登录：`nonce` 是原始值（交给 Apple 的是它的 SHA-256 十六进制），`device` 是设备名（可为空）。
///
/// # Safety
/// 前四个参数是有效的 UTF-8 C 字符串，`device` 为空或同上。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qj_account_sign_in_apple(
    path: *const c_char,
    identity_token: *const c_char,
    authorization_code: *const c_char,
    nonce: *const c_char,
    device: *const c_char,
) -> *mut c_char {
    let (Some(path), Some(identity_token), Some(authorization_code), Some(nonce)) = (
        unsafe { path_arg(path) },
        unsafe { path_arg(identity_token) },
        unsafe { path_arg(authorization_code) },
        unsafe { path_arg(nonce) },
    ) else {
        return owned("参数无效");
    };
    let device = unsafe { path_arg(device) }.unwrap_or_default();
    outcome(|| {
        super::sign_in_apple(
            Path::new(path),
            identity_token,
            authorization_code,
            nonce,
            device,
        )
    })
}

/// 给邮箱发验证码。
///
/// # Safety
/// 两个参数都是有效的 UTF-8 C 字符串。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qj_account_email_start(
    path: *const c_char,
    email: *const c_char,
) -> *mut c_char {
    let (Some(path), Some(email)) = (unsafe { path_arg(path) }, unsafe { path_arg(email) }) else {
        return owned("参数无效");
    };
    outcome(|| super::email_start(Path::new(path), email))
}

/// 邮箱加验证码登录。
///
/// # Safety
/// 前三个参数是有效的 UTF-8 C 字符串，`device` 为空或同上。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qj_account_email_verify(
    path: *const c_char,
    email: *const c_char,
    code: *const c_char,
    device: *const c_char,
) -> *mut c_char {
    let (Some(path), Some(email), Some(code)) = (
        unsafe { path_arg(path) },
        unsafe { path_arg(email) },
        unsafe { path_arg(code) },
    ) else {
        return owned("参数无效");
    };
    let device = unsafe { path_arg(device) }.unwrap_or_default();
    outcome(|| super::email_verify(Path::new(path), email, code, device))
}

/// 开关一项功能（`clipboard` / `sync` / `input_log` / `llm`），成功后写回 `cloud.toml` 的开关。
///
/// # Safety
/// 两个字符串参数都是有效的 UTF-8 C 字符串。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qj_account_set_consent(
    path: *const c_char,
    feature: *const c_char,
    enabled: bool,
) -> *mut c_char {
    let (Some(path), Some(feature)) = (unsafe { path_arg(path) }, unsafe {
        path_arg(feature).and_then(Feature::parse)
    }) else {
        return owned("参数无效");
    };
    outcome(|| super::set_consent(Path::new(path), feature, enabled))
}

/// 注销本账号的某台设备。
///
/// # Safety
/// `path` 是有效的 UTF-8 C 字符串。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qj_account_revoke_session(
    path: *const c_char,
    session_id: i64,
) -> *mut c_char {
    let Some(path) = (unsafe { path_arg(path) }) else {
        return owned("参数无效");
    };
    outcome(|| super::revoke_session(Path::new(path), session_id))
}

/// 退出登录：本机总会退出，服务器上没注销掉只记日志。
///
/// # Safety
/// `path` 是有效的 UTF-8 C 字符串。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qj_account_sign_out(path: *const c_char) -> *mut c_char {
    let Some(path) = (unsafe { path_arg(path) }) else {
        return owned("参数无效");
    };
    outcome(|| super::sign_out(Path::new(path)))
}

/// 删账号：服务器删成功才清本机令牌。
///
/// # Safety
/// `path` 是有效的 UTF-8 C 字符串。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qj_account_delete(path: *const c_char) -> *mut c_char {
    let Some(path) = (unsafe { path_arg(path) }) else {
        return owned("参数无效");
    };
    outcome(|| super::delete_account(Path::new(path)))
}

/// 成功返回空，失败返回原因；panic 折成一句通用的话（穿过 `extern "C"` 会直接 abort）。
fn outcome(f: impl FnOnce() -> Result<(), String>) -> *mut c_char {
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(Ok(())) => ptr::null_mut(),
        Ok(Err(reason)) => owned(&reason),
        Err(_) => owned("出错了，请重试"),
    }
}
