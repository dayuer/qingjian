//! 青简 Cloud 服务端：SQLite 存设备与事件，axum 提供 HTTP 接口与 SSE 推送。二进制入口在 `main.rs`。

pub mod cli;
pub mod error;
pub mod http;
pub mod llm;
pub mod prune;
pub mod store;

pub use error::ServerError;
pub use http::{AppState, router};
pub use store::Store;

/// 当前 Unix 毫秒。
pub fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or_default()
}
