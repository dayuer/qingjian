//! `POST /v1/learning` 推变化、`GET /v1/learning` 拉当前值，以及 `GET` / `PUT /v1/config`。

use axum::Json;
use axum::extract::{Query, State};
use qingjian_cloud_proto::{ConfigDoc, LearningPage, LearningPush, MAX_PAGE, PutConfig};
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
    Json(push): Json<LearningPush>,
) -> Result<Json<LearningPage>, ServerError> {
    let latest = state.store.push_learning(&device, &push)?;
    tracing::info!(device = %device.name, changes = push.len(), latest, "学习数据");
    Ok(Json(LearningPage {
        rows: Vec::new(),
        latest,
    }))
}

pub async fn list(
    State(state): State<AppState>,
    AuthDevice(_): AuthDevice,
    Query(query): Query<PageQuery>,
) -> Result<Json<LearningPage>, ServerError> {
    let limit = query.limit.unwrap_or(MAX_PAGE).clamp(1, MAX_PAGE);
    Ok(Json(state.store.learning_since(query.since, limit)?))
}

pub async fn get_config(
    State(state): State<AppState>,
    AuthDevice(_): AuthDevice,
) -> Result<Json<ConfigDoc>, ServerError> {
    state.store.config()?.map(Json).ok_or(ServerError::NotFound)
}

pub async fn put_config(
    State(state): State<AppState>,
    AuthDevice(device): AuthDevice,
    Json(put): Json<PutConfig>,
) -> Result<Json<ConfigDoc>, ServerError> {
    let doc = state.store.put_config(&device, &put)?;
    tracing::info!(device = %device.name, version = doc.version, "配置文件");
    Ok(Json(doc))
}
