//! 输入日志：各设备 `input-log.jsonl` 的原样行，汇总保存（不清理，用户要全部历史）。
//! 同一设备的同一批（`batch_id`）只收一次，客户端断网重发不会重复。

use qingjian_cloud_proto::{InputLogLine, InputLogPage, InputLogPush};
use rusqlite::{OptionalExtension, params};
use serde_json::Value;

use super::{Device, Store};
use crate::ServerError;

/// 清空次数在 `counters` 表里的名字；客户端看到它变了就丢掉下载过的副本。
const GENERATION: &str = "input_log_generation";

/// 一批最多多少行。
const MAX_LINES: usize = 5000;

/// 拼上文时最多往回看多少行。
const CONTEXT_SCAN: usize = 2000;

impl Store {
    /// 存一批行，返回最新的 `seq`。
    pub fn push_input_log(&self, device: &Device, push: &InputLogPush) -> Result<u64, ServerError> {
        if push.lines.len() > MAX_LINES || push.batch_id.is_empty() || push.batch_id.len() > 128 {
            return Err(ServerError::BadRequest("bad input log batch".to_owned()));
        }
        let mut conn = self.conn();
        let tx = conn.transaction()?;
        let fresh = tx.execute(
            "INSERT INTO input_log_batches (device_id, batch_id) VALUES (?1, ?2)
             ON CONFLICT DO NOTHING",
            params![device.id, push.batch_id],
        )? == 1;
        if fresh {
            let now = crate::now_ms();
            let mut insert =
                tx.prepare("INSERT INTO input_log (device_id, at, line) VALUES (?1, ?2, ?3)")?;
            for line in &push.lines {
                insert.execute(params![device.id, now, line])?;
            }
        }
        let latest = latest(&tx)?;
        tx.commit()?;
        Ok(latest)
    }

    pub fn input_log_since(&self, since: u64, limit: usize) -> Result<InputLogPage, ServerError> {
        let mut conn = self.conn();
        let tx = conn.transaction()?;
        let lines = {
            let mut statement = tx.prepare(
                "SELECT l.seq, COALESCE(d.name, 'device-' || l.device_id), l.line
                 FROM input_log l LEFT JOIN devices d ON d.id = l.device_id
                 WHERE l.seq > ?1 ORDER BY l.seq LIMIT ?2",
            )?;
            statement
                .query_map(params![since as i64, limit as i64], |row| {
                    Ok(InputLogLine {
                        seq: row.get::<_, i64>(0)? as u64,
                        device: row.get(1)?,
                        line: row.get(2)?,
                    })
                })?
                .collect::<Result<Vec<_>, _>>()?
        };
        let page = InputLogPage {
            lines,
            latest: latest(&tx)?,
            generation: generation(&tx)?,
        };
        tx.commit()?;
        Ok(page)
    }

    /// 清空所有设备的历史（某台设备上点了「清空输入日志」）。
    pub fn clear_input_log(&self) -> Result<u64, ServerError> {
        let mut conn = self.conn();
        let tx = conn.transaction()?;
        tx.execute("DELETE FROM input_log", [])?;
        tx.execute("DELETE FROM input_log_batches", [])?;
        let next = generation(&tx)? + 1;
        tx.execute(
            "INSERT INTO counters (name, value) VALUES (?1, ?2)
             ON CONFLICT (name) DO UPDATE SET value = ?2",
            params![GENERATION, next as i64],
        )?;
        tx.commit()?;
        Ok(next)
    }

    /// 各设备最近上屏的文字，按时间先后拼起来，最多 `chars` 个字符。
    /// 取 `commit` 的文字与 `passthrough` 的标点；`break`（换应用、失焦）记成换行。
    pub fn recent_text(&self, chars: usize) -> Result<String, ServerError> {
        if chars == 0 {
            return Ok(String::new());
        }
        let conn = self.conn();
        let mut statement =
            conn.prepare("SELECT line FROM input_log ORDER BY seq DESC LIMIT ?1")?;
        let lines = statement
            .query_map([CONTEXT_SCAN as i64], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        let mut pieces: Vec<String> = Vec::new();
        let mut total = 0;
        for line in lines {
            let Ok(entry) = serde_json::from_str::<Value>(&line) else {
                continue;
            };
            let piece = match entry.get("event").and_then(Value::as_str) {
                Some("commit" | "passthrough") => entry
                    .get("text")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_owned(),
                Some("break") => "\n".to_owned(),
                _ => continue,
            };
            total += piece.chars().count();
            pieces.push(piece);
            if total >= chars {
                break;
            }
        }
        pieces.reverse();
        let text: String = pieces.concat();
        let skip = text.chars().count().saturating_sub(chars);
        Ok(text
            .chars()
            .skip(skip)
            .collect::<String>()
            .trim()
            .to_owned())
    }
}

fn latest(tx: &rusqlite::Transaction<'_>) -> Result<u64, ServerError> {
    let seq: Option<i64> = tx
        .query_row(
            "SELECT seq FROM sqlite_sequence WHERE name = 'input_log'",
            [],
            |row| row.get(0),
        )
        .optional()?;
    Ok(seq.unwrap_or(0) as u64)
}

fn generation(tx: &rusqlite::Transaction<'_>) -> Result<u64, ServerError> {
    let value: Option<i64> = tx
        .query_row(
            "SELECT value FROM counters WHERE name = ?1",
            [GENERATION],
            |row| row.get(0),
        )
        .optional()?;
    Ok(value.unwrap_or(0) as u64)
}
