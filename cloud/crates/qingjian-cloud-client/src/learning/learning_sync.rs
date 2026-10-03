//! 一轮学习数据同步：确认上次的收件箱已被合并 → 推本机的变化 → 拉别的设备的变化写收件箱。
//! 每一步做完都先把基线与进度落盘再往下走，任何一步断网或崩溃，下一轮都能接着做，不会重复计数。

use std::path::{Path, PathBuf};

use qingjian_cloud_proto::{LearningPush, MAX_LEARNING_PUSH, MAX_PAGE};

use super::{INBOX, LearningState, Snapshot};
use crate::{Client, ClientError};

/// 基线与进度在 `state_dir` 下的文件名。
const BASE_FILE: &str = "learning-base.json";
const STATE_FILE: &str = "learning-state.json";

/// 一轮做了什么。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LearningOutcome {
    /// 推出去的变化条数。
    pub pushed: usize,

    /// 写进收件箱的条数。
    pub delivered: usize,

    /// 收件箱还没被输入法合并（输入法没在用，或者没打分叉补丁）。
    pub waiting: bool,
}

pub struct LearningSync {
    client: Client,

    /// 输入法的数据目录（`~/Library/Application Support/Qingjian`）。
    ime_dir: PathBuf,

    base_path: PathBuf,

    state_path: PathBuf,

    base: Snapshot,

    state: LearningState,
}

impl LearningSync {
    pub fn open(client: Client, ime_dir: &Path, state_dir: &Path) -> Result<Self, ClientError> {
        std::fs::create_dir_all(state_dir)?;
        let base_path = state_dir.join(BASE_FILE);
        let state_path = state_dir.join(STATE_FILE);
        let state = std::fs::read(&state_path)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default();
        Ok(Self {
            client,
            ime_dir: ime_dir.to_owned(),
            base: Snapshot::load(&base_path)?,
            base_path,
            state_path,
            state,
        })
    }

    fn inbox_path(&self) -> PathBuf {
        self.ime_dir.join(INBOX)
    }

    pub fn cycle(&mut self) -> Result<LearningOutcome, ClientError> {
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
            self.client.push_learning(&chunk)?;
            // 推一批就记一批：断在中间时，已推的不会在下一轮重推
            self.base.apply_push(&chunk);
            self.save()?;
            outcome.pushed += chunk.len();
        }

        let mut rows = Vec::new();
        let mut cursor = self.state.cursor;
        loop {
            let page = self.client.learning(cursor, MAX_PAGE)?;
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
