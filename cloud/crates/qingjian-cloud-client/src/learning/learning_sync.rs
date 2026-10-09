//! 一轮学习数据同步：确认上次的收件箱已被合并 → 推本机的变化 → 拉别的设备的变化写收件箱。
//! 每一步做完都先把基线与进度落盘再往下走，任何一步断网或崩溃，下一轮都能接着做，不会重复计数。
//! 同一个 `state_dir` 可能同时有好几个实例（iOS 上每个键盘扩展进程、每次重建引擎都起一个 `DataSync`）：
//! 每轮先拿 `learning.lock` 的排他锁、从磁盘重读基线与进度，拿不到锁就跳过这一轮。
//! 只在内存里留基线的话，各实例会把同一份本机变化各推一遍，对方推的又经收件箱回来再被当成本机变化推出去，计数无限涨。

use std::fs::{File, OpenOptions, TryLockError};
use std::path::{Path, PathBuf};

use qingjian_cloud_proto::{LearningPush, MAX_LEARNING_PUSH, MAX_PAGE};

use super::{INBOX, LearningRemote, LearningState, Snapshot};
use crate::{Client, ClientError};

/// 基线与进度在 `state_dir` 下的文件名。
pub(crate) const BASE_FILE: &str = "learning-base.json";
pub(crate) const STATE_FILE: &str = "learning-state.json";

/// 一轮同步期间持排他锁的文件，与基线同目录。
const LOCK_FILE: &str = "learning.lock";

/// 一轮做了什么。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LearningOutcome {
    /// 推出去的变化条数。
    pub pushed: usize,

    /// 写进收件箱的条数。
    pub delivered: usize,

    /// 收件箱还没被输入法合并（输入法没在用，或者没打分叉补丁）。
    pub waiting: bool,

    /// 同一份基线的另一个实例正在同步，这一轮跳过了。
    pub busy: bool,
}

pub struct LearningSync {
    remote: Box<dyn LearningRemote>,

    /// 输入法的数据目录（`~/Library/Application Support/Qingjian`）。
    ime_dir: PathBuf,

    base_path: PathBuf,

    state_path: PathBuf,

    lock_path: PathBuf,

    base: Snapshot,

    state: LearningState,
}

impl LearningSync {
    pub fn open(client: Client, ime_dir: &Path, state_dir: &Path) -> Result<Self, ClientError> {
        Self::open_with(Box::new(client), ime_dir, state_dir)
    }

    pub(crate) fn open_with(
        remote: Box<dyn LearningRemote>,
        ime_dir: &Path,
        state_dir: &Path,
    ) -> Result<Self, ClientError> {
        std::fs::create_dir_all(state_dir)?;
        let base_path = state_dir.join(BASE_FILE);
        let state_path = state_dir.join(STATE_FILE);
        Ok(Self {
            remote,
            ime_dir: ime_dir.to_owned(),
            base: Snapshot::load(&base_path)?,
            state: load_state(&state_path),
            base_path,
            state_path,
            lock_path: state_dir.join(LOCK_FILE),
        })
    }

    fn inbox_path(&self) -> PathBuf {
        self.ime_dir.join(INBOX)
    }

    /// 锁只在这一轮里拿着（最多几次带超时的请求），进程退出时系统会放掉。
    pub fn cycle(&mut self) -> Result<LearningOutcome, ClientError> {
        let lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(&self.lock_path)?;
        match lock.try_lock() {
            Ok(()) => {}
            Err(TryLockError::WouldBlock) => {
                tracing::debug!("另一个实例正在同步学习数据，跳过这一轮");
                return Ok(LearningOutcome {
                    busy: true,
                    ..LearningOutcome::default()
                });
            }
            Err(TryLockError::Error(error)) => return Err(error.into()),
        }
        self.base = Snapshot::load(&self.base_path)?;
        self.state = load_state(&self.state_path);
        let outcome = self.cycle_locked();
        drop::<File>(lock);
        outcome
    }

    fn cycle_locked(&mut self) -> Result<LearningOutcome, ClientError> {
        let mut outcome = LearningOutcome::default();
        if let Some(inbox) = self.state.pending.clone() {
            if self.inbox_path().exists() {
                outcome.waiting = true;
                return Ok(outcome);
            }
            // 输入法已合并：它加上的正是这些增量，基线同样加上
            self.base.apply_inbox(&inbox);
            self.state.cursor = self.state.pending_cursor;
            self.state.pending = None;
            self.save()?;
        }

        let current = Snapshot::read_dir(&self.ime_dir)?;
        let changes = current.diff(&self.base);
        for chunk in chunks(changes) {
            self.remote.push_learning(&chunk)?;
            // 推一批就记一批：断在中间时，已推的不会在下一轮重推
            self.base.apply_push(&chunk);
            self.save()?;
            outcome.pushed += chunk.len();
        }

        let mut rows = Vec::new();
        let mut cursor = self.state.cursor;
        loop {
            let page = self.remote.learning(cursor, MAX_PAGE)?;
            let full = page.rows.len() == MAX_PAGE;
            cursor = match page.rows.last() {
                Some(last) if full => last.seq,
                _ => page.latest.max(cursor),
            };
            rows.extend(page.rows);
            if !full {
                break;
            }
        }
        let inbox = self.base.inbox_for(&rows);
        if inbox.is_empty() {
            self.state.cursor = cursor;
            self.save()?;
            return Ok(outcome);
        }
        outcome.delivered = inbox.lines().count();
        // 先记下「等合并」，再写收件箱：写完就崩溃时，下一轮知道它还没并进基线
        self.state.pending = Some(inbox.clone());
        self.state.pending_cursor = cursor;
        self.save()?;
        write_atomic(&self.inbox_path(), &inbox)?;
        outcome.waiting = true;
        Ok(outcome)
    }

    fn save(&self) -> Result<(), ClientError> {
        self.base.save(&self.base_path)?;
        let bytes =
            serde_json::to_vec(&self.state).map_err(|e| ClientError::BadResponse(e.to_string()))?;
        write_atomic(&self.state_path, &String::from_utf8_lossy(&bytes))
    }
}

/// 进度文件读不了或坏了从头来（游标 0，没有待合并的收件箱）。
fn load_state(path: &Path) -> LearningState {
    std::fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

fn chunks(push: LearningPush) -> Vec<LearningPush> {
    let mut out = Vec::new();
    let mut current = LearningPush::default();
    let flush = |current: &mut LearningPush, out: &mut Vec<LearningPush>| {
        if current.len() >= MAX_LEARNING_PUSH {
            out.push(std::mem::take(current));
        }
    };
    for change in push.counts {
        current.counts.push(change);
        flush(&mut current, &mut out);
    }
    for put in push.puts {
        current.puts.push(put);
        flush(&mut current, &mut out);
    }
    for delete in push.deletes {
        current.deletes.push(delete);
        flush(&mut current, &mut out);
    }
    if !current.is_empty() {
        out.push(current);
    }
    out
}

fn write_atomic(path: &Path, text: &str) -> Result<(), ClientError> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(".tmp");
    let temp = path.with_file_name(name);
    std::fs::write(&temp, text)?;
    std::fs::rename(&temp, path)?;
    Ok(())
}
