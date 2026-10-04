//! 两个线程与壳共用的状态。事件交付只走 [`Shared::deliver`] 一处，按 `seq` 去重：
//! SSE 与上传后的补拉可能拿到同一条。

use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::Sender;
use std::sync::{Condvar, Mutex, MutexGuard};
use std::time::Duration;

use qingjian_cloud_proto::{Event, MAX_PAGE};

use super::{Incoming, Status, lock};
use crate::{Client, ClientError, Outbox, SyncState};

pub struct Shared {
    pub client: Client,

    state: Mutex<SyncState>,

    state_path: PathBuf,

    outbox: Mutex<Outbox>,

    sender: Mutex<Sender<Incoming>>,

    status: Mutex<Status>,

    /// 叫醒上传线程：值为「有新活」。
    wake: (Mutex<bool>, Condvar),

    pub stop: AtomicBool,
}

impl Shared {
    pub fn new(
        client: Client,
        state: SyncState,
        state_path: PathBuf,
        outbox: Outbox,
        sender: Sender<Incoming>,
    ) -> Self {
        Self {
            client,
            state: Mutex::new(state),
            state_path,
            outbox: Mutex::new(outbox),
            sender: Mutex::new(sender),
            status: Mutex::new(Status::Connecting),
            wake: (Mutex::new(false), Condvar::new()),
            stop: AtomicBool::new(false),
        }
    }

    pub fn stopped(&self) -> bool {
        self.stop.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn outbox(&self) -> MutexGuard<'_, Outbox> {
        lock(&self.outbox)
    }

    pub fn status(&self) -> Status {
        lock(&self.status).clone()
    }

    pub fn set_status(&self, status: Status) {
        let mut current = lock(&self.status);
        if *current != status {
            tracing::info!(?status, "Cloud 连接状态");
            *current = status;
        }
    }

    /// 按错误种类改状态。
    pub fn set_error(&self, error: &ClientError) {
        self.set_status(Status::from_error(error));
    }

    pub fn cursor(&self) -> u64 {
        lock(&self.state).cursor
    }

    pub fn device_known(&self) -> bool {
        lock(&self.state).device.is_some()
    }

    pub fn set_device(&self, device: String) {
        let mut state = lock(&self.state);
        state.device = Some(device);
        self.save(&state);
    }

    /// 交给壳。已经交付过的 `seq` 跳过。
    pub fn deliver(&self, event: Event) {
        let mut state = lock(&self.state);
        if event.seq <= state.cursor {
            return;
        }
        state.cursor = event.seq;
        self.save(&state);
        let mine = state.device.as_deref() == Some(event.device.as_str());
        drop(state);
        let _ = lock(&self.sender).send(Incoming { event, mine });
    }

    /// 用普通请求把 cursor 之后的事件拉完；SSE 断着的时候也能及时拿到别的设备的条目。
    pub fn catch_up(&self) -> Result<(), ClientError> {
        loop {
            let page = self.client.events(self.cursor(), MAX_PAGE)?;
            let full = page.events.len() == MAX_PAGE;
            for event in page.events {
                self.deliver(event);
            }
            if !full {
                return Ok(());
            }
        }
    }

    pub fn wake_uploader(&self) {
        let (flag, condvar) = &self.wake;
        *lock(flag) = true;
        condvar.notify_all();
    }

    /// 上传线程等活：被叫醒或超时返回。
    pub fn wait_for_work(&self, timeout: Duration) {
        let (flag, condvar) = &self.wake;
        let mut pending = lock(flag);
        if !*pending {
            pending = condvar
                .wait_timeout(pending, timeout)
                .map(|(guard, _)| guard)
                .unwrap_or_else(|poisoned| poisoned.into_inner().0);
        }
        *pending = false;
    }

    fn save(&self, state: &SyncState) {
        if let Err(error) = state.save(&self.state_path) {
            tracing::warn!(%error, "同步进度写入失败");
        }
    }
}
