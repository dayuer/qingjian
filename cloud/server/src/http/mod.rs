//! HTTP 接口：路由与共享状态。路径常量在 `qingjian-cloud-proto`。

mod auth;
mod chat;
mod clipboard;
mod events;
mod input_log;
mod learning;
mod state;
mod stream;

use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::routing::{delete, get, post};
use qingjian_cloud_proto::{
    MAX_CLIP_BYTES, PATH_CHAT, PATH_CLIPBOARD, PATH_CONFIG, PATH_EVENTS, PATH_HEALTH,
    PATH_INPUT_LOG, PATH_INPUT_LOG_CLEAR, PATH_LEARNING, PATH_STREAM, PATH_WHOAMI,
};

pub use auth::AuthDevice;
pub use state::AppState;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route(PATH_HEALTH, get(|| async { "ok" }))
        .route(PATH_WHOAMI, get(events::whoami))
        .route(PATH_CLIPBOARD, post(clipboard::push))
        .route(
            &format!("{PATH_CLIPBOARD}/{{seq}}"),
            delete(clipboard::delete),
        )
        .route(PATH_EVENTS, get(events::list))
        .route(PATH_STREAM, get(stream::stream))
        .route(PATH_LEARNING, post(learning::push).get(learning::list))
        .route(
            PATH_CONFIG,
            get(learning::get_config).put(learning::put_config),
        )
        .route(PATH_INPUT_LOG, post(input_log::push).get(input_log::list))
        .route(PATH_INPUT_LOG_CLEAR, post(input_log::clear))
        .route(PATH_CHAT, post(chat::chat))
        // 文本上限之外留出 JSON 转义的余量（中文按 \uXXXX 写最多 6 倍，但正常客户端发 UTF-8 原文）
        .layer(DefaultBodyLimit::max(MAX_CLIP_BYTES * 2))
        .with_state(state)
}
