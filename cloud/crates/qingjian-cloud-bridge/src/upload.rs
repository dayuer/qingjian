//! 素材（2B）与输入日志的后台上传：App 前台或键盘轮询时踢一脚，后台线程慢慢传，绝不阻塞按键。
//! 同意开关以 cloud.toml 的镜像为准（App 同意后经 [`crate::qj_consent_set`] 写回）；每轮重读配置，
//! 登录、退出、开关变化都自然生效。素材上传前先替换对象名（〔对象〕）再走规则脱敏，本机原文不动。

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::OnceLock;
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use qingjian_cloud_client::{Client, ClientError};
use qingjian_cloud_proto::{InputLogPush, MAX_MEMORY_ITEMS, MemoryItem, MemoryPush};

use crate::cloud_config::CloudConfig;
use crate::mask_contact_names;
use crate::memory::MemoryStore;
use crate::memory::now_unix;
use qingjian_cloud_redact::redact_rules;

/// 两轮上传之间的最短间隔：键盘每 0.25 秒轮询反复踢，也最多这么久跑一轮。
const MIN_INTERVAL: Duration = Duration::from_secs(30);

/// 一批输入日志最多多少行。
const INPUT_LOG_BATCH: usize = 200;

/// 已起的上传器，按 `user_dir` 登记（[`crate::qj_upload_kick`] 给 App 前台用）。
static UPLOADERS: OnceLock<Mutex<Vec<(PathBuf, Sender<()>)>>> = OnceLock::new();

fn registry() -> &'static Mutex<Vec<(PathBuf, Sender<()>)>> {
    UPLOADERS.get_or_init(|| Mutex::new(Vec::new()))
}

/// 后台上传器。持有它就活着，丢了线程随会话退出（先跑完当轮再退）。
pub struct Uploader {
    tx: Sender<()>,
    handle: Option<JoinHandle<()>>,
    user_dir: PathBuf,
}

impl Uploader {
    /// 起后台线程。没令牌也照起：每轮重读 cloud.toml，登录之后自然开始传。
    pub fn start(cloud_path: PathBuf, user_dir: PathBuf) -> Self {
        let (tx, rx) = channel::<()>();
        let run_path = cloud_path.clone();
        let run_dir = user_dir.clone();
        let handle = std::thread::Builder::new()
            .name("qj-upload".to_owned())
            .spawn(move || run(run_path, run_dir, rx))
            .expect("起上传线程");
        registry()
            .lock()
            .expect("上传器登记表")
            .retain(|(dir, _)| *dir != user_dir);
        registry()
            .lock()
            .expect("上传器登记表")
            .push((user_dir.clone(), tx.clone()));
        Self {
            tx,
            handle: Some(handle),
            user_dir,
        }
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
        let _ = self.tx.send(());
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
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

fn run(cloud_path: PathBuf, user_dir: PathBuf, rx: Receiver<()>) {
    let mut last: Option<Instant> = None;
    loop {
        match rx.recv_timeout(MIN_INTERVAL) {
            Ok(()) | Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }
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
    if config.memory {
        if let Err(error) = upload_materials(&client, user_dir) {
            tracing::debug!(%error, "素材上传这轮没走完");
        }
    }
    if config.logs {
        if let Err(error) = upload_input_log(&client, user_dir, config.user_id) {
            tracing::debug!(%error, "输入日志上传这轮没走完");
        }
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

/// 输入日志的进度：已经推到第几行。batch_id 按「用户:起始:结束」确定性生成，重发只记一次。
fn upload_input_log(
    client: &Client,
    user_dir: &std::path::Path,
    user_id: Option<i64>,
) -> Result<(), ClientError> {
    let log_path = user_dir.join("input-log.jsonl");
    let Ok(text) = std::fs::read_to_string(&log_path) else {
        return Ok(());
    };
    let lines: Vec<&str> = text.lines().collect();
    let progress_path = user_dir.join("cloud/input-log-progress");
    let mut done: usize = std::fs::read_to_string(&progress_path)
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(0);
    if done > lines.len() {
        // 本机日志被清空过（见 qj_input_log_clear）：进度归零从头对齐
        done = 0;
    }
    while done < lines.len() {
        let end = (done + INPUT_LOG_BATCH).min(lines.len());
        let batch_id = format!("u{}:{}:{}", user_id.unwrap_or(0), done, end);
        client.push_input_log(&InputLogPush {
            batch_id,
            lines: lines[done..end].iter().map(|s| (*s).to_owned()).collect(),
        })?;
        done = end;
        let _ = std::fs::write(&progress_path, done.to_string());
    }
    Ok(())
}

/// 清空云端输入记录之后：本机日志与进度一起清（[`crate::qj_input_log_clear`] 调）。
pub fn reset_input_log_progress(user_dir: &std::path::Path) {
    let _ = std::fs::remove_file(user_dir.join("input-log.jsonl"));
    let _ = std::fs::remove_file(user_dir.join("cloud/input-log-progress"));
}
