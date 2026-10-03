//! 学习数据同步：输入法的词频、按输入串的选择、个人 n-gram、敲错表、个人英文词（计数表）与用户词（集合表）。
//!
//! 本程序不碰输入法内存里的数据，只读它落盘的文件；别的设备的增量写成收件箱 `sync/inbox.tsv`，
//! 由（打了分叉补丁的）输入法合并进内存、落盘、删掉文件。增量相加，与本机正在学的互不覆盖。
//! 正确性靠「基线」：上次同步后双方一致的那份数据。本机增量 = 当前文件 − 基线，别的设备的增量 = 服务器值 − 基线。

mod count_entry;
mod learning_state;
mod learning_sync;
mod snapshot;
mod table;

pub use count_entry::CountEntry;
pub use learning_state::LearningState;
pub use learning_sync::{LearningOutcome, LearningSync};
pub use snapshot::Snapshot;
pub use table::Table;

/// 收件箱在输入法数据目录里的相对路径（与输入法那边 `host/cloud/inbox.rs` 一致）。
pub const INBOX: &str = "sync/inbox.tsv";
