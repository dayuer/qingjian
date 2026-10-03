//! 青简 Cloud 的客户端：阻塞式 HTTP 接口（[`Client`]）、SSE 解析、落盘的离线队列（[`Outbox`]），
//! 以及把它们串起来的剪贴板同步线程（[`ClipboardSync`]）。不依赖任何平台 API，Mac / iOS / Windows 壳共用。

mod client;
mod clipboard_sync;
mod error;
mod outbox;
mod sse_reader;
mod sync_state;

pub use client::Client;
pub use clipboard_sync::{ClipboardSync, Incoming, Status, SyncConfig};
pub use error::ClientError;
pub use outbox::Outbox;
pub use sse_reader::SseReader;
pub use sync_state::SyncState;
