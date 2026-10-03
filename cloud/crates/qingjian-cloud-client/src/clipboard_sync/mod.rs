//! 剪贴板同步：两个后台线程。上传线程把离线队列逐条发出去，监听线程挂着 SSE 收别的设备的条目。
//! 壳只做两件事：本机剪贴板变了调 [`ClipboardSync::copy`]，定时用 [`ClipboardSync::try_recv`] 取收到的条目写进本机剪贴板。
//! 所有网络失败都只改 [`Status`]、按退避重试，不打扰输入。

mod backoff;
mod incoming;
mod listener;
mod shared;
mod status;
mod sync_config;
mod uploader;

use std::sync::atomic::Ordering;
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};

use qingjian_cloud_proto::MAX_CLIP_BYTES;

use crate::{Client, ClientError, Outbox, SyncState};

pub use incoming::Incoming;
pub use status::Status;
pub use sync_config::SyncConfig;

use backoff::Backoff;
use shared::Shared;

/// 进度文件名，在 `state_dir` 下。
const STATE_FILE: &str = "state.json";

/// 离线队列文件名，在 `state_dir` 下。
const OUTBOX_FILE: &str = "outbox.jsonl";

pub struct ClipboardSync {
    shared: Arc<Shared>,

    incoming: Receiver<Incoming>,
}

impl ClipboardSync {
    /// 读进度与离线队列，起两个线程。
    pub fn start(config: SyncConfig) -> Result<Self, ClientError> {
        std::fs::create_dir_all(&config.state_dir)?;
        let state_path = config.state_dir.join(STATE_FILE);
        let mut state = SyncState::load(&state_path);
        let server = config.server.trim().trim_end_matches('/').to_owned();
        if state.server.as_deref() != Some(server.as_str()) {
            state = SyncState {
                server: Some(server.clone()),
                ..SyncState::default()
            };
        }
        let outbox = Outbox::open(config.state_dir.join(OUTBOX_FILE))?;
        let (sender, incoming) = mpsc::channel();
        let shared = Arc::new(Shared::new(
            Client::new(&server, &config.token),
            state,
            state_path,
            outbox,
            sender,
        ));
        uploader::spawn(shared.clone());
        listener::spawn(shared.clone());
        Ok(Self { shared, incoming })
    }

    /// 本机复制了一段文本：排进离线队列并叫醒上传线程。空文本与超过上限的直接忽略。
    pub fn copy(&self, text: String) {
        if text.is_empty() || text.len() > MAX_CLIP_BYTES {
            tracing::debug!(bytes = text.len(), "剪贴板为空或过大，不同步");
            return;
        }
        match self.shared.outbox().push(text) {
            Ok(_) => self.shared.wake_uploader(),
            Err(error) => tracing::warn!(%error, "离线队列写入失败"),
        }
    }

    /// 删掉一条记录（所有设备同步删除）。离线时删不掉，记一条警告。
    pub fn delete(&self, seq: u64) {
        let client = self.shared.client.clone();
        std::thread::spawn(move || {
            if let Err(error) = client.delete_clip(seq) {
                tracing::warn!(seq, %error, "删除剪贴板失败");
            }
        });
    }

    /// 取一条收到的事件，没有就返回 `None`，不阻塞。
    pub fn try_recv(&self) -> Option<Incoming> {
        self.incoming.try_recv().ok()
    }

    pub fn status(&self) -> Status {
        self.shared.status()
    }

    /// 离线队列里还有几条没发出去。
    pub fn pending(&self) -> usize {
        self.shared.outbox().len()
    }
}

impl Drop for ClipboardSync {
    /// 让两个线程在下一个检查点退出；阻塞在 SSE 读上的监听线程最迟一条连接的时限后退出。
    fn drop(&mut self) {
        self.shared.stop.store(true, Ordering::Relaxed);
        self.shared.wake_uploader();
    }
}

/// 锁中毒时照样拿来用：里面都是可以重建的进度数据。
fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}
