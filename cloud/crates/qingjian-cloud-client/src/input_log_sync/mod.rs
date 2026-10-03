//! 输入日志同步：把输入法的 `input-log.jsonl` 从上次的位置接着传（只传完整的行），
//! 可选地把别的设备的日志拉下来存成 `<设备>.jsonl`（给 `qingjian-cli --replay`）。
//! 文件变短或开头变了，说明用户点了「清空输入日志」：服务器上所有设备的历史一并清空，各设备下载的副本随之作废。

mod input_log_state;

use std::fs::OpenOptions;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use qingjian_cloud_proto::{InputLogPush, MAX_PAGE};

use crate::{Client, ClientError};

pub use input_log_state::InputLogState;

/// 输入法的日志文件名。
const LOG_FILE: &str = "input-log.jsonl";

const STATE_FILE: &str = "input-log-state.json";

/// 记下文件开头多少字节，用来认出「清空后又写了新内容」。
const HEAD_BYTES: usize = 64;

/// 一轮最多读多少字节，大文件第一次上传分几轮走完。
const MAX_READ: u64 = 4 * 1024 * 1024;

/// 一批最多多少行 / 多少字节（服务器的请求体上限是 2 MB）。
const BATCH_LINES: usize = 2000;
const BATCH_BYTES: usize = 900 * 1024;

/// 一轮做了什么。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct InputLogOutcome {
    pub uploaded: usize,

    pub downloaded: usize,

    pub cleared: bool,
}

pub struct InputLogSync {
    client: Client,

    ime_dir: PathBuf,

    /// 别的设备的日志存哪；`None` 不下载。
    download_dir: Option<PathBuf>,

    state_path: PathBuf,

    state: InputLogState,
}

impl InputLogSync {
    pub fn open(
        client: Client,
        ime_dir: &Path,
        state_dir: &Path,
        download_dir: Option<PathBuf>,
    ) -> Result<Self, ClientError> {
        std::fs::create_dir_all(state_dir)?;
        let state_path = state_dir.join(STATE_FILE);
        let state = std::fs::read(&state_path)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default();
        Ok(Self {
            client,
            ime_dir: ime_dir.to_owned(),
            download_dir,
            state_path,
            state,
        })
    }

    pub fn cycle(&mut self) -> Result<InputLogOutcome, ClientError> {
        let mut outcome = self.upload()?;
        if self.download_dir.is_some() {
            outcome.downloaded = self.download()?;
        }
        Ok(outcome)
    }

    fn upload(&mut self) -> Result<InputLogOutcome, ClientError> {
        let mut outcome = InputLogOutcome::default();
        let path = self.ime_dir.join(LOG_FILE);
        let mut file = match std::fs::File::open(&path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(outcome),
            Err(error) => return Err(error.into()),
        };
        let len = file.metadata()?.len();
        let mut head = vec![0; HEAD_BYTES.min(len as usize)];
        file.read_exact(&mut head)?;
        let head = String::from_utf8_lossy(&head).into_owned();
        let cleared = self.state.offset > 0
            && (len < self.state.offset
                || !head.starts_with(&self.state.head) && !self.state.head.starts_with(&head));
        if cleared {
            self.state.generation = self.client.clear_input_log()?;
            self.state.offset = 0;
            self.state.cursor = 0;
            self.clear_downloads()?;
            self.save()?;
            outcome.cleared = true;
        }
        if len <= self.state.offset {
            return Ok(outcome);
        }
        file.seek(SeekFrom::Start(self.state.offset))?;
        let mut bytes = Vec::new();
        file.take(MAX_READ).read_to_end(&mut bytes)?;
        // 只传完整的行：最后一行可能还在写
        let Some(end) = bytes.iter().rposition(|b| *b == b'\n') else {
            return Ok(outcome);
        };
        bytes.truncate(end + 1);

        let mut start = self.state.offset;
        let mut batch: Vec<String> = Vec::new();
        let mut batch_bytes = 0;
        let mut consumed = 0u64;
        for raw in bytes.split_inclusive(|b| *b == b'\n') {
            consumed += raw.len() as u64;
            let line = String::from_utf8_lossy(raw).trim().to_owned();
            if !line.is_empty() {
                batch_bytes += line.len();
                batch.push(line);
            }
            if batch.len() >= BATCH_LINES || batch_bytes >= BATCH_BYTES {
                outcome.uploaded += self.send(start, &mut batch, consumed, &head)?;
                start = self.state.offset;
                consumed = 0;
                batch_bytes = 0;
            }
        }
        if consumed > 0 {
            outcome.uploaded += self.send(start, &mut batch, consumed, &head)?;
        }
        Ok(outcome)
    }

    /// 发一批；成功后偏移前进并落盘。批号是起始偏移，断网重发同一批服务器只收一次。
    fn send(
        &mut self,
        start: u64,
        batch: &mut Vec<String>,
        consumed: u64,
        head: &str,
    ) -> Result<usize, ClientError> {
        let lines = std::mem::take(batch);
        let count = lines.len();
        if count > 0 {
            self.client.push_input_log(&InputLogPush {
                batch_id: format!("{}:{start}", self.state.generation),
                lines,
            })?;
        }
        self.state.offset = start + consumed;
        self.state.head = head.to_owned();
        self.save()?;
        Ok(count)
    }

    fn download(&mut self) -> Result<usize, ClientError> {
        if self.state.device.is_none() {
            self.state.device = Some(self.client.whoami()?.device);
        }
        let mut downloaded = 0;
        loop {
            let page = self.client.input_log(self.state.cursor, MAX_PAGE)?;
            if page.generation != self.state.generation {
                // 别的设备清空过：之前下载的都作废，从头拉
                self.clear_downloads()?;
                self.state.generation = page.generation;
                self.state.cursor = 0;
                self.save()?;
                continue;
            }
            let Some(last) = page.lines.last() else {
                return Ok(downloaded);
            };
            let next = last.seq;
            for line in &page.lines {
                if Some(&line.device) == self.state.device.as_ref() {
                    continue;
                }
                self.append(&line.device, &line.line)?;
                downloaded += 1;
            }
            self.state.cursor = next;
            self.save()?;
            if page.lines.len() < MAX_PAGE {
                return Ok(downloaded);
            }
        }
    }

    fn append(&self, device: &str, line: &str) -> Result<(), ClientError> {
        let Some(dir) = &self.download_dir else {
            return Ok(());
        };
        std::fs::create_dir_all(dir)?;
        let name: String = device
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '-' || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(dir.join(format!("{name}.jsonl")))?;
        writeln!(file, "{line}")?;
        Ok(())
    }

    fn clear_downloads(&self) -> Result<(), ClientError> {
        let Some(dir) = &self.download_dir else {
            return Ok(());
        };
        let Ok(entries) = std::fs::read_dir(dir) else {
            return Ok(());
        };
        for entry in entries.flatten() {
            if entry.path().extension().is_some_and(|ext| ext == "jsonl") {
                std::fs::remove_file(entry.path())?;
            }
        }
        Ok(())
    }

    fn save(&self) -> Result<(), ClientError> {
        let bytes =
            serde_json::to_vec(&self.state).map_err(|e| ClientError::BadResponse(e.to_string()))?;
        let temp = self.state_path.with_extension("json.tmp");
        std::fs::write(&temp, bytes)?;
        std::fs::rename(&temp, &self.state_path)?;
        Ok(())
    }
}
