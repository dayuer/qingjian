//! `GET /v1/events` 拉取与 `GET /v1/whoami`。

use axum::Json;
use axum::extract::{Query, State};
use qingjian_cloud_proto::{EventPage, MAX_PAGE, Whoami};
use serde::Deserialize;

use super::{AppState, AuthDevice};
use crate::ServerError;

#[derive(Deserialize)]
pub struct ListQuery {
    #[serde(default)]
    since: u64,

    limit: Option<usize>,
}

pub async fn list(
    State(state): State<AppState>,
    AuthDevice(_): AuthDevice,
    Query(query): Query<ListQuery>,
) -> Result<Json<EventPage>, ServerError> {
    let limit = query.limit.unwrap_or(MAX_PAGE).clamp(1, MAX_PAGE);
    Ok(Json(state.store.events_since(query.since, limit)?))
}

pub async fn whoami(
    State(state): State<AppState>,
    AuthDevice(device): AuthDevice,
) -> Result<Json<Whoami>, ServerError> {
    Ok(Json(Whoami {
        device: device.name,
        latest: state.store.latest_seq()?,
    }))
}
