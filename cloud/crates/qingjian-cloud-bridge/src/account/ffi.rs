//! 账号的 C 接口（主 App 用），与 `include/qingjian_bridge.h` 一一对应。都是阻塞的网络请求，Swift 在后台调。
//! `path` 是 App Group 里的 `cloud.toml`。返回值：状态返回 JSON（参数无效时为空）；操作成功返回空，失败返回 JSON `{"code":"…","message":"…"}`（code 见 `Failure`，message 是给用户看的中文）。
//! 都用 `qj_string_free` 释放。

use std::ffi::c_char;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::Path;
use std::ptr;

use qingjian_cloud_proto::{Feature, PairPoll};

use super::AccountStatus;
use super::failure::Failure;
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
/// 前四个参数是有效的 UTF-8 C 字符串，`device` 为空或同上。`cross_border_consented` 为假时不联网，直接返回 `consent_required`。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qj_account_sign_in_apple(
    path: *const c_char,
    identity_token: *const c_char,
    authorization_code: *const c_char,
    nonce: *const c_char,
    device: *const c_char,
    cross_border_consented: bool,
) -> *mut c_char {
    let (Some(path), Some(identity_token), Some(authorization_code), Some(nonce)) = (
        unsafe { path_arg(path) },
        unsafe { path_arg(identity_token) },
        unsafe { path_arg(authorization_code) },
        unsafe { path_arg(nonce) },
    ) else {
        return owned(&Failure::invalid_argument().to_json());
    };
    let device = unsafe { path_arg(device) }.unwrap_or_default();
    outcome(|| {
        super::sign_in_apple(
            Path::new(path),
            identity_token,
            authorization_code,
            nonce,
            device,
            cross_border_consented,
        )
    })
}

/// 给邮箱发验证码。
///
/// # Safety
/// 两个字符串参数都是有效的 UTF-8 C 字符串。`cross_border_consented` 为假时不联网，直接返回 `consent_required`。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qj_account_email_start(
    path: *const c_char,
    email: *const c_char,
    cross_border_consented: bool,
) -> *mut c_char {
    let (Some(path), Some(email)) = (unsafe { path_arg(path) }, unsafe { path_arg(email) }) else {
        return owned(&Failure::invalid_argument().to_json());
    };
    outcome(|| super::email_start(Path::new(path), email, cross_border_consented))
}

/// 邮箱加验证码登录。
///
/// # Safety
/// 前三个参数是有效的 UTF-8 C 字符串，`device` 为空或同上。`cross_border_consented` 为假时不联网，直接返回 `consent_required`。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qj_account_email_verify(
    path: *const c_char,
    email: *const c_char,
    code: *const c_char,
    device: *const c_char,
    cross_border_consented: bool,
) -> *mut c_char {
    let (Some(path), Some(email), Some(code)) = (
        unsafe { path_arg(path) },
        unsafe { path_arg(email) },
        unsafe { path_arg(code) },
    ) else {
        return owned(&Failure::invalid_argument().to_json());
    };
    let device = unsafe { path_arg(device) }.unwrap_or_default();
    outcome(|| super::email_verify(Path::new(path), email, code, device, cross_border_consented))
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
        return owned(&Failure::invalid_argument().to_json());
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
        return owned(&Failure::invalid_argument().to_json());
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
        return owned(&Failure::invalid_argument().to_json());
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
        return owned(&Failure::invalid_argument().to_json());
    };
    outcome(|| super::delete_account(Path::new(path)))
}

/// 建空间：开通云服务的第一步，不用登录。成功后令牌写进 `cloud.toml`，返回值里没有它。
///
/// # Safety
/// `path` 是有效的 UTF-8 C 字符串，`device` 为空或同上。`cross_border_consented` 为假时不联网，直接返回 `consent_required`。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qj_space_create(
    path: *const c_char,
    device: *const c_char,
    cross_border_consented: bool,
) -> *mut c_char {
    let Some(path) = (unsafe { path_arg(path) }) else {
        return owned(&Failure::invalid_argument().to_json());
    };
    let device = unsafe { path_arg(device) }.unwrap_or_default();
    outcome(|| super::pair::create_space(Path::new(path), device, cross_border_consented))
}

/// 出一张匹配码（要已登录）：成功 `{"pair_code":"K7P2-9QXM","expires_at":…}`。
///
/// # Safety
/// `path` 是有效的 UTF-8 C 字符串。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qj_pair_code(path: *const c_char) -> *mut c_char {
    let Some(path) = (unsafe { path_arg(path) }) else {
        return owned(&Failure::invalid_argument().to_json());
    };
    outcome_json(|| {
        super::pair::pair_code(Path::new(path)).map(|code| {
            // 不叫 `code`：失败的那个 JSON 里 `code` 是错误种类，两个不能重名
            serde_json::json!({"pair_code": code.code, "expires_at": code.expires_at}).to_string()
        })
    })
}

/// 新设备输码申请加入：成功 `{"request_id":…,"secret":…,"expires_at":…}`，`secret` 是轮询的凭据。
///
/// # Safety
/// 两个字符串参数都是有效的 UTF-8 C 字符串，`device` 为空或同上。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qj_pair_join(
    path: *const c_char,
    code: *const c_char,
    device: *const c_char,
) -> *mut c_char {
    let (Some(path), Some(code)) = (unsafe { path_arg(path) }, unsafe { path_arg(code) }) else {
        return owned(&Failure::invalid_argument().to_json());
    };
    let device = unsafe { path_arg(device) }.unwrap_or_default();
    outcome_json(|| {
        super::pair::pair_join(Path::new(path), code, device).map(|grant| {
            serde_json::json!({
                "request_id": grant.request_id,
                "secret": grant.secret,
                "expires_at": grant.expires_at,
            })
            .to_string()
        })
    })
}

/// 轮询这次申请：成功 `{"state":"pending"|"denied"|"approved"}`。
/// `approved` 时会话已经写进 `cloud.toml`，返回值里没有令牌。
///
/// # Safety
/// 三个字符串参数都是有效的 UTF-8 C 字符串。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qj_pair_poll(
    path: *const c_char,
    request_id: *const c_char,
    secret: *const c_char,
) -> *mut c_char {
    let (Some(path), Some(request_id), Some(secret)) = (
        unsafe { path_arg(path) },
        unsafe { path_arg(request_id) },
        unsafe { path_arg(secret) },
    ) else {
        return owned(&Failure::invalid_argument().to_json());
    };
    outcome_json(|| {
        super::pair::pair_poll(Path::new(path), request_id, secret).map(|poll| {
            let state = match poll {
                PairPoll::Pending => "pending",
                PairPoll::Denied => "denied",
                PairPoll::Approved(_) => "approved",
            };
            serde_json::json!({"state": state}).to_string()
        })
    })
}

/// 等这台设备处理的加入申请（要已登录）：成功是一个 JSON 数组。
///
/// # Safety
/// `path` 是有效的 UTF-8 C 字符串。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qj_pair_requests(path: *const c_char) -> *mut c_char {
    let Some(path) = (unsafe { path_arg(path) }) else {
        return owned(&Failure::invalid_argument().to_json());
    };
    outcome_json(|| {
        super::pair::pair_requests(Path::new(path)).and_then(|requests| {
            serde_json::to_string(&requests).map_err(|_| Failure::other("出错了，请重试"))
        })
    })
}

/// 允许或拒绝一条加入申请（要已登录）。
///
/// # Safety
/// 两个字符串参数都是有效的 UTF-8 C 字符串。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qj_pair_decide(
    path: *const c_char,
    request_id: *const c_char,
    allow: bool,
) -> *mut c_char {
    let (Some(path), Some(request_id)) =
        (unsafe { path_arg(path) }, unsafe { path_arg(request_id) })
    else {
        return owned(&Failure::invalid_argument().to_json());
    };
    outcome(|| super::pair::pair_decide(Path::new(path), request_id, allow))
}

/// 成功返回空，失败返回 [`Failure`] 的 JSON；panic 折成一句通用的话（穿过 `extern "C"` 会直接 abort）。
fn outcome(f: impl FnOnce() -> Result<(), Failure>) -> *mut c_char {
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(Ok(())) => ptr::null_mut(),
        Ok(Err(failure)) => owned(&failure.to_json()),
        Err(_) => owned(&Failure::other("出错了，请重试").to_json()),
    }
}

/// 同 [`outcome`]，但成功要交出一个 JSON 串。
fn outcome_json(f: impl FnOnce() -> Result<String, Failure>) -> *mut c_char {
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(Ok(json)) => owned(&json),
        Ok(Err(failure)) => owned(&failure.to_json()),
        Err(_) => owned(&Failure::other("出错了，请重试").to_json()),
    }
}
