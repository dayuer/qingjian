//! 青简 Cloud 的客户端：阻塞式 HTTP 接口（[`Client`]）、SSE 解析、落盘的离线队列（[`Outbox`]），
//! 以及把它们串起来的剪贴板同步线程（[`ClipboardSync`]）、学习数据与配置文件的同步线程（[`DataSync`]）。不依赖任何平台 API，Mac / iOS / Windows 壳共用。

mod client;
mod clipboard_sync;
mod config_sync;
mod data_sync;
mod error;
mod input_log_sync;
mod learning;
mod outbox;
mod sse_reader;
mod supervise;
mod sync_state;

pub use client::Client;
pub use clipboard_sync::{ClipboardSync, Incoming, Status, SyncConfig};
pub use config_sync::{ConfigOutcome, ConfigState, ConfigSync};
pub use data_sync::{DataStatus, DataSync, DataSyncConfig};
pub use error::ClientError;
pub use input_log_sync::{InputLogOutcome, InputLogState, InputLogSync};
pub use learning::{INBOX, LearningOutcome, LearningSync, Snapshot, Table};
pub use outbox::Outbox;
pub use sse_reader::SseReader;
pub use sync_state::SyncState;
