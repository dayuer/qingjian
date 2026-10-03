//! 学习数据的一份快照：读输入法的文件得到「当前」，落盘的一份当「基线」。
//! 两份相减得到要推的变化；基线与服务器值相减得到收件箱。

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;

use qingjian_cloud_proto::{CountDelta, LearningPush, LearningRow, SetDelete, SetPut};
use serde::{Deserialize, Serialize};

use super::{CountEntry, Table};
use crate::ClientError;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Snapshot {
    counts: BTreeMap<(Table, String), CountEntry>,

    sets: BTreeMap<(Table, String), String>,
}

/// 基线文件的 JSON 形状（元组键不能直接当 JSON 对象的键）。
#[derive(Serialize, Deserialize)]
struct Stored {
    counts: Vec<(String, String, i64, Option<String>)>,

    sets: Vec<(String, String, String)>,
}

impl Snapshot {
    /// 读输入法数据目录下的各张表；文件不存在算空表，格式不对的行跳过（与输入法自己的读法一致）。
    pub fn read_dir(dir: &Path) -> Result<Self, ClientError> {
        let mut snapshot = Self::default();
        for table in Table::ALL {
            match std::fs::read_to_string(dir.join(table.file())) {
                Ok(text) => snapshot.parse_table(table, &text),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            }
        }
        Ok(snapshot)
    }

    pub fn parse_table(&mut self, table: Table, text: &str) {
        for line in text.lines().map(str::trim) {
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let fields: Vec<&str> = line.split('\t').collect();
            if table.is_set() {
                // user-words.tsv：词\t拼音\t词频，词频是固定值不同步
                if let [word, pinyin, ..] = fields.as_slice()
                    && !word.is_empty()
                    && !pinyin.trim().is_empty()
                {
                    self.sets
                        .insert((table, (*word).to_owned()), pinyin.trim().to_owned());
                }
                continue;
            }
            let Some((count, keys)) = fields.split_last() else {
                continue;
            };
            let Ok(count) = count.trim().parse::<i64>() else {
                continue;
            };
            if count <= 0
                || !table.key_columns().contains(&keys.len())
                || keys.iter().any(|k| k.is_empty())
            {
                continue;
            }
            let (key, display) = match table {
                Table::English => (keys[0].to_ascii_lowercase(), Some(keys[0].to_owned())),
                _ => (keys.join("\t"), None),
            };
            let entry = self.counts.entry((table, key)).or_default();
            entry.count += count;
            if entry.display.is_none() {
                entry.display = display;
            }
        }
    }

    pub fn load(path: &Path) -> Result<Self, ClientError> {
        let bytes = match std::fs::read(path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Self::default());
            }
            Err(error) => return Err(error.into()),
        };
        let stored: Stored = serde_json::from_slice(&bytes)
            .map_err(|e| ClientError::BadResponse(format!("基线文件坏了：{e}")))?;
        let mut snapshot = Self::default();
        for (table, key, count, display) in stored.counts {
            if let Some(table) = Table::from_name(&table) {
                snapshot
                    .counts
                    .insert((table, key), CountEntry { count, display });
            }
        }
        for (table, key, value) in stored.sets {
            if let Some(table) = Table::from_name(&table) {
                snapshot.sets.insert((table, key), value);
            }
        }
        Ok(snapshot)
    }

    /// 先写临时文件再改名。
    pub fn save(&self, path: &Path) -> Result<(), ClientError> {
        let stored = Stored {
            counts: self
                .counts
                .iter()
                .map(|((t, k), e)| (t.name().to_owned(), k.clone(), e.count, e.display.clone()))
                .collect(),
            sets: self
                .sets
                .iter()
                .map(|((t, k), v)| (t.name().to_owned(), k.clone(), v.clone()))
                .collect(),
        };
        let bytes =
            serde_json::to_vec(&stored).map_err(|e| ClientError::BadResponse(e.to_string()))?;
        let mut name = path.file_name().unwrap_or_default().to_os_string();
        name.push(".tmp");
        let temp = path.with_file_name(name);
        std::fs::write(&temp, bytes)?;
        std::fs::rename(&temp, path)?;
        Ok(())
    }

    pub fn is_empty(&self) -> bool {
        self.counts.is_empty() && self.sets.is_empty()
    }

    /// `self`（当前）相对 `base` 的变化。
    pub fn diff(&self, base: &Self) -> LearningPush {
        let mut push = LearningPush::default();
        for ((table, key), entry) in &self.counts {
            let before = base
                .counts
                .get(&(*table, key.clone()))
                .map_or(0, |e| e.count);
            if entry.count != before {
                push.counts.push(CountDelta {
                    table: table.name().to_owned(),
                    key: key.clone(),
                    delta: entry.count - before,
                    value: entry.display.clone(),
                });
            }
        }
        for ((table, key), entry) in &base.counts {
            if !self.counts.contains_key(&(*table, key.clone())) {
                push.counts.push(CountDelta {
                    table: table.name().to_owned(),
                    key: key.clone(),
                    delta: -entry.count,
                    value: None,
                });
            }
        }
        for ((table, key), value) in &self.sets {
            if base.sets.get(&(*table, key.clone())) != Some(value) {
                push.puts.push(SetPut {
                    table: table.name().to_owned(),
                    key: key.clone(),
                    value: value.clone(),
                });
            }
        }
        for (table, key) in base.sets.keys() {
            if !self.sets.contains_key(&(*table, key.clone())) {
                push.deletes.push(SetDelete {
                    table: table.name().to_owned(),
                    key: key.clone(),
                });
            }
        }
        push
    }

    /// 把推出去的变化并进基线。
    pub fn apply_push(&mut self, push: &LearningPush) {
        for change in &push.counts {
            if let Some(table) = Table::from_name(&change.table) {
                self.add(table, &change.key, change.delta, change.value.as_deref());
            }
        }
        for put in &push.puts {
            if let Some(table) = Table::from_name(&put.table) {
                self.sets
                    .insert((table, put.key.clone()), put.value.clone());
            }
        }
        for delete in &push.deletes {
            if let Some(table) = Table::from_name(&delete.table) {
                self.sets.remove(&(table, delete.key.clone()));
            }
        }
    }

    /// 服务器上变过的行与基线的差，写成输入法收件箱的格式；没有差别返回空串。
    pub fn inbox_for(&self, rows: &[LearningRow]) -> String {
        let mut out = String::new();
        for row in rows {
            let Some(table) = Table::from_name(&row.table) else {
                continue;
            };
            let key = (table, row.key.clone());
            if table.is_set() {
                let current = self.sets.get(&key);
                match (&row.value, row.deleted) {
                    (_, true) | (None, _) if current.is_some() => {
                        let _ = writeln!(out, "{}\tdel\t{}", table.name(), row.key);
                    }
                    (Some(value), false) if current != Some(value) => {
                        let _ = writeln!(out, "{}\tput\t{}\t{value}", table.name(), row.key);
                    }
                    _ => {}
                }
                continue;
            }
            let before = self.counts.get(&key).map_or(0, |e| e.count);
            let delta = row.count - before;
            if delta == 0 {
                continue;
            }
            let fields = match table {
                Table::English => row.value.clone().unwrap_or_else(|| row.key.clone()),
                _ => row.key.clone(),
            };
            let _ = writeln!(out, "{}\tadd\t{fields}\t{delta}", table.name());
        }
        out
    }

    /// 输入法合并了收件箱：把同样的增量并进基线。
    pub fn apply_inbox(&mut self, inbox: &str) {
        for line in inbox.lines() {
            let fields: Vec<&str> = line.split('\t').collect();
            let Some(table) = fields.first().and_then(|name| Table::from_name(name)) else {
                continue;
            };
            match fields.as_slice() {
                [_, "put", key, value] => {
                    self.sets
                        .insert((table, (*key).to_owned()), (*value).to_owned());
                }
                [_, "del", key] => {
                    self.sets.remove(&(table, (*key).to_owned()));
                }
                [_, "add", keys @ .., delta] if !keys.is_empty() => {
                    let Ok(delta) = delta.parse::<i64>() else {
                        continue;
                    };
                    let (key, display) = match table {
                        Table::English => (keys[0].to_ascii_lowercase(), Some(keys[0])),
                        _ => (keys.join("\t"), None),
                    };
                    self.add(table, &key, delta, display);
                }
                _ => {}
            }
        }
    }

    /// 计数加增量，减到 0 及以下就删（与输入法、服务器的做法一致）。
    fn add(&mut self, table: Table, key: &str, delta: i64, display: Option<&str>) {
        let entry = self.counts.entry((table, key.to_owned())).or_default();
        entry.count += delta;
        if entry.display.is_none() {
            entry.display = display.map(str::to_owned);
        }
        if entry.count <= 0 {
            self.counts.remove(&(table, key.to_owned()));
        }
    }
}
