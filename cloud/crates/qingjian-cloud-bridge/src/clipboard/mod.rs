//! iOS 键盘的跨设备剪贴板：键盘弹出时后台拉服务器上的剪贴板事件，别的设备最近复制的一条给候选栏；
//! 用户点了「发到其他设备」才上传本机剪贴板（读 iOS 剪贴板可能弹系统授权提示，不在后台偷读）。
//! 键盘只在弹出时运行，所以不挂 SSE 长连接，弹出一次拉一次。

mod offer;
mod state;

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use qingjian_cloud_client::Client;
use qingjian_cloud_proto::{EventKind, MAX_PAGE, PushClip};

pub use self::offer::ClipOffer;
use self::state::ClipState;

/// 别的设备复制后多久内还给提示：验证码、链接这类都是马上要用的，旧的不打扰。
const FRESH_MS: i64 = 10 * 60 * 1000;

/// 第一次拉（没有进度）时只看最近这么多条，不翻完整个历史。
const FIRST_FETCH: u64 = 50;

pub struct Clipboard {
    client: Client,

    state_path: PathBuf,

    shared: Arc<Shared>,
}

struct Shared {
    state: Mutex<ClipState>,

    offer: Mutex<Option<ClipOffer>>,

    /// 这台设备在服务器上的名字（whoami），用来跳过自己复制的。
    me: Mutex<Option<String>>,

    /// 正在拉，避免键盘反复弹出时并发拉。
    fetching: AtomicBool,
}

impl Clipboard {
    pub fn new(client: Client, state_path: PathBuf) -> Self {
        if let Some(dir) = state_path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let state = ClipState::load(&state_path);
        Self {
            client,
            state_path,
            shared: Arc::new(Shared {
                state: Mutex::new(state),
                offer: Mutex::new(None),
                me: Mutex::new(None),
                fetching: AtomicBool::new(false),
            }),
        }
    }

    /// 后台拉一次新事件（键盘弹出时调）；已经在拉就不重复。
    pub fn refresh(&self) {
        if self.shared.fetching.swap(true, Ordering::SeqCst) {
            return;
        }
        let client = self.client.clone();
        let shared = self.shared.clone();
        let state_path = self.state_path.clone();
        let spawned = std::thread::Builder::new()
            .name("cloud-clipboard".to_owned())
            .spawn(move || {
                if let Err(error) = fetch(&client, &shared, &state_path) {
                    tracing::warn!(%error, "剪贴板拉取失败");
                }
                shared.fetching.store(false, Ordering::SeqCst);
            });
        if spawned.is_err() {
            self.shared.fetching.store(false, Ordering::SeqCst);
        }
    }

    /// 还新鲜、没处理过的提示。
    pub fn offer(&self) -> Option<ClipOffer> {
        lock(&self.shared.offer)
            .clone()
            .filter(|offer| now_ms() - offer.at <= FRESH_MS)
    }

    /// 用户插入或关掉了这条提示：记下，不再给。
    pub fn handled(&self) {
        let Some(offer) = lock(&self.shared.offer).take() else {
            return;
        };
        let mut state = lock(&self.shared.state);
        state.handled = state.handled.max(offer.seq);
        state.save(&self.state_path);
    }

    /// 把本机剪贴板里的文字发给别的设备（用户点了按钮才调），后台发，失败只记日志。
    pub fn push(&self, text: &str) {
        let text = text.trim().to_owned();
        if text.is_empty() {
            return;
        }
        let client = self.client.clone();
        let clip = PushClip {
            client_id: uuid::Uuid::new_v4().to_string(),
            text,
        };
        let spawned = std::thread::Builder::new()
            .name("cloud-clipboard-push".to_owned())
            .spawn(move || {
                if let Err(error) = client.push_clip(&clip) {
                    tracing::warn!(%error, "剪贴板上传失败");
                }
            });
        if let Err(error) = spawned {
            tracing::warn!(%error, "剪贴板上传线程起不来");
        }
    }
}

/// 拉 `seen` 之后的事件：别的设备最新的一条 `ClipAdded`（够新、没给过）成为提示，被删掉的撤回。
fn fetch(
    client: &Client,
    shared: &Shared,
    state_path: &std::path::Path,
) -> Result<(), qingjian_cloud_client::ClientError> {
    // 先取出来再 match：match 的判别式里的锁守卫会活到整个 match 结束，None 分支再拿同一把锁就死锁
    let cached = lock(&shared.me).clone();
    let me = match cached {
        Some(me) => me,
        None => {
            let me = client.whoami()?.device;
            *lock(&shared.me) = Some(me.clone());
            me
        }
    };
    let mut since = lock(&shared.state).seen;
    if since == 0 {
        // 没有进度：从最近的几十条开始，不翻完整个历史
        let latest = client.events(0, 1)?.latest;
        since = latest.saturating_sub(FIRST_FETCH);
    }
    let now = now_ms();
    let handled = lock(&shared.state).handled;
    let mut candidate = lock(&shared.offer).clone();
    loop {
        let page = client.events(since, MAX_PAGE)?;
        let full = page.events.len() == MAX_PAGE;
        for event in page.events {
            since = since.max(event.seq);
            match event.kind {
                EventKind::ClipAdded { text, .. }
                    if event.device != me
                        && event.seq > handled
                        && now - event.at <= FRESH_MS
                        && !text.trim().is_empty() =>
                {
                    candidate = Some(ClipOffer {
                        seq: event.seq,
                        device: event.device,
                        text,
                        at: event.at,
                    });
                }
                EventKind::ClipDeleted { target }
                    if candidate.as_ref().is_some_and(|c| c.seq == target) =>
                {
                    candidate = None;
                }
                _ => {}
            }
        }
        if !full {
            break;
        }
    }
    // 拉的这段时间里用户可能已经处理过当前提示
    let handled = lock(&shared.state).handled;
    *lock(&shared.offer) = candidate.filter(|c| c.seq > handled);
    let mut state = lock(&shared.state);
    state.seen = state.seen.max(since);
    state.save(state_path);
    Ok(())
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}
