//! 对象的分区学习（spec「2A 本地记忆 · 分区学习」）：[`ScopedLearner`] 包全局与对象两层 `FrequencyLearner`，
//! 会话用 [`ScopeHandle`] 换叠加层，当前场景与对象存在 [`ScopeState`]（`memory/state.json`）。
//! 目录约定：对象层 `memory/<对象 id>/learning/`，文件名与全局层一样是 `user*.tsv`。
//! 场景自 2026-10-05 起只是用户自建的分组，不再有场景层的分区学习（那时把写死的三个场景并成了一个）。

mod handle;
mod overlay;
mod pick;
mod scoped_learner;
mod snapshot;
mod state;

#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, PoisonError};

use qingjian_learning::FrequencyLearner;

use self::overlay::Overlay;
use self::snapshot::Snapshot;

pub use self::handle::ScopeHandle;
pub use self::pick::ContactPick;
pub use self::scoped_learner::ScopedLearner;
pub use self::state::ScopeState;

/// 对象 id 是 16 字节随机数的小写十六进制；它要拿来当目录名，不合格的一律不认（挡 `../` 之类）。
pub fn is_contact_id(id: &str) -> bool {
    id.len() == 32
        && id
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

pub fn contact_learning_dir(memory_dir: &Path, contact_id: &str) -> PathBuf {
    memory_dir.join(contact_id).join("learning")
}

/// 打开一层：目录不在就建（`FrequencyLearner` 落盘时不建父目录）；读不了（开机后还没解锁过时的数据保护、权限）
/// 退回只在内存里学，不拿空表覆盖用户文件。
pub(crate) fn load_layer(dir: &Path) -> FrequencyLearner {
    if let Err(error) = std::fs::create_dir_all(dir) {
        tracing::warn!(%error, "学习数据目录建不了，这一层只在内存里学习");
        return FrequencyLearner::default();
    }
    let path = dir.join("user.tsv");
    FrequencyLearner::from_path(&path).unwrap_or_else(|error| {
        tracing::error!(path = %path.display(), %error, "学习数据读取失败，这一层只在内存里学习");
        FrequencyLearner::default()
    })
}

/// 叠加层的锁：只有键盘主线程在拿；中毒了也照用里面的数据。
fn lock(overlay: &Mutex<Overlay>) -> MutexGuard<'_, Overlay> {
    overlay.lock().unwrap_or_else(PoisonError::into_inner)
}
