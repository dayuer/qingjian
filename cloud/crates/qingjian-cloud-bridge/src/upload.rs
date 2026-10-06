//! 素材（2B）与输入日志的后台上传：App 前台或键盘轮询时踢一脚，后台线程慢慢传，绝不阻塞按键。
//! 同意开关以 cloud.toml 的镜像为准（App 同意后经 [`crate::qj_consent_set`] 写回）；每轮重读配置，
//! 登录、退出、开关变化都自然生效。素材上传前先替换对象名（〔对象〕）再走规则脱敏，本机原文不动。

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel};
use std::time::{Duration, Instant};

use qingjian_cloud_client::{Client, ClientError, InputLogSync};
use qingjian_cloud_proto::{MAX_MEMORY_ITEMS, MemoryItem, MemoryPush};

use crate::cloud_config::CloudConfig;
use crate::mask_contact_names;
use crate::memory::MemoryStore;
use crate::memory::now_unix;
use qingjian_cloud_redact::redact_rules;

/// 两轮上传之间的最短间隔：键盘每 0.25 秒轮询反复踢，也最多这么久跑一轮。
const MIN_INTERVAL: Duration = Duration::from_secs(30);

/// `user_dir` → 那个上传器的踢脚通道。
type Kicker = (PathBuf, Sender<()>);

/// 已起的上传器，按 `user_dir` 登记（[`crate::qj_upload_kick`] 给 App 前台用）。
static UPLOADERS: OnceLock<Mutex<Vec<Kicker>>> = OnceLock::new();

fn registry() -> &'static Mutex<Vec<Kicker>> {
    UPLOADERS.get_or_init(|| Mutex::new(Vec::new()))
}

/// 后台上传器。持有它就活着；丢了置停止标志并唤醒，线程把手头这轮跑完自己退。
/// **不能 join**：登记表里给 App 前台留着 `Sender` 的克隆，channel 永不断连，
/// join 会等到天荒地老（测试里就是它挂死的）。
pub struct Uploader {
    tx: Sender<()>,
    stop: Arc<AtomicBool>,
    user_dir: PathBuf,
}

impl Uploader {
    /// 起后台线程。没令牌也照起：每轮重读 cloud.toml，登录之后自然开始传。
    pub fn start(cloud_path: PathBuf, user_dir: PathBuf) -> Self {
        let (tx, rx) = channel::<()>();
        let stop = Arc::new(AtomicBool::new(false));
        let run_path = cloud_path.clone();
        let run_dir = user_dir.clone();
        let thread_stop = Arc::clone(&stop);
        std::thread::Builder::new()
            .name("qj-upload".to_owned())
            .spawn(move || run(run_path, run_dir, rx, thread_stop))
            .expect("起上传线程");
        let mut guard = registry().lock().expect("上传器登记表");
        guard.retain(|(dir, _)| *dir != user_dir);
        guard.push((user_dir.clone(), tx.clone()));
        Self { tx, stop, user_dir }
    }

    /// 踢一脚：App 前台、键盘轮询时调，节流在线程里做。
    pub fn kick(&self) {
        let _ = self.tx.send(());
    }
}

impl Drop for Uploader {
    fn drop(&mut self) {
        registry()
            .lock()
            .expect("上传器登记表")
            .retain(|(dir, _)| *dir != self.user_dir);
        self.stop.store(true, Ordering::Release);
        let _ = self.tx.send(());
    }
}

/// 给 App 前台踢一脚：`user_dir` 上有没有活着的上传器都算成功。
pub fn kick_by_dir(user_dir: &std::path::Path) {
    let senders: Vec<Sender<()>> = registry()
        .lock()
        .expect("上传器登记表")
        .iter()
        .filter(|(dir, _)| dir == user_dir)
        .map(|(_, tx)| tx.clone())
        .collect();
    for tx in senders {
        let _ = tx.send(());
    }
}

fn run(cloud_path: PathBuf, user_dir: PathBuf, rx: Receiver<()>, stop: Arc<AtomicBool>) {
    let mut last: Option<Instant> = None;
    // 醒的间隔用短的（停止能在一秒内生效），节流另由 `last` 管
    while !matches!(
        rx.recv_timeout(Duration::from_secs(1)),
        Err(RecvTimeoutError::Disconnected)
    ) && !stop.load(Ordering::Acquire)
    {
        if last.is_some_and(|at| at.elapsed() < MIN_INTERVAL) {
            continue;
        }
        last = Some(Instant::now());
        cycle(&cloud_path, &user_dir);
    }
}

/// 一轮：重读配置分流。没登录、开关没开、静默失败下一轮再来。
fn cycle(cloud_path: &std::path::Path, user_dir: &std::path::Path) {
    let Some(config) = CloudConfig::load(cloud_path) else {
        return;
    };
    if config.token.is_empty() {
        return;
    }
    let client = Client::new(&config.server_or_default(), &config.token);
    if config.memory
        && let Err(error) = upload_materials(&client, user_dir)
    {
        tracing::debug!(%error, "素材上传这轮没走完");
    }
    if config.logs
        && let Err(error) = upload_input_log(&client, user_dir)
    {
        tracing::debug!(%error, "输入日志上传这轮没走完");
    }
}

/// 一个待传素材：`contact_id` 为 `None` 是无主桶的；上传成功后按它标 `uploaded`。
struct Pending {
    contact_id: Option<String>,
    item: MemoryItem,
}

/// 收集没上传的素材，上传前替换对象名并走规则脱敏。纯收集，不碰网络。
fn collect_pending(store: &MemoryStore, now: i64) -> Vec<Pending> {
    let mut out = Vec::new();
    let mut push = |contact: Option<&crate::memory::Contact>,
                    material: &crate::memory::Material| {
        if out.len() >= MAX_MEMORY_ITEMS {
            return;
        }
        let text = match contact {
            Some(contact) => redact_rules(&mask_contact_names(&material.text, contact)).0,
            None => redact_rules(&material.text).0,
        };
        out.push(Pending {
            contact_id: contact.map(|c| c.id.clone()),
            item: MemoryItem {
                client_id: material.client_id.clone(),
                contact_id: contact.map(|c| c.id.clone()),
                scene: None,
                kind: material.kind,
                text,
                at: material.at,
            },
        });
    };
    for contact in store.contacts() {
        let Ok(materials) = store.materials(&contact.id, now) else {
            continue;
        };
        for material in materials.iter().filter(|m| !m.uploaded) {
            push(Some(&contact), material);
        }
    }
    if let Ok(unassigned) = store.unassigned_materials(now) {
        for material in unassigned.iter().filter(|m| !m.uploaded) {
            push(None, material);
        }
    }
    out
}

fn upload_materials(client: &Client, user_dir: &std::path::Path) -> Result<(), ClientError> {
    let store = MemoryStore::open(user_dir);
    let now = now_unix();
    let pending = collect_pending(&store, now);
    if pending.is_empty() {
        return Ok(());
    }
    // 先登记这批涉及的对象（服务端幂等）；名额满的对象这轮不上传，素材留在本机
    let mut limit_full: HashSet<String> = HashSet::new();
    for contact_id in pending
        .iter()
        .filter_map(|p| p.contact_id.as_ref())
        .collect::<HashSet<_>>()
    {
        match client.register_contact(contact_id) {
            Ok(()) => {}
            Err(ClientError::ContactLimit) => {
                limit_full.insert(contact_id.clone());
            }
            Err(error) => return Err(error),
        }
    }
    let send: Vec<MemoryItem> = pending
        .iter()
        .filter(|p| {
            p.contact_id
                .as_ref()
                .is_none_or(|id| !limit_full.contains(id))
        })
        .map(|p| p.item.clone())
        .collect();
    if send.is_empty() {
        return Ok(());
    }
    let sent_ids: Vec<(Option<&str>, &str)> = pending
        .iter()
        .filter(|p| {
            p.contact_id
                .as_ref()
                .is_none_or(|id| !limit_full.contains(id))
        })
        .map(|p| (p.contact_id.as_deref(), p.item.client_id.as_str()))
        .collect();
    client.push_materials(&MemoryPush { items: send })?;
    // 服务端按 client_id 幂等：发出去的都算已收（accepted 只给条数）
    for contact_id in sent_ids
        .iter()
        .filter_map(|(c, _)| *c)
        .collect::<HashSet<_>>()
    {
        let ids: Vec<&str> = sent_ids
            .iter()
            .filter(|(c, _)| *c == Some(contact_id))
            .map(|(_, id)| *id)
            .collect();
        let _ = store.mark_materials_uploaded(contact_id, &ids, now);
    }
    let unassigned: Vec<&str> = sent_ids
        .iter()
        .filter(|(c, _)| c.is_none())
        .map(|(_, id)| *id)
        .collect();
    if !unassigned.is_empty() {
        let _ = store.mark_unassigned_uploaded(&unassigned, now);
    }
    Ok(())
}

/// 输入日志走 Mac 同款的 [`InputLogSync`]：字节偏移续传、状态落盘（`cloud/input-log-state.json`）、
/// 文件变短或开头变了认出「清空过」并从新的清空代数（generation）重传——批号带代数，清空后从 0
/// 重传不会撞旧批号。这里只上传不下载（`download_dir` 为 `None`），键盘不消费别的设备的日志。
fn upload_input_log(client: &Client, user_dir: &std::path::Path) -> Result<(), ClientError> {
    let mut sync = InputLogSync::open(client.clone(), user_dir, &user_dir.join("cloud"), None)?;
    sync.cycle()?;
    Ok(())
}

/// 清空云端输入记录之后：本机日志删掉、**状态文件留着**（[`crate::qj_input_log_clear`] 调）。
/// 状态里带着上次的偏移与服务器的清空代数：下一轮同步认出文件变短，先再清一次（拿新代数）再从 0
/// 重传，批号不会与清空前的撞上。
pub fn reset_input_log_progress(user_dir: &std::path::Path) {
    let _ = std::fs::remove_file(user_dir.join("input-log.jsonl"));
}
