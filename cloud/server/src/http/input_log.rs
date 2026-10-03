//! `POST /v1/input-log` 上传、`GET /v1/input-log` 拉取、`POST /v1/input-log/clear` 清空。

use axum::Json;
use axum::extract::{Query, State};
use qingjian_cloud_proto::{InputLogPage, InputLogPush, MAX_PAGE};
use serde::Deserialize;

use super::{AppState, AuthDevice};
use crate::ServerError;

#[derive(Deserialize)]
pub struct PageQuery {
    #[serde(default)]
    since: u64,

    limit: Option<usize>,
}

pub async fn push(
    State(state): State<AppState>,
    AuthDevice(device): AuthDevice,
    Json(push): Json<InputLogPush>,
) -> Result<Json<InputLogPage>, ServerError> {
    let latest = state.store.push_input_log(&device, &push)?;
    tracing::debug!(device = %device.name, lines = push.lines.len(), "输入日志");
    Ok(Json(InputLogPage {
        lines: Vec::new(),
        latest,
        generation: 0,
    }))
}

pub async fn list(
    State(state): State<AppState>,
    AuthDevice(_): AuthDevice,
    Query(query): Query<PageQuery>,
) -> Result<Json<InputLogPage>, ServerError> {
    let limit = query.limit.unwrap_or(MAX_PAGE).clamp(1, MAX_PAGE);
    Ok(Json(state.store.input_log_since(query.since, limit)?))
}

pub async fn clear(
    State(state): State<AppState>,
    AuthDevice(device): AuthDevice,
) -> Result<Json<InputLogPage>, ServerError> {
    let generation = state.store.clear_input_log()?;
    tracing::info!(device = %device.name, generation, "清空了所有设备的输入日志");
    Ok(Json(InputLogPage {
        lines: Vec::new(),
        latest: 0,
        generation,
    }))
}
