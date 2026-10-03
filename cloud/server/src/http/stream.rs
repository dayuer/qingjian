//! `GET /v1/events/stream?since=`：SSE。先订阅广播再读积压，按 `seq` 去重，积压与实时之间不漏也不重。
//! 连接落后太多（广播缓冲溢出）就结束流，客户端带上收到的最后一个 `seq` 重连补拉。

use std::convert::Infallible;
use std::time::Duration;

use axum::extract::{Query, State};
use axum::response::sse::{Event as SseEvent, KeepAlive, Sse};
use qingjian_cloud_proto::{Event, MAX_PAGE};
use serde::Deserialize;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::{Stream, StreamExt};

use super::{AppState, AuthDevice};
use crate::ServerError;

/// 空闲时多久发一次注释行，防止反向代理与 NAT 把连接当死掉。
const KEEP_ALIVE: Duration = Duration::from_secs(15);

#[derive(Deserialize)]
pub struct StreamQuery {
    #[serde(default)]
    since: u64,
}

pub async fn stream(
    State(state): State<AppState>,
    AuthDevice(device): AuthDevice,
    Query(query): Query<StreamQuery>,
) -> Result<Sse<impl Stream<Item = Result<SseEvent, Infallible>>>, ServerError> {
    let live = BroadcastStream::new(state.events.subscribe());
    let mut backlog = Vec::new();
    let mut cursor = query.since;
    loop {
        let page = state.store.events_since(cursor, MAX_PAGE)?;
        let full = page.events.len() == MAX_PAGE;
        if let Some(last) = page.events.last() {
            cursor = last.seq;
        }
        backlog.extend(page.events);
        if !full {
            break;
        }
    }
    tracing::info!(device = %device.name, since = query.since, backlog = backlog.len(), "SSE 连接");
    let live = live
        .map_while(Result::ok)
        .filter(move |event| event.seq > cursor);
    let events = tokio_stream::iter(backlog).chain(live).map(to_sse);
    Ok(Sse::new(events).keep_alive(KeepAlive::new().interval(KEEP_ALIVE)))
}

fn to_sse(event: Event) -> Result<SseEvent, Infallible> {
    let data = serde_json::to_string(&event).unwrap_or_default();
    Ok(SseEvent::default().id(event.seq.to_string()).data(data))
}
