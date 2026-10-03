//! 各请求共享的状态：存储与实时事件的广播。

use std::sync::Arc;

use qingjian_cloud_proto::Event;
use tokio::sync::broadcast;

use crate::Store;

/// 广播缓冲：慢的 SSE 连接落后超过这么多条就断开，客户端重连后按 `seq` 补拉。
const BROADCAST_CAPACITY: usize = 256;

#[derive(Clone)]
pub struct AppState {
    pub store: Arc<Store>,

    pub events: broadcast::Sender<Event>,
}

impl AppState {
    pub fn new(store: Arc<Store>) -> Self {
        let (events, _) = broadcast::channel(BROADCAST_CAPACITY);
        Self { store, events }
    }

    /// 新事件推给所有在线的 SSE 连接；没人在听也不算错。
    pub fn publish(&self, event: Event) {
        let _ = self.events.send(event);
    }
}
