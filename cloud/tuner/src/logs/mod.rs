//! 汇总的输入日志：从服务器全部拉下来，按时间切成「训练」与「留出」两段，取出上屏与合成。

mod commit;
mod composition;

use qingjian_cloud_client::Client;
use qingjian_cloud_proto::MAX_PAGE;
use serde_json::Value;

use crate::error::TunerError;

pub use commit::Commit;
pub use composition::{Composition, compositions};

/// 一行日志及其来源设备。
#[derive(Debug, Clone)]
pub struct LogLine {
    pub device: String,

    pub raw: String,

    pub entry: Value,
}

/// 拉全部日志（按 `seq` 顺序）。
pub fn fetch(client: &Client) -> Result<Vec<LogLine>, TunerError> {
    let mut lines = Vec::new();
    let mut since = 0;
    loop {
        let page = client.input_log(since, MAX_PAGE)?;
        let Some(last) = page.lines.last() else {
            break;
        };
        since = last.seq;
        let full = page.lines.len() == MAX_PAGE;
        for line in page.lines {
            if let Ok(entry) = serde_json::from_str(&line.line) {
                lines.push(LogLine {
                    device: line.device,
                    raw: line.line,
                    entry,
                });
            }
        }
        if !full {
            break;
        }
    }
    Ok(lines)
}

/// 按上屏条数切：前 `train_ratio` 用来找问题，后面留出来回放把关。
pub fn split(lines: &[LogLine], train_ratio: f64) -> (&[LogLine], &[LogLine]) {
    let commits = lines.iter().filter(|l| is_commit(&l.entry)).count();
    let cut_commits = (commits as f64 * train_ratio).round() as usize;
    let mut seen = 0;
    for (index, line) in lines.iter().enumerate() {
        if is_commit(&line.entry) {
            if seen == cut_commits {
                return lines.split_at(index);
            }
            seen += 1;
        }
    }
    (lines, &[])
}

pub fn is_commit(entry: &Value) -> bool {
    entry.get("event").and_then(Value::as_str) == Some("commit")
}

/// 把一段日志写成 jsonl 文件给 CLI 回放；每台设备的段落之前补一条 session，CLI 按它切换方案。
pub fn write_jsonl(lines: &[LogLine], path: &std::path::Path) -> std::io::Result<usize> {
    let mut out = String::new();
    let mut commits = 0;
    for line in lines {
        out.push_str(&line.raw);
        out.push('\n');
        commits += usize::from(is_commit(&line.entry));
    }
    std::fs::write(path, out)?;
    Ok(commits)
}

/// 最近上屏的文字（每台设备按顺序拼），给话题补词当素材；最多 `chars` 个字符。
pub fn recent_text(lines: &[LogLine], chars: usize) -> String {
    let mut text = String::new();
    for line in lines {
        match line.entry.get("event").and_then(Value::as_str) {
            Some("commit" | "passthrough") => {
                text.push_str(
                    line.entry
                        .get("text")
                        .and_then(Value::as_str)
                        .unwrap_or_default(),
                );
            }
            Some("break") => text.push('\n'),
            _ => {}
        }
    }
    let skip = text.chars().count().saturating_sub(chars);
    text.chars().skip(skip).collect()
}
