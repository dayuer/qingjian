//! 青简 Cloud 的 iOS 桥：把上游 [`qingjian_core::Engine`] 包成 C ABI，键盘扩展（Swift）链这个静态库。
//!
//! 头文件在 `include/qingjian_bridge.h`，改了这里的签名要同步改它。约定：
//! 返回 `char *` 的函数交出所有权，调用方用 [`qj_string_free`] 释放；会话指针只在一个线程（主线程）上用。

mod error;
mod session;

use std::ffi::{CStr, CString, c_char};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::Path;
use std::ptr;

pub use self::error::BridgeError;
pub use self::session::Session;

/// 打开会话；`user_dir` 可为空（只在内存里学习）。失败返回空指针。
///
/// # Safety
/// `data_dir` 必须是有效的 UTF-8 C 字符串，`user_dir` 为空或同上。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qj_session_open(
    data_dir: *const c_char,
    user_dir: *const c_char,
) -> *mut Session {
    let Some(data_dir) = (unsafe { path_arg(data_dir) }) else {
        return ptr::null_mut();
    };
    let user_dir = unsafe { path_arg(user_dir) };
    let opened = catch_unwind(|| Session::open(Path::new(data_dir), user_dir.map(Path::new)));
    match opened {
        Ok(Ok(session)) => Box::into_raw(Box::new(session)),
        Ok(Err(error)) => {
            tracing::error!(%error, "会话打开失败");
            ptr::null_mut()
        }
        Err(_) => ptr::null_mut(),
    }
}

/// # Safety
/// `session` 来自 [`qj_session_open`] 且之后不再使用；可为空。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qj_session_free(session: *mut Session) {
    if !session.is_null() {
        drop(unsafe { Box::from_raw(session) });
    }
}

/// 敲一个字母进拼音缓冲区。
///
/// # Safety
/// `session` 来自 [`qj_session_open`] 且未释放。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qj_push(session: *mut Session, c: u32) {
    if let Some(c) = char::from_u32(c) {
        with(session, (), |s| s.push(c));
    }
}

/// # Safety
/// 同 [`qj_push`]。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qj_backspace(session: *mut Session) {
    with(session, (), Session::backspace);
}

/// # Safety
/// 同 [`qj_push`]。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qj_clear(session: *mut Session) {
    with(session, (), Session::clear);
}

/// # Safety
/// 同 [`qj_push`]。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qj_composing(session: *mut Session) -> bool {
    with(session, false, |s| s.composing())
}

/// 拼音行显示串；没在组句时是空串。
///
/// # Safety
/// 同 [`qj_push`]；返回值用 [`qj_string_free`] 释放。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qj_preedit(session: *mut Session) -> *mut c_char {
    with(session, ptr::null_mut(), |s| owned(s.preedit()))
}

/// # Safety
/// 同 [`qj_push`]。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qj_candidate_count(session: *mut Session) -> u32 {
    with(session, 0, |s| {
        u32::try_from(s.candidates().len()).unwrap_or(u32::MAX)
    })
}

/// 第 `index` 个候选的文字；越界返回空指针。
///
/// # Safety
/// 同 [`qj_preedit`]。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qj_candidate_text(session: *mut Session, index: u32) -> *mut c_char {
    with(session, ptr::null_mut(), |s| {
        s.candidates()
            .get(index as usize)
            .map_or(ptr::null_mut(), |c| owned(&c.text))
    })
}

/// 上屏第 `index` 个候选，返回要插入的文字；越界返回空指针。
///
/// # Safety
/// 同 [`qj_preedit`]。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qj_commit(session: *mut Session, index: u32) -> *mut c_char {
    with(session, ptr::null_mut(), |s| {
        s.commit(index as usize)
            .map_or(ptr::null_mut(), |text| owned(&text))
    })
}

/// 敲过的字母原样上屏。
///
/// # Safety
/// 同 [`qj_preedit`]。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qj_take_raw(session: *mut Session) -> *mut c_char {
    with(session, ptr::null_mut(), |s| owned(&s.take_raw()))
}

/// 没在组句时敲的标点，返回要插入的文字（中文模式下转全角）。
///
/// # Safety
/// 同 [`qj_preedit`]。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qj_punctuate(session: *mut Session, c: u32) -> *mut c_char {
    let Some(c) = char::from_u32(c) else {
        return ptr::null_mut();
    };
    with(session, ptr::null_mut(), |s| owned(&s.punctuate(c)))
}

/// 学习数据落盘。
///
/// # Safety
/// 同 [`qj_push`]。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qj_flush(session: *mut Session) {
    with(session, (), Session::flush);
}

/// # Safety
/// `text` 来自本库返回的 `char *` 且之后不再使用；可为空。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qj_string_free(text: *mut c_char) {
    if !text.is_null() {
        drop(unsafe { CString::from_raw(text) });
    }
}

/// 空指针与 panic 都折成 `fallback`：panic 穿过 `extern "C"` 会直接 abort，带着宿主应用的键盘一起没。
fn with<T>(session: *mut Session, fallback: T, f: impl FnOnce(&mut Session) -> T) -> T {
    // SAFETY: 调用方保证指针来自 qj_session_open、未释放、且只在一个线程上用。
    let Some(session) = (unsafe { session.as_mut() }) else {
        return fallback;
    };
    catch_unwind(AssertUnwindSafe(|| f(session))).unwrap_or(fallback)
}

/// 文字里不会有 NUL（候选与拼音都来自词库），万一有就截到 NUL 前。
fn owned(text: &str) -> *mut c_char {
    let text = text.split('\0').next().unwrap_or_default();
    CString::new(text).map_or(ptr::null_mut(), CString::into_raw)
}

unsafe fn path_arg<'a>(raw: *const c_char) -> Option<&'a str> {
    if raw.is_null() {
        return None;
    }
    unsafe { CStr::from_ptr(raw) }.to_str().ok()
}
