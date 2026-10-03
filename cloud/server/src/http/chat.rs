//! `POST /v1/chat/completions`：转给上游。输入法的 `api_key` 填设备令牌，这里换成服务器上的密钥。
//! 流式（`"stream": true`）原样逐块转发；非流式的成功回答进缓存，并从 `usage` 记 token。

use axum::body::{Body, Bytes};
use axum::extract::State;
use axum::http::header::CONTENT_TYPE;
use axum::response::{IntoResponse, Response};
use serde_json::Value;

use super::{AppState, AuthDevice};
use crate::ServerError;
use crate::llm::{ResponseCache, inject_history};

pub async fn chat(
    State(state): State<AppState>,
    AuthDevice(device): AuthDevice,
    body: Bytes,
) -> Result<Response, ServerError> {
    let upstream = state
        .llm
        .clone()
        .filter(|upstream| upstream.enabled())
        .ok_or(ServerError::LlmDisabled)?;
    let mut request: Value = serde_json::from_slice(&body)
        .map_err(|e| ServerError::BadRequest(format!("bad JSON: {e}")))?;
    if let Some(model) = &upstream.config.model {
        request["model"] = Value::String(model.clone());
    }
    if upstream.config.context_chars > 0 {
        let history = state.store.recent_text(upstream.config.context_chars)?;
        inject_history(&mut request, &history);
    }
    let stream = request
        .get("stream")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let body = serde_json::to_vec(&request).map_err(|e| ServerError::BadRequest(e.to_string()))?;
    let key = ResponseCache::key(&body);
    if !stream && let Some(cached) = state.cache.get(key) {
        state.store.record_usage(&device, true, 0, 0)?;
        return Ok(([(CONTENT_TYPE, "application/json")], cached).into_response());
    }

    let response = upstream
        .client
        .post(upstream.chat_url())
        .bearer_auth(&upstream.config.api_key)
        .header(CONTENT_TYPE, "application/json")
        .body(body)
        .send()
        .await
        .map_err(|e| ServerError::Upstream(e.to_string()))?;
    let status = response.status();
    let content_type = response
        .headers()
        .get(CONTENT_TYPE)
        .cloned()
        .unwrap_or_else(|| "application/json".parse().expect("static header"));

    if stream {
        state.store.record_usage(&device, false, 0, 0)?;
        let body = Body::from_stream(response.bytes_stream());
        return Ok((status, [(CONTENT_TYPE, content_type)], body).into_response());
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|e| ServerError::Upstream(e.to_string()))?;
    if status.is_success() {
        let usage = serde_json::from_slice::<Value>(&bytes)
            .ok()
            .and_then(|reply| reply.get("usage").cloned())
            .unwrap_or_default();
        let tokens = |name: &str| usage.get(name).and_then(Value::as_i64).unwrap_or(0);
        state.store.record_usage(
            &device,
            false,
            tokens("prompt_tokens"),
            tokens("completion_tokens"),
        )?;
        state.cache.insert(key, bytes.to_vec());
    } else {
        // 上游的错误原样还给输入法（它会记日志），这里只记状态码，不记可能带用户文本的正文
        tracing::warn!(status = status.as_u16(), device = %device.name, "上游大模型返回错误");
    }
    Ok((status, [(CONTENT_TYPE, content_type)], bytes).into_response())
}
