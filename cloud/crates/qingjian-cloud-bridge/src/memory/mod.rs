//! 本地记忆（spec「2A 本地记忆」）：对象、记忆卡、当前场景的存储，打字时的提示，以及 C 接口。
//! 数据在学习数据目录的 `memory/` 下（iOS 开了完全访问时是 App Group 的 `Qingjian/memory/`）；
//! App 整份读写，键盘只读（「记一笔」「知道了」与当前场景除外）；两个进程的读-改-写都在 `memory/.lock` 的文件锁里。
//! 卡片的种类与来源用 proto 的 `CardKind`、`CardSource`。

mod card;
mod cards_file;
mod contact;
mod dismissed_file;
mod error;
mod hint;
mod local_date;
mod pronoun;
mod recent;
mod snapshot;
mod store;

#[cfg(test)]
mod tests;

use std::time::{SystemTime, UNIX_EPOCH};

use qingjian_cloud_proto::{CardKind, Scene};

use self::cards_file::CardsFile;
use self::dismissed_file::DismissedFile;
use crate::scope::ScopeState;

pub use self::card::Card;
pub use self::contact::Contact;
pub use self::error::MemoryError;
pub use self::hint::{Hint, HintIndex, HintReason, days_away, panel_cards, reminder_text};
pub use self::local_date::LocalDate;
pub use self::pronoun::Pronoun;
pub use self::recent::RecentText;
pub use self::snapshot::MemorySnapshot;
pub use self::store::{DEFAULT_LOCK_TIMEOUT, KEYBOARD_LOCK_TIMEOUT, MemoryStore};

/// 学习数据目录下放记忆的子目录。
pub const MEMORY_DIR: &str = "memory";

/// 恋爱场景最多几个对象。
pub const MAX_CONTACTS: usize = 8;

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

/// 当前对象不在名单上（被删了）、不是恋爱场景的人，或当前不在恋爱场景：退回不指定。
pub(crate) fn sanitized_scope(mut state: ScopeState, contacts: &[Contact]) -> ScopeState {
    let known = state.contact_id.as_deref().is_some_and(|id| {
        contacts
            .iter()
            .any(|c| c.id == id && c.scene == Scene::Dating)
    });
    if state.scene != Scene::Dating || !known {
        state.contact_id = None;
    }
    state
}
