//! 大模型代理：把输入法的 OpenAI 兼容请求原样转给上游，密钥只在服务器上。
//! 可选地在请求前插一条「最近在各设备上输入的文字」当上文（缺省关，要回放评测过再开），
//! 并缓存相同请求的回答、按设备按天记 token 用量。

mod cache;
mod context;
mod llm_config;
mod upstream;

pub use cache::ResponseCache;
pub use context::inject_history;
pub use llm_config::LlmConfig;
pub use upstream::Upstream;
