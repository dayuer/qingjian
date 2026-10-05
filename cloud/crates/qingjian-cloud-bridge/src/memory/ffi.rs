//! 本地记忆的 C 接口，与 `include/qingjian_bridge.h` 一一对应。`qj_scope_*` 与键盘用的 `qj_memory_*` 带会话（只在主线程上用）；
//! `qj_memory_read` / `qj_memory_write` / `qj_memory_materials` / `qj_memory_material_delete` 是 App 用的，按学习数据目录传（与 `qj_settings_*` 同一做法）。
//! 返回的字符串都用 `qj_string_free` 释放；失败返回 `{"code","message"}`（见 [`MemoryError`]）。全部折掉 panic。

use std::ffi::c_char;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::Path;
use std::ptr;

use super::{
    Material, MaterialSource, MemoryError, MemorySnapshot, MemoryStore, now_unix, unprocessed,
};
use crate::scope::ContactPick;
use crate::session::{DroppedNotes, Session};
use crate::{owned, path_arg, with};

/// 切当前对象：`contact_id` 为空指针时保持现在选的人不变（幂等），为空字符串时明确不指定。
/// 名单上没有的对象当不指定。`_scene` 是上一版留下的场景参数，这一版已经不看它。
///
/// # Safety
/// `session` 来自 `qj_session_open` 且未释放；`_scene` 为空或有效 UTF-8 C 字符串，`contact_id` 为空或同上。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qj_scope_set(
    session: *mut Session,
    _scene: *const c_char,
    contact_id: *const c_char,
) {
    let pick = ContactPick::from_arg(unsafe { path_arg(contact_id) });
    with(session, (), |s| s.set_scope(&pick));
}

/// `{"contact_id":"…"|null,"used":{"<id>":秒,…}}`；没有记忆的会话返回空指针。
///
/// # Safety
/// 同 [`qj_scope_set`]。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qj_scope_get(session: *mut Session) -> *mut c_char {
    with(session, ptr::null_mut(), |s| {
        s.scope().map_or(ptr::null_mut(), |state| {
            let json = serde_json::json!({
                "contact_id": state.contact_id,
                "used": state.used,
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

/// 键盘「记一笔」：原话存成这个对象的待整理素材（超过 2000 字节切成几条），不再写卡。成功返回空指针，失败返回 `{"code","message"}`
/// （`material_limit`：没整理的装不下这次的几条，另带 `remaining` 与 `needed`）。`source` 取 `clipboard` / `typed`，为空指针或认不得时按 `typed`。
/// 键盘只等 200 毫秒的锁：拿不到（`lock_timeout`）时也返回空指针，表示已接受、稍后写入（内存待办，下次按键、poll、flush 时补写；
/// 补写被拒绝的条数用 [`qj_memory_dropped`] 取）。
///
/// # Safety
/// 同 [`qj_scope_set`]；`contact_id`、`text` 为有效 UTF-8 C 字符串，`source` 为空或同上。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qj_memory_note(
    session: *mut Session,
    contact_id: *const c_char,
    text: *const c_char,
    source: *const c_char,
) -> *mut c_char {
    let (Some(contact_id), Some(text)) = (
        unsafe { path_arg(contact_id) }.map(str::to_owned),
        unsafe { path_arg(text) }.map(str::to_owned),
    ) else {
        return owned(&MemoryError::Invalid("参数无效").to_json());
    };
    let source = unsafe { path_arg(source) }.map_or(MaterialSource::Typed, MaterialSource::parse);
    // 会话为空或 panic 时 with 给的是这个兜底；不能用空指针兜底，空指针在这里表示成功
    let noted = with(session, Err(MemoryError::Invalid("参数无效")), |s| {
        s.memory_note(&contact_id, &text, source)
    });
    match noted {
        Ok(()) => ptr::null_mut(),
        Err(error) => owned(&error.to_json()),
    }
}

/// 待办补写时被拒绝、没记上的条数 `{"material_limit":n,"contact_gone":n}`，取走即清零；都是 0 或会话无效时返回空指针。
///
/// # Safety
/// 同 [`qj_scope_set`]。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qj_memory_dropped(session: *mut Session) -> *mut c_char {
    let dropped = with(
        session,
        DroppedNotes::default(),
        Session::take_dropped_notes,
    );
    if dropped.is_empty() {
        return ptr::null_mut();
    }
    serde_json::to_string(&dropped).map_or(ptr::null_mut(), |json| owned(&json))
}

/// App 用：一个对象没整理的素材 `{"unprocessed_count":n,"materials":[…]}`，按时间倒序（同一时间的按写入倒序）；
/// 整理过 30 天的顺手删掉。参数无效或读不了（开机后还没解锁过、锁超时）时返回空指针。
///
/// # Safety
/// 两个参数为有效 UTF-8 C 字符串。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qj_memory_materials(
    user_dir: *const c_char,
    contact_id: *const c_char,
) -> *mut c_char {
    let (Some(user_dir), Some(contact_id)) = (unsafe { path_arg(user_dir) }, unsafe {
        path_arg(contact_id)
    }) else {
        return ptr::null_mut();
    };
    catch_unwind(|| {
        let all = MemoryStore::open(Path::new(user_dir))
            .materials(contact_id, now_unix())
            .ok()?;
        let mut pending: Vec<Material> = all.into_iter().filter(|m| !m.processed).collect();
        // 先倒过来再稳定排序：同一秒切出来的几条按写入倒序，与整体「新的在上」一致
        pending.reverse();
        pending.sort_by_key(|m| std::cmp::Reverse(m.at));
        let json = serde_json::json!({
            "unprocessed_count": unprocessed(&pending),
            "materials": pending,
        });
        Some(json.to_string())
    })
    .ok()
    .flatten()
    .map_or(ptr::null_mut(), |json| owned(&json))
}

/// App 用：删一条素材；没有这条也算成功。成功返回空指针，失败返回 `{"code","message"}`（`invalid` / `lock_timeout` / `io`）。
///
/// # Safety
/// 三个参数为有效 UTF-8 C 字符串。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qj_memory_material_delete(
    user_dir: *const c_char,
    contact_id: *const c_char,
    client_id: *const c_char,
) -> *mut c_char {
    let (Some(user_dir), Some(contact_id), Some(client_id)) = (
        unsafe { path_arg(user_dir) },
        unsafe { path_arg(contact_id) },
        unsafe { path_arg(client_id) },
    ) else {
        return owned(&MemoryError::Invalid("参数无效").to_json());
    };
    let deleted = catch_unwind(|| {
        MemoryStore::open(Path::new(user_dir)).delete_material(contact_id, client_id, now_unix())
    });
    match deleted {
        Ok(Ok(())) => ptr::null_mut(),
        Ok(Err(error)) => owned(&error.to_json()),
        Err(_) => owned(&MemoryError::Invalid("删除时出错").to_json()),
    }
}

/// App 用：**还没归到人的**素材 `{"unprocessed_count":n,"materials":[…]}`，与 [`qj_memory_materials`] 同形、
/// 同样的「新的在上」；这些是首页「+ 记一条」先记下的，还没补上归给谁。参数无效或读不了时返回空指针。
///
/// # Safety
/// `user_dir` 为有效 UTF-8 C 字符串。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qj_memory_unassigned_materials(user_dir: *const c_char) -> *mut c_char {
    let Some(user_dir) = (unsafe { path_arg(user_dir) }) else {
        return ptr::null_mut();
    };
    catch_unwind(|| {
        let all = MemoryStore::open(Path::new(user_dir))
            .unassigned_materials(now_unix())
            .ok()?;
        let mut pending: Vec<Material> = all.into_iter().filter(|m| !m.processed).collect();
        // 先倒过来再稳定排序：同一秒切出来的几条按写入倒序，与整体「新的在上」一致
        pending.reverse();
        pending.sort_by_key(|m| std::cmp::Reverse(m.at));
        Some(
            serde_json::json!({
                "unprocessed_count": unprocessed(&pending),
                "materials": pending,
            })
            .to_string(),
        )
    })
    .ok()
    .flatten()
    .map_or(ptr::null_mut(), |json| owned(&json))
}

/// App 用：首页「+ 记一条」——把一句话存成**不绑对象**的待整理素材，切段与上限跟键盘「记一笔」同一套。
/// 成功返回空指针（与 [`qj_memory_note`] 一样，存下了什么都不用回；要看内容调 [`qj_memory_unassigned_materials`]）；
/// 失败返回 `{"code","message"}`（`material_limit` 另带 `remaining` / `needed`、`invalid`、`lock_timeout`、`io`）。
/// `source` 取 `clipboard` / `typed`，空指针或认不得时按 `typed`。
///
/// # Safety
/// `user_dir`、`text` 为有效 UTF-8 C 字符串，`source` 可为空指针。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qj_memory_unassigned_note(
    user_dir: *const c_char,
    text: *const c_char,
    source: *const c_char,
) -> *mut c_char {
    let (Some(user_dir), Some(text)) = (unsafe { path_arg(user_dir) }, unsafe { path_arg(text) })
    else {
        return owned(&MemoryError::Invalid("参数无效").to_json());
    };
    let source = unsafe { path_arg(source) }.map_or(MaterialSource::Typed, MaterialSource::parse);
    let added = catch_unwind(|| {
        MemoryStore::open(Path::new(user_dir)).add_unassigned_material(text, source, now_unix())
    });
    match added {
        Ok(Ok(_)) => ptr::null_mut(),
        Ok(Err(error)) => owned(&error.to_json()),
        Err(_) => owned(&MemoryError::Invalid("记一条时出错").to_json()),
    }
}

/// App 用：「补上」——把无主桶里的一条素材归到某个人名下（写进那个人的 `materials.jsonl`）。
/// 成功返回空指针；失败返回 `{"code","message"}`（`invalid`：人或素材不在，以及 `lock_timeout` / `io`）。
///
/// # Safety
/// 三个参数为有效 UTF-8 C 字符串。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qj_memory_assign_material(
    user_dir: *const c_char,
    client_id: *const c_char,
    contact_id: *const c_char,
) -> *mut c_char {
    let (Some(user_dir), Some(client_id), Some(contact_id)) = (
        unsafe { path_arg(user_dir) },
        unsafe { path_arg(client_id) },
        unsafe { path_arg(contact_id) },
    ) else {
        return owned(&MemoryError::Invalid("参数无效").to_json());
    };
    let assigned = catch_unwind(|| {
        MemoryStore::open(Path::new(user_dir)).assign_material(client_id, contact_id, now_unix())
    });
    match assigned {
        Ok(Ok(())) => ptr::null_mut(),
        Ok(Err(error)) => owned(&error.to_json()),
        Err(_) => owned(&MemoryError::Invalid("归人时出错").to_json()),
    }
}

/// 键盘里新建一个对象。成功返回 `{"id":"…"}`，失败返回 `{"code","message"}`
/// （`lock_timeout`：App 正占着锁，再点一次；`invalid` / `io`）。
/// `pronoun` 取 `ta` / `ta_m` / `ta_f` / `name`，认不得或为空指针时按 `ta`。
/// `_scene` 是上一版留下的场景参数，这一版已经不看它。
///
/// # Safety
/// 同 [`qj_scope_set`]；`name` 为有效 UTF-8 C 字符串，`pronoun`、`_scene` 可为空指针。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qj_memory_add_contact(
    session: *mut Session,
    name: *const c_char,
    pronoun: *const c_char,
    _scene: *const c_char,
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
/// `conflict` 表示这期间别处改过卡片，App 重读、合并后再写。
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
