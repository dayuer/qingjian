//! 青简 Cloud 的协议：HTTP 接口路径、请求与响应的 JSON 类型。服务端与各客户端共用这一份定义。
//!
//! 所有变化都是带全局递增 `seq` 的事件，客户端记住拉到哪个 `seq`，断线后从那里补拉。

mod account;
mod apple_client;
mod apple_sign_in;
mod card_kind;
mod card_page;
mod card_source;
mod config_doc;
mod consents;
mod contact_registration;
mod device;
mod email_start;
mod email_verify;
mod event;
mod event_kind;
mod event_page;
mod feature;
mod handoff_exchange;
mod handoff_grant;
mod identity_info;
mod input_log;
mod learning_page;
mod learning_push;
mod learning_row;
mod memory_accepted;
mod memory_card;
mod memory_item;
mod memory_kind;
mod memory_push;
mod platform;
mod processor_info;
mod push_clip;
mod put_card;
mod put_consent;
mod scene;
mod session_grant;
mod session_info;
mod whoami;

pub use account::Account;
pub use apple_client::AppleClient;
pub use apple_sign_in::AppleSignIn;
pub use card_kind::CardKind;
pub use card_page::CardPage;
pub use card_source::CardSource;
pub use config_doc::{ConfigDoc, PutConfig};
pub use consents::Consents;
pub use contact_registration::ContactRegistration;
pub use device::Device;
pub use email_start::EmailStart;
pub use email_verify::EmailVerify;
pub use event::Event;
pub use event_kind::EventKind;
pub use event_page::EventPage;
pub use feature::Feature;
pub use handoff_exchange::HandoffExchange;
pub use handoff_grant::HandoffGrant;
pub use identity_info::IdentityInfo;
pub use input_log::{InputLogLine, InputLogPage, InputLogPush};
pub use learning_page::LearningPage;
pub use learning_push::{CountDelta, LearningPush, MAX_LEARNING_PUSH, SetDelete, SetPut};
pub use learning_row::LearningRow;
pub use memory_accepted::MemoryAccepted;
pub use memory_card::MemoryCard;
pub use memory_item::MemoryItem;
pub use memory_kind::MemoryKind;
pub use memory_push::MemoryPush;
pub use platform::Platform;
pub use processor_info::ProcessorInfo;
pub use push_clip::PushClip;
pub use put_card::PutCard;
pub use put_consent::PutConsent;
pub use scene::Scene;
pub use session_grant::SessionGrant;
pub use session_info::SessionInfo;
pub use whoami::Whoami;

/// 剪贴板单条文本的上限（UTF-8 字节）。
pub const MAX_CLIP_BYTES: usize = 1024 * 1024;

/// 一次拉取最多返回多少条事件。
pub const MAX_PAGE: usize = 500;

/// 会话令牌的前缀，便于在配置里认出来；旧的设备令牌是 `qjc_`，已作废。
pub const TOKEN_PREFIX: &str = "sjt_";

/// 客户端发送的出境同意文本版本；改了同意文本就换版本号，服务端只认已知版本。
pub const CROSS_BORDER_CONSENT_VERSION: &str = "2026-10-04";

/// `POST` 上传素材（需要 `memory` 功能）。
pub const PATH_MEMORY_MATERIALS: &str = "/v1/memory/materials";

/// `PUT` / `DELETE` 登记与删除对象，路径后接 `/{contact_id}`。
pub const PATH_MEMORY_CONTACTS: &str = "/v1/memory/contacts";

/// `GET` 处理素材的大模型供应商。
pub const PATH_MEMORY_PROCESSOR: &str = "/v1/memory/processor";

/// `GET`（带 `?since=&limit=`）拉卡片；`PUT` / `DELETE` 后接 `/{card_id}`。
pub const PATH_MEMORY_CARDS: &str = "/v1/memory/cards";

/// 一次拉卡片最多返回多少张。
pub const MAX_CARD_PAGE: usize = 500;

/// 手动卡文字最多多少字（按 `chars().count()`）。
pub const MAX_CARD_TEXT_CHARS: usize = 200;

/// 一张卡的关键词最多多少个。
pub const MAX_CARD_KEYWORDS: usize = 8;

/// 一次上传最多多少条素材。
pub const MAX_MEMORY_ITEMS: usize = 100;

/// 每条素材文字最多多少字节。
pub const MAX_MEMORY_TEXT_BYTES: usize = 2000;

/// 健康检查，不要鉴权。
pub const PATH_HEALTH: &str = "/healthz";

/// `POST` Apple 登录（不要鉴权）。
pub const PATH_AUTH_APPLE: &str = "/v1/auth/apple";

/// `POST` 给邮箱发验证码（不要鉴权），成功是 204。
pub const PATH_AUTH_EMAIL_START: &str = "/v1/auth/email/start";

/// `POST` 邮箱加验证码登录（不要鉴权）。
pub const PATH_AUTH_EMAIL_VERIFY: &str = "/v1/auth/email/verify";

/// `POST` 网页登录的一次性码加 verifier 换会话（不要鉴权）。
pub const PATH_AUTH_HANDOFF: &str = "/v1/auth/handoff";

/// `GET` 账号；`DELETE` 删账号。
pub const PATH_ACCOUNT: &str = "/v1/account";

/// `DELETE /v1/sessions/{id}` 注销某台设备，`DELETE /v1/sessions/current` 退出登录。
pub const PATH_SESSIONS: &str = "/v1/sessions";

/// `PUT /v1/consents/{feature}` 开关一项功能。
pub const PATH_CONSENTS: &str = "/v1/consents";

/// 网页登录页（Mac 用 `ASWebAuthenticationSession` 打开）。
pub const PATH_LOGIN: &str = "/login";

/// 网页登录回跳的 URL scheme：`sujian://auth?handoff=…`。
pub const LOGIN_CALLBACK_SCHEME: &str = "sujian";

/// 查询当前会话对应的设备名与本用户最新的 seq。
pub const PATH_WHOAMI: &str = "/v1/whoami";

/// `POST` 上传一条剪贴板；`DELETE /v1/clipboard/{seq}` 删除一条。
pub const PATH_CLIPBOARD: &str = "/v1/clipboard";

/// `GET ?since=&limit=` 拉取事件。
pub const PATH_EVENTS: &str = "/v1/events";

/// `GET ?since=` SSE 推送：先补发 `since` 之后的积压，再推实时事件。
pub const PATH_STREAM: &str = "/v1/events/stream";

/// `POST` 推送学习数据的变化；`GET ?since=&limit=` 拉取合并后的当前值。
pub const PATH_LEARNING: &str = "/v1/learning";

/// `GET` / `PUT` 输入法配置文件。
pub const PATH_CONFIG: &str = "/v1/config";

/// `POST` 上传一批输入日志；`GET ?since=&limit=` 拉取所有设备的。
pub const PATH_INPUT_LOG: &str = "/v1/input-log";

/// `POST` 清空服务器上所有设备的输入日志。
pub const PATH_INPUT_LOG_CLEAR: &str = "/v1/input-log/clear";

/// 大模型代理（OpenAI 兼容）。输入法的接口地址填 `https://<服务器>/v1`。
pub const PATH_CHAT: &str = "/v1/chat/completions";
