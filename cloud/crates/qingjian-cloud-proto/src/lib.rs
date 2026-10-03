//! 青简 Cloud 的协议：HTTP 接口路径、请求与响应的 JSON 类型。服务端与各客户端共用这一份定义。
//!
//! 所有变化都是带全局递增 `seq` 的事件，客户端记住拉到哪个 `seq`，断线后从那里补拉。

mod event;
mod event_kind;
mod event_page;
mod push_clip;
mod whoami;

pub use event::Event;
pub use event_kind::EventKind;
pub use event_page::EventPage;
pub use push_clip::PushClip;
pub use whoami::Whoami;

/// 剪贴板单条文本的上限（UTF-8 字节）。
pub const MAX_CLIP_BYTES: usize = 1024 * 1024;

/// 一次拉取最多返回多少条事件。
pub const MAX_PAGE: usize = 500;

/// 设备令牌的前缀，便于在配置里认出来。
pub const TOKEN_PREFIX: &str = "qjc_";

/// 健康检查，不要鉴权。
pub const PATH_HEALTH: &str = "/healthz";

/// 查询当前令牌对应的设备。
pub const PATH_WHOAMI: &str = "/v1/whoami";

/// `POST` 上传一条剪贴板；`DELETE /v1/clipboard/{seq}` 删除一条。
pub const PATH_CLIPBOARD: &str = "/v1/clipboard";

/// `GET ?since=&limit=` 拉取事件。
pub const PATH_EVENTS: &str = "/v1/events";

/// `GET ?since=` SSE 推送：先补发 `since` 之后的积压，再推实时事件。
pub const PATH_STREAM: &str = "/v1/events/stream";
