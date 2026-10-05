//! 本地记忆（spec「2A 本地记忆」）：对象、记忆卡、「记一笔」的待整理素材、当前对象的存储，打字时的提示，以及 C 接口。
//! 数据在学习数据目录的 `memory/` 下（iOS 开了完全访问时是 App Group 的 `Qingjian/memory/`）；
//! App 整份读写，键盘只读（「记一笔」「知道了」与当前对象除外）；两个进程的读-改-写都在 `memory/.lock` 的文件锁里。
//! 卡片的种类与来源用 proto 的 `CardKind`、`CardSource`。

mod card;
mod cards_file;
mod contact;
mod dismissed_file;
mod error;
mod ffi;
mod hint;
mod initial;
mod local_date;
mod materials;
mod pronoun;
mod recent;
mod snapshot;
mod store;
mod validate;

#[cfg(test)]
mod tests;

use std::time::{SystemTime, UNIX_EPOCH};

use qingjian_cloud_proto::CardKind;

use self::cards_file::CardsFile;
use self::dismissed_file::DismissedFile;
use crate::scope::{ContactPick, ScopeState};

pub use self::card::Card;
pub use self::contact::{Contact, MAX_DISPLAY_NAME_CHARS, MAX_PINNED};
pub use self::error::MemoryError;
pub use self::hint::{Hint, HintIndex, HintReason, days_away, panel_cards, reminder_text};
pub use self::initial::initial_of;
pub use self::local_date::LocalDate;
pub use self::materials::{
    CONTACT_PLACEHOLDER, CloudState, Consent, MAX_UNPROCESSED_MATERIALS, Material, MaterialSource,
    PROCESSED_KEEP_DAYS, UploadDecision, mask_contact_names, should_upload, split_note,
    unprocessed,
};
pub use self::pronoun::Pronoun;
pub use self::recent::RecentText;
pub use self::snapshot::MemorySnapshot;
pub use self::store::{DEFAULT_LOCK_TIMEOUT, KEYBOARD_LOCK_TIMEOUT, MemoryStore};

/// 学习数据目录下放记忆的子目录。
pub const MEMORY_DIR: &str = "memory";

/// 提示拿最近上屏的多少个字去匹配。
pub const RECENT_CHARS: usize = 24;

/// 对象与卡片的 id：16 字节随机数的小写十六进制（32 位，不含名字）。
pub fn new_id() -> Result<String, MemoryError> {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes).map_err(|error| MemoryError::Io(std::io::Error::other(error)))?;
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

/// 现在的 Unix 秒；系统时钟早于 1970 时当 0。
pub fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| {
            i64::try_from(elapsed.as_secs()).unwrap_or(i64::MAX)
        })
}

/// 只有日子与约定的 `when` 有意义（提醒、面板排序、校验都按它）。
pub fn has_date(kind: CardKind) -> bool {
    matches!(kind, CardKind::Date | CardKind::Promise)
}

/// 当前对象必须在名单上，否则退回不指定；`used` 里不在名单上的人也去掉。
pub(crate) fn sanitized_scope(mut state: ScopeState, contacts: &[Contact]) -> ScopeState {
    if !state
        .contact_id
        .as_deref()
        .is_some_and(|id| contacts.iter().any(|c| c.id == id))
    {
        state.contact_id = None;
    }
    state
        .used
        .retain(|id, _| contacts.iter().any(|c| &c.id == id));
    state
}

/// 按 `pick` 定当前对象。
pub(crate) fn scope_with(
    state: ScopeState,
    pick: &ContactPick,
    contacts: &[Contact],
) -> ScopeState {
    let mut state = sanitized_scope(state, contacts);
    state.contact_id = match pick {
        // 保持现在选的人：sanitize 完直接回去（原来靠 `state.last`，场景去掉后没有「上次」了）
        ContactPick::Keep => return state,
        ContactPick::Nobody => None,
        ContactPick::Contact(id) => Some(id.clone()),
    };
    sanitized_scope(state, contacts)
}
