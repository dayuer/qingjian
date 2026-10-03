//! `POST /v1/clipboard` 与 `DELETE /v1/clipboard/{seq}`。

use axum::Json;
use axum::extract::{Path, State};
use qingjian_cloud_proto::{Event, PushClip};

use super::{AppState, AuthDevice};
use crate::ServerError;

pub async fn push(
    State(state): State<AppState>,
    AuthDevice(device): AuthDevice,
    Json(clip): Json<PushClip>,
) -> Result<Json<Event>, ServerError> {
    let (event, created) = state.store.push_clip(&device, &clip)?;
    if created {
        tracing::info!(seq = event.seq, device = %device.name, bytes = clip.text.len(), "剪贴板");
        state.publish(event.clone());
    }
    Ok(Json(event))
}

pub async fn delete(
    State(state): State<AppState>,
    AuthDevice(device): AuthDevice,
    Path(seq): Path<u64>,
) -> Result<Json<Event>, ServerError> {
    let event = state.store.delete_clip(&device, seq)?;
    state.publish(event.clone());
    Ok(Json(event))
}
