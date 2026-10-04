//! 本地记忆的 C 接口，与 `include/qingjian_bridge.h` 一一对应。`qj_scope_*` 与键盘用的 `qj_memory_*` 带会话（只在主线程上用）；
//! `qj_memory_read` / `qj_memory_write` 是 App 用的，按学习数据目录传（与 `qj_settings_*` 同一做法）。
//! 返回的字符串都用 `qj_string_free` 释放；失败返回 `{"code","message"}`（见 [`MemoryError`]）。全部折掉 panic。

use std::ffi::c_char;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::Path;
use std::ptr;

use super::{MemoryError, MemorySnapshot, MemoryStore};
use crate::scope::{parse_scene, scene_name};
use crate::session::Session;
use crate::{owned, path_arg, with};

/// 切场景与对象；`contact_id` 可为空（不指定）。场景认不得时什么都不做。
///
/// # Safety
/// `session` 来自 `qj_session_open` 且未释放；`scene` 为有效 UTF-8 C 字符串，`contact_id` 为空或同上。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qj_scope_set(
    session: *mut Session,
    scene: *const c_char,
    contact_id: *const c_char,
) {
    let Some(scene) = (unsafe { path_arg(scene) }).and_then(parse_scene) else {
        return;
    };
    let contact = unsafe { path_arg(contact_id) }.map(str::to_owned);
    with(session, (), |s| s.set_scope(scene, contact.as_deref()));
}

/// `{"scene":"dating","contact_id":"…"|null}`；没有记忆的会话返回空指针。
///
/// # Safety
/// 同 [`qj_scope_set`]。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qj_scope_get(session: *mut Session) -> *mut c_char {
    with(session, ptr::null_mut(), |s| {
        s.scope().map_or(ptr::null_mut(), |state| {
            let json = serde_json::json!({
                "scene": scene_name(state.scene),
                "contact_id": state.contact_id,
            });
            owned(&json.to_string())
        })
    })
}

/// 宿主换了输入框：清掉最近上屏的字与正在显示的匹配提示（键盘收起时 `qj_flush` 也会清）。
///
/// # Safety
/// 同 [`qj_scope_set`]。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qj_reset_context(session: *mut Session) {
    with(session, (), Session::reset_context);
}

/// 当前提示 `{"card_id","text","reason","more"}`；没有时返回空指针（私密输入时恒为空）。
///
/// # Safety
/// 同 [`qj_scope_set`]。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qj_memory_hint(session: *mut Session) -> *mut c_char {
    with(session, ptr::null_mut(), |s| {
        s.memory_hint()
            .and_then(|hint| serde_json::to_string(hint).ok())
            .map_or(ptr::null_mut(), |json| owned(&json))
    })
}

/// 「知道了」：`today` 为真当天不再出，为假 10 分钟内不再出。
///
/// # Safety
/// 同 [`qj_scope_set`]；`card_id` 为有效 UTF-8 C 字符串。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qj_memory_dismiss(
    session: *mut Session,
    card_id: *const c_char,
    today: bool,
) {
    let Some(card_id) = (unsafe { path_arg(card_id) }).map(str::to_owned) else {
        return;
    };
    with(session, (), |s| s.dismiss_hint(&card_id, today));
}

/// 键盘内对象卡面板：今日相关最多 3 张卡的 JSON 数组；没有记忆的会话返回空指针。
///
/// # Safety
/// 同 [`qj_scope_set`]；`contact_id` 为有效 UTF-8 C 字符串。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qj_memory_cards(
    session: *mut Session,
    contact_id: *const c_char,
) -> *mut c_char {
    let Some(contact_id) = (unsafe { path_arg(contact_id) }).map(str::to_owned) else {
        return ptr::null_mut();
    };
    with(session, ptr::null_mut(), |s| {
        if s.scope().is_none() {
            return ptr::null_mut();
        }
        serde_json::to_string(&s.memory_cards(&contact_id))
            .map_or(ptr::null_mut(), |json| owned(&json))
    })
}

/// 键盘「记一笔」：给对象建一张 `other` 卡。成功返回空指针，失败返回 `{"code","message"}`。
/// 键盘只等 200 毫秒的锁：拿不到（`lock_timeout`）时也返回空指针，表示已接受、稍后写入（内存待办，下次按键、poll、flush 时补写）。
///
/// # Safety
/// 同 [`qj_scope_set`]；两个字符串参数为有效 UTF-8 C 字符串。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qj_memory_note(
    session: *mut Session,
    contact_id: *const c_char,
    text: *const c_char,
) -> *mut c_char {
    let (Some(contact_id), Some(text)) = (
        unsafe { path_arg(contact_id) }.map(str::to_owned),
        unsafe { path_arg(text) }.map(str::to_owned),
    ) else {
        return owned(&MemoryError::Invalid("参数无效").to_json());
    };
    // 会话为空或 panic 时 with 给的是这个兜底；不能用空指针兜底，空指针在这里表示成功
    let noted = with(session, Err(MemoryError::Invalid("参数无效")), |s| {
        s.memory_note(&contact_id, &text)
    });
    match noted {
        Ok(()) => ptr::null_mut(),
        Err(error) => owned(&error.to_json()),
    }
}

/// 键盘里新建一个恋爱场景的对象。成功返回 `{"id":"…"}`，失败返回 `{"code","message"}`
/// （`contact_limit`：恋爱场景已满 8 个；`lock_timeout`：App 正占着锁，再点一次；`invalid` / `io`）。
/// `pronoun` 取 `ta` / `ta_m` / `ta_f` / `name`，认不得或为空指针时按 `ta`。
///
/// # Safety
/// 同 [`qj_scope_set`]；`name` 为有效 UTF-8 C 字符串，`pronoun` 可为空指针。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qj_memory_add_contact(
    session: *mut Session,
    name: *const c_char,
    pronoun: *const c_char,
) -> *mut c_char {
    let Some(name) = (unsafe { path_arg(name) }).map(str::to_owned) else {
        return owned(&MemoryError::Invalid("参数无效").to_json());
    };
    let pronoun = unsafe { path_arg(pronoun) }
        .and_then(|text| serde_json::from_value(serde_json::Value::String(text.to_owned())).ok())
        .unwrap_or_default();
    let added = with(session, Err(MemoryError::Invalid("参数无效")), |s| {
        s.memory_add_contact(&name, pronoun)
    });
    match added {
        Ok(id) => owned(&serde_json::json!({ "id": id }).to_string()),
        Err(error) => owned(&error.to_json()),
    }
}

/// App 用：整份读出 `{"contacts","cards","revs","state","broken"}`；参数无效或有文件读不了（开机后还没解锁过）时返回空指针。
///
/// # Safety
/// `user_dir` 为有效 UTF-8 C 字符串。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qj_memory_read(user_dir: *const c_char) -> *mut c_char {
    let Some(user_dir) = (unsafe { path_arg(user_dir) }) else {
        return ptr::null_mut();
    };
    catch_unwind(|| {
        let snapshot = MemoryStore::open(Path::new(user_dir)).snapshot().ok()?;
        serde_json::to_string(&snapshot).ok()
    })
    .ok()
    .flatten()
    .map_or(ptr::null_mut(), |json| owned(&json))
}

/// App 用：整份写回。成功返回空指针，失败返回 `{"code","message"}`，code 取 `contact_limit` / `invalid` / `conflict` / `lock_timeout`（等了 2 秒没拿到锁）/ `io`；
/// `conflict` 表示键盘这期间改过，App 重读、合并后再写。
///
/// # Safety
/// 两个参数为有效 UTF-8 C 字符串。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qj_memory_write(
    user_dir: *const c_char,
    json: *const c_char,
) -> *mut c_char {
    let (Some(user_dir), Some(json)) = (unsafe { path_arg(user_dir) }, unsafe { path_arg(json) })
    else {
        return owned(&MemoryError::Invalid("参数无效").to_json());
    };
    let written = catch_unwind(AssertUnwindSafe(|| {
        let snapshot: MemorySnapshot =
            serde_json::from_str(json).map_err(|_| MemoryError::Invalid("数据格式不对"))?;
        MemoryStore::open(Path::new(user_dir)).write_snapshot(&snapshot)
    }));
    match written {
        Ok(Ok(())) => ptr::null_mut(),
        Ok(Err(error)) => owned(&error.to_json()),
        Err(_) => owned(&MemoryError::Invalid("写入时出错").to_json()),
    }
}
