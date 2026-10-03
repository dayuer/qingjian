//! 服务端错误：存储错误与请求错误，后者在 HTTP 层转成状态码与 `{"error": …}`。

use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

#[derive(Debug, thiserror::Error)]
pub enum ServerError {
    #[error("missing or invalid device token")]
    Unauthorized,

    #[error("{0}")]
    BadRequest(String),

    #[error("not found")]
    NotFound,

    #[error("clipboard text exceeds {0} bytes")]
    TooLarge(usize),

    #[error("version conflict, server has {0}")]
    Conflict(u64),

    #[error("device already exists: {0}")]
    DeviceExists(String),

    #[error("this server has no LLM key configured")]
    LlmDisabled,

    #[error("upstream LLM: {0}")]
    Upstream(String),

    #[error("database: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("io: {0}")]
    Io(#[from] std::io::Error),

    #[error("random source unavailable: {0}")]
    Random(String),
}

impl IntoResponse for ServerError {
    fn into_response(self) -> Response {
        let status = match &self {
            Self::Unauthorized => StatusCode::UNAUTHORIZED,
            Self::BadRequest(_) => StatusCode::BAD_REQUEST,
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::TooLarge(_) => StatusCode::PAYLOAD_TOO_LARGE,
            Self::DeviceExists(_) | Self::Conflict(_) => StatusCode::CONFLICT,
            Self::LlmDisabled => StatusCode::SERVICE_UNAVAILABLE,
            Self::Upstream(_) => {
                tracing::warn!(error = %self, "上游大模型请求失败");
                StatusCode::BAD_GATEWAY
            }
            Self::Database(_) | Self::Io(_) | Self::Random(_) => {
                tracing::error!(error = %self, "请求处理失败");
                StatusCode::INTERNAL_SERVER_ERROR
            }
        };
        (
            status,
            Json(serde_json::json!({ "error": self.to_string() })),
        )
            .into_response()
    }
}
