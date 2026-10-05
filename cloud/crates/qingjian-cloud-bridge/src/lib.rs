//! 青简 Cloud 的 iOS 桥：把上游 [`qingjian_core::Engine`] 包成 C ABI，键盘扩展（Swift）链这个静态库。
//!
//! 头文件在 `include/qingjian_bridge.h`，改了这里的签名要同步改它。约定：
//! 返回 `char *` 的函数交出所有权，调用方用 [`qj_string_free`] 释放；会话指针只在一个线程（主线程）上用。

mod account;
mod clipboard;
mod cloud_config;
mod entry;
mod error;
mod memory;
mod rewrite;
mod scope;
mod session;
mod settings;

use std::ffi::{CStr, CString, c_char};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::Path;
use std::ptr;

pub use self::account::AccountStatus;
pub use self::clipboard::{ClipOffer, Clipboard};
pub use self::cloud_config::{CloudConfig, DEFAULT_SERVER, parse_failure_note};
pub use self::entry::Entry;
pub use self::error::BridgeError;
pub use self::memory::{
    CONTACT_PLACEHOLDER, Card, CloudState, Consent, Contact, DEFAULT_LOCK_TIMEOUT,
    DEFAULT_SCENE_ID, DEFAULT_SCENE_NAME, Hint, HintIndex, HintReason, KEYBOARD_LOCK_TIMEOUT,
    LocalDate, MAX_DISPLAY_NAME_CHARS, MAX_PINNED, MAX_SCENE_NAME_CHARS, MAX_UNPROCESSED_MATERIALS,
    MEMORY_DIR, Material, MaterialSource, MemoryError, MemorySnapshot, MemoryStore,
    PROCESSED_KEEP_DAYS, Pronoun, RECENT_CHARS, RecentText, Scene, UploadDecision, days_away,
    has_date, is_scene_id, mask_contact_names, new_id, now_unix, panel_cards, reminder_text,
    should_upload, split_note, unprocessed, validate_scenes,
};
pub use self::rewrite::{RewriteState, Rewriter};
pub use self::scope::{
    ContactPick, ScopeHandle, ScopeState, ScopedLearner, contact_learning_dir, is_contact_id,
};
pub use self::session::{DroppedNotes, Session};
pub use self::settings::{DomainSetting, SchemeOption, Settings};

/// 打开会话；`user_dir` 可为空（只在内存里学习），`config` 为空时用 `user_dir` 下的 `config.toml`，
/// `cloud_config` 可为空或指向不存在的文件（完全离线）。失败返回空指针。
///
/// # Safety
/// `data_dir` 必须是有效的 UTF-8 C 字符串，其余参数为空或同上。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qj_session_open(
    data_dir: *const c_char,
    user_dir: *const c_char,
    config: *const c_char,
    cloud_config: *const c_char,
) -> *mut Session {
    let Some(data_dir) = (unsafe { path_arg(data_dir) }) else {
        return ptr::null_mut();
    };
    let user_dir = unsafe { path_arg(user_dir) };
    let config = unsafe { path_arg(config) };
    let cloud =
        unsafe { path_arg(cloud_config) }.and_then(|path| CloudConfig::load(Path::new(path)));
    let opened = catch_unwind(AssertUnwindSafe(|| {
        Session::open(
            Path::new(data_dir),
            user_dir.map(Path::new),
            config.map(Path::new),
            cloud,
        )
    }));
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
        u32::try_from(s.entries().len()).unwrap_or(u32::MAX)
    })
}

/// 第 `index` 个候选的文字；越界返回空指针。
///
/// # Safety
/// 同 [`qj_preedit`]。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qj_candidate_text(session: *mut Session, index: u32) -> *mut c_char {
    with(session, ptr::null_mut(), |s| {
        s.entries()
            .get(index as usize)
            .map_or(ptr::null_mut(), |entry| owned(entry.text()))
    })
}

/// 第 `index` 格是不是大模型给的（候选栏用别的样式画）。
///
/// # Safety
/// 同 [`qj_push`]。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qj_candidate_is_cloud(session: *mut Session, index: u32) -> bool {
    with(session, false, |s| {
        s.entries().get(index as usize).is_some_and(Entry::is_cloud)
    })
}

/// 一次取回整个候选栏：每格是「`0`/`1`（本地 / 云端）+ 文字」，格与格之间用 U+001E 隔开；没有候选返回空串。
/// 每键只过一次边界，省掉逐格取的上百次分配。
///
/// # Safety
/// 同 [`qj_preedit`]。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qj_candidates(session: *mut Session) -> *mut c_char {
    with(session, ptr::null_mut(), |s| {
        let mut joined = String::new();
        for (index, entry) in s.entries().iter().enumerate() {
            if index > 0 {
                joined.push(CANDIDATE_SEPARATOR);
            }
            joined.push(if entry.is_cloud() { '1' } else { '0' });
            // 分隔符不会出现在词库与大模型的候选里，万一有就去掉，免得格子错位
            joined.extend(entry.text().chars().filter(|&c| c != CANDIDATE_SEPARATOR));
        }
        owned(&joined)
    })
}

/// [`qj_candidates`] 里格与格之间的分隔符（ASCII 记录分隔符）。
const CANDIDATE_SEPARATOR: char = '\u{1e}';

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

/// 没在组字时直接输出的字符（空格、回车），记进输入日志的文本流。
///
/// # Safety
/// 同 [`qj_push`]。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qj_note_passthrough(session: *mut Session, c: u32) {
    if let Some(c) = char::from_u32(c) {
        with(session, (), |s| s.note_passthrough(c));
    }
}

/// 学习数据落盘。
///
/// # Safety
/// 同 [`qj_push`]。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qj_flush(session: *mut Session) {
    with(session, (), Session::flush);
}

/// 宿主光标前后的文字（`documentContextBeforeInput` / `AfterInput`），每次敲键前更新，联想请求带上。
///
/// # Safety
/// 同 [`qj_push`]；两个字符串为空时当空串。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qj_set_context(
    session: *mut Session,
    before: *const c_char,
    after: *const c_char,
) {
    let before = unsafe { path_arg(before) }.unwrap_or_default().to_owned();
    let after = unsafe { path_arg(after) }.unwrap_or_default().to_owned();
    with(session, (), |s| s.set_context(&before, &after));
}

/// 键盘可见期间定时调（几百毫秒一次）：合并别的设备的学习数据、取回大模型结果。候选栏变了返回 true。
///
/// # Safety
/// 同 [`qj_push`]。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qj_poll(session: *mut Session) -> bool {
    with(session, false, Session::poll)
}

/// 配了青简 Cloud（大模型或同步至少开了一样）。
///
/// # Safety
/// 同 [`qj_push`]。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qj_cloud_enabled(session: *mut Session) -> bool {
    with(session, false, |s| s.cloud_enabled())
}

/// 素材会被整理：`path` 指向的 `cloud.toml` 有服务器地址和登录令牌（[`CloudConfig::load`]），并且同意了「记忆」（`memory`），不联网。
/// App 的「待整理」引导与键盘记一笔的 toast 按它二选一：没同意时素材不会上传，不能说「明早整理」。文件不在、读不了、没登录都是 false。
///
/// # Safety
/// `path` 为空或有效的 UTF-8 C 字符串。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qj_memory_cloud_ready(path: *const c_char) -> bool {
    let Some(path) = (unsafe { path_arg(path) }) else {
        return false;
    };
    catch_unwind(|| CloudConfig::load(Path::new(path)).is_some_and(|config| config.memory))
        .unwrap_or(false)
}

/// 马上同步一轮学习数据（键盘出现时调）。
///
/// # Safety
/// 同 [`qj_push`]。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qj_sync_now(session: *mut Session) {
    with(session, (), |s| s.sync_now());
}

/// 润色能不能用（配了青简 Cloud 且开了大模型）。
///
/// # Safety
/// 同 [`qj_push`]。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qj_rewrite_available(session: *mut Session) -> bool {
    with(session, false, |s| s.rewriter().is_some())
}

/// 开始润色 `text`；之前没回来的那次作废。
///
/// # Safety
/// 同 [`qj_push`]；`text` 为有效 UTF-8 C 字符串。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qj_rewrite_start(session: *mut Session, text: *const c_char) {
    let Some(text) = (unsafe { path_arg(text) }).map(str::to_owned) else {
        return;
    };
    with(session, (), |s| {
        if let Some(rewriter) = s.rewriter() {
            // TODO(Task 5)：这个 C 接口要加第三个参数（技能 id），在那之前用当前生效的那个。
            rewriter.start(&text, None);
        }
    });
}

/// 0 空闲、1 等待中、2 结果就绪（用 [`qj_rewrite_take`] 取）、3 失败、4 模型给的不合用（已丢掉）。
///
/// # Safety
/// 同 [`qj_push`]。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qj_rewrite_status(session: *mut Session) -> u32 {
    with(session, 0, |s| s.rewriter().map_or(0, Rewriter::status))
}

/// 取走润色结果；没就绪返回空指针。
///
/// # Safety
/// 同 [`qj_preedit`]。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qj_rewrite_take(session: *mut Session) -> *mut c_char {
    with(session, ptr::null_mut(), |s| {
        s.rewriter()
            .and_then(Rewriter::take)
            .map_or(ptr::null_mut(), |text| owned(&text))
    })
}

/// 放弃润色（用户开始打字、关掉了结果）。
///
/// # Safety
/// 同 [`qj_push`]。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qj_rewrite_cancel(session: *mut Session) {
    with(session, (), |s| {
        if let Some(rewriter) = s.rewriter() {
            rewriter.cancel();
        }
    });
}

/// 焦点在验证码、密码、信用卡号这类输入框时设 true：不学习、不记日志、不发云端，剪贴板与润色也停。
///
/// # Safety
/// 同 [`qj_push`]。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qj_set_private(session: *mut Session, private: bool) {
    with(session, (), |s| s.set_private(private));
}

/// 配了跨设备剪贴板。
///
/// # Safety
/// 同 [`qj_push`]。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qj_clipboard_enabled(session: *mut Session) -> bool {
    with(session, false, |s| s.clipboard_enabled())
}

/// 后台拉一次别的设备的剪贴板（键盘弹出时调）。
///
/// # Safety
/// 同 [`qj_push`]。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qj_clip_refresh(session: *mut Session) {
    with(session, (), |s| s.refresh_clipboard());
}

/// 别的设备最近复制、还没处理过的文字；没有返回空指针。
///
/// # Safety
/// 同 [`qj_preedit`]。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qj_clip_offer_text(session: *mut Session) -> *mut c_char {
    with(session, ptr::null_mut(), |s| {
        s.clip_offer()
            .map_or(ptr::null_mut(), |offer| owned(&offer.text))
    })
}

/// 那条文字来自哪台设备；没有提示时返回空指针。
///
/// # Safety
/// 同 [`qj_preedit`]。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qj_clip_offer_device(session: *mut Session) -> *mut c_char {
    with(session, ptr::null_mut(), |s| {
        s.clip_offer()
            .map_or(ptr::null_mut(), |offer| owned(&offer.device))
    })
}

/// 用户插入或关掉了提示：不再给这一条。
///
/// # Safety
/// 同 [`qj_push`]。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qj_clip_handled(session: *mut Session) {
    with(session, (), |s| s.clip_handled());
}

/// 把本机剪贴板的文字发给别的设备（用户点了按钮才调）。
///
/// # Safety
/// 同 [`qj_push`]；`text` 为有效 UTF-8 C 字符串。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qj_clip_push(session: *mut Session, text: *const c_char) {
    let Some(text) = (unsafe { path_arg(text) }).map(str::to_owned) else {
        return;
    };
    with(session, (), |s| s.clip_push(&text));
}

/// 主 App 设置页：读 `config.toml`（没有按缺省），领域词库按 `dicts_dir` 列；返回 JSON（[`Settings`]），失败返回空。
///
/// # Safety
/// 两个参数都是有效的 UTF-8 C 字符串。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qj_settings_read(
    config_path: *const c_char,
    dicts_dir: *const c_char,
) -> *mut c_char {
    let (Some(config_path), Some(dicts_dir)) = (unsafe { path_arg(config_path) }, unsafe {
        path_arg(dicts_dir)
    }) else {
        return ptr::null_mut();
    };
    catch_unwind(|| {
        let settings = Settings::read(Path::new(config_path), Path::new(dicts_dir));
        serde_json::to_string(&settings).ok()
    })
    .ok()
    .flatten()
    .map_or(ptr::null_mut(), |json| owned(&json))
}

/// 把设置页的 JSON 写回 `config.toml`；成功返回空，失败返回原因（给用户看）。
///
/// # Safety
/// 同 [`qj_settings_read`]。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qj_settings_write(
    config_path: *const c_char,
    json: *const c_char,
) -> *mut c_char {
    let (Some(config_path), Some(json)) =
        (unsafe { path_arg(config_path) }, unsafe { path_arg(json) })
    else {
        return owned("参数无效");
    };
    let written = catch_unwind(|| {
        let settings: Settings = serde_json::from_str(json).map_err(|e| e.to_string())?;
        settings.write(Path::new(config_path))
    });
    match written {
        Ok(Ok(())) => ptr::null_mut(),
        Ok(Err(error)) => owned(&error),
        Err(_) => owned("写入时出错"),
    }
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
pub(crate) fn with<T>(session: *mut Session, fallback: T, f: impl FnOnce(&mut Session) -> T) -> T {
    // SAFETY: 调用方保证指针来自 qj_session_open、未释放、且只在一个线程上用。
    let Some(session) = (unsafe { session.as_mut() }) else {
        return fallback;
    };
    catch_unwind(AssertUnwindSafe(|| f(session))).unwrap_or(fallback)
}

/// 文字里不会有 NUL（候选与拼音都来自词库），万一有就截到 NUL 前。
pub(crate) fn owned(text: &str) -> *mut c_char {
    let text = text.split('\0').next().unwrap_or_default();
    CString::new(text).map_or(ptr::null_mut(), CString::into_raw)
}

pub(crate) unsafe fn path_arg<'a>(raw: *const c_char) -> Option<&'a str> {
    if raw.is_null() {
        return None;
    }
    unsafe { CStr::from_ptr(raw) }.to_str().ok()
}
