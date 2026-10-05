//! 键盘拿不到 `memory/.lock`（200 毫秒超时）时先记在内存里的写入：「记一笔」（补写成素材）按顺序排队，切场景只留最新一次。
//! 下次 refresh、poll、flush 或下一次写入时重试；卡片与场景在内存里已经生效，只是磁盘上晚几步。
//! 笔记队列同时落在 `memory/pending-keyboard.jsonl`（一行一条），键盘扩展被系统杀掉也不丢，下次启动读回来接着补写；切场景不落盘。
//! 只有键盘这一个进程写这个文件，不需要 flock：每次队列变了就整份写临时文件再改名（读到的不会是半截），
//! 文件始终等于队列，上限自然一致；队列空了就删文件。`memory/` 不存在时不建、不报错。

mod note;

use std::collections::VecDeque;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use qingjian_cloud_proto::Scene;

use crate::cloud_config::write_atomic;

pub(in crate::session) use self::note::PendingNote;

/// 待写的「记一笔」最多几条，超出丢最旧的。
pub(in crate::session) const MAX_PENDING_NOTES: usize = 32;

/// 落盘文件在 `memory/` 下的名字。
pub(in crate::session) const PENDING_FILE: &str = "pending-keyboard.jsonl";

#[derive(Debug, Default)]
pub(in crate::session) struct PendingWrites {
    notes: VecDeque<PendingNote>,

    /// 笔记队列落盘的文件；没有记忆目录时为空（不落盘）。
    file: Option<PathBuf>,

    scope: Option<(Scene, Option<String>)>,
}

impl PendingWrites {
    /// 读回上次被杀时留下的笔记（坏行跳过并记日志，只留最后 [`MAX_PENDING_NOTES`] 条）。
    pub(in crate::session) fn open(memory_dir: &Path) -> Self {
        let file = memory_dir.join(PENDING_FILE);
        let mut notes = VecDeque::new();
        match std::fs::read_to_string(&file) {
            Ok(text) => {
                for line in text.lines().filter(|line| !line.trim().is_empty()) {
                    match serde_json::from_str::<PendingNote>(line) {
                        Ok(note) => notes.push_back(note),
                        // serde_json 的报错可能带上原话，只记种类
                        Err(error) => {
                            tracing::warn!(kind = ?error.classify(), "待写笔记里有一行坏了，跳过");
                        }
                    }
                }
                while notes.len() > MAX_PENDING_NOTES {
                    notes.pop_front();
                }
            }
            Err(error) if error.kind() == ErrorKind::NotFound => {}
            Err(error) => tracing::warn!(%error, "待写笔记文件读不了"),
        }
        Self {
            notes,
            file: Some(file),
            scope: None,
        }
    }

    /// 队列变了就整份落盘；空了删文件。目录不在（`memory/` 还没建）时不写也不报错。
    fn persist(&self) {
        let Some(file) = self.file.as_deref() else {
            return;
        };
        let result = if self.notes.is_empty() {
            std::fs::remove_file(file)
        } else {
            let mut text = String::new();
            for note in &self.notes {
                if let Ok(line) = serde_json::to_string(note) {
                    text.push_str(&line);
                    text.push('\n');
                }
            }
            write_atomic(file, text.as_bytes(), false)
        };
        match result {
            Err(error) if error.kind() != ErrorKind::NotFound => {
                tracing::warn!(%error, "待写笔记没落盘");
            }
            _ => {}
        }
    }

    pub(in crate::session) fn is_empty(&self) -> bool {
        self.notes.is_empty() && self.scope.is_none()
    }

    #[cfg(test)]
    pub(in crate::session) fn has_scope(&self) -> bool {
        self.scope.is_some()
    }

    pub(in crate::session) fn note_count(&self) -> usize {
        self.notes.len()
    }

    /// 排到队尾；满了丢最旧的（记日志）。
    pub(in crate::session) fn push_note(&mut self, note: PendingNote) {
        if self.notes.len() >= MAX_PENDING_NOTES {
            self.notes.pop_front();
            tracing::warn!("待写的记一笔太多，丢掉最旧的一条");
        }
        self.notes.push_back(note);
        self.persist();
    }

    pub(in crate::session) fn front_note(&self) -> Option<&PendingNote> {
        self.notes.front()
    }

    pub(in crate::session) fn pop_note(&mut self) -> Option<PendingNote> {
        let note = self.notes.pop_front();
        self.persist();
        note
    }

    /// 后一次覆盖前一次。
    pub(in crate::session) fn set_scope(&mut self, scene: Scene, contact: Option<String>) {
        self.scope = Some((scene, contact));
    }

    pub(in crate::session) fn take_scope(&mut self) -> Option<(Scene, Option<String>)> {
        self.scope.take()
    }

    /// 重试没成功时放回去；期间若有更新的一次（重入）就不覆盖它。
    pub(in crate::session) fn restore_scope(&mut self, scope: (Scene, Option<String>)) {
        self.scope.get_or_insert(scope);
    }
}
