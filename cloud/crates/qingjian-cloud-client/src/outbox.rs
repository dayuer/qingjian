//! 离线队列：待上传的剪贴板，一行一条 JSON 落盘，连不上服务器时攒着，重启也不丢。
//! 每条带客户端生成的 `client_id`，补发时服务端据此去重，「发出去了但没收到回应」重发也只记一次。

use std::collections::VecDeque;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use qingjian_cloud_proto::PushClip;

use crate::ClientError;

/// 最多攒多少条，超出丢最旧的：离线一整天复制的东西没必要全部补发。
const MAX_ITEMS: usize = 100;

pub struct Outbox {
    path: PathBuf,

    items: VecDeque<PushClip>,
}

impl Outbox {
    /// 读已有队列；文件不存在就是空队列，坏行跳过。
    pub fn open(path: impl Into<PathBuf>) -> Result<Self, ClientError> {
        let path = path.into();
        let items = match std::fs::read_to_string(&path) {
            Ok(text) => text
                .lines()
                .filter_map(|line| serde_json::from_str(line).ok())
                .collect(),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => VecDeque::new(),
            Err(error) => return Err(error.into()),
        };
        Ok(Self { path, items })
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn front(&self) -> Option<&PushClip> {
        self.items.front()
    }

    /// 排进一条新文本，返回它的 `client_id`。
    pub fn push(&mut self, text: String) -> Result<String, ClientError> {
        let clip = PushClip {
            client_id: uuid::Uuid::new_v4().to_string(),
            text,
        };
        let client_id = clip.client_id.clone();
        self.items.push_back(clip);
        if self.items.len() > MAX_ITEMS {
            self.items.pop_front();
            self.rewrite()?;
        } else {
            let mut file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.path)?;
            writeln!(file, "{}", line(self.items.back().expect("just pushed")))?;
            file.sync_all()?;
        }
        Ok(client_id)
    }

    /// 队首上传成功后去掉它。
    pub fn pop_front(&mut self) -> Result<(), ClientError> {
        if self.items.pop_front().is_some() {
            self.rewrite()?;
        }
        Ok(())
    }

    /// 清空队列（服务器上关了剪贴板，攒着的明文不再补传）。
    pub fn clear(&mut self) -> Result<(), ClientError> {
        if !self.items.is_empty() {
            self.items.clear();
            self.rewrite()?;
        }
        Ok(())
    }

    /// 整个重写：先写临时文件再改名，崩溃时磁盘上要么是旧队列要么是新队列。
    fn rewrite(&self) -> Result<(), ClientError> {
        let temp = temp_path(&self.path);
        {
            let mut file = File::create(&temp)?;
            for clip in &self.items {
                writeln!(file, "{}", line(clip))?;
            }
            file.sync_all()?;
        }
        std::fs::rename(&temp, &self.path)?;
        Ok(())
    }
}

fn line(clip: &PushClip) -> String {
    serde_json::to_string(clip).unwrap_or_default()
}

fn temp_path(path: &Path) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(".tmp");
    path.with_file_name(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn survives_reopen_and_caps_size() {
        let dir = std::env::temp_dir().join(format!("qjc-outbox-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("outbox.jsonl");
        let mut outbox = Outbox::open(&path).unwrap();
        for i in 0..MAX_ITEMS + 2 {
            outbox.push(format!("t{i}")).unwrap();
        }
        outbox.pop_front().unwrap();
        let reopened = Outbox::open(&path).unwrap();
        assert_eq!(reopened.len(), MAX_ITEMS - 1);
        assert_eq!(reopened.front().unwrap().text, "t3");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn clear_empties_queue_on_disk() {
        let dir = std::env::temp_dir().join(format!("qjc-outbox-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("outbox.jsonl");
        let mut outbox = Outbox::open(&path).unwrap();
        outbox.push("a".to_owned()).unwrap();
        outbox.push("b".to_owned()).unwrap();
        outbox.clear().unwrap();
        assert!(outbox.is_empty());
        assert!(Outbox::open(&path).unwrap().is_empty());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
