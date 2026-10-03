//! 大模型用量：按设备、按天（UTC）记请求数、命中缓存数与 token。

use rusqlite::params;

use super::{Device, Store};
use crate::ServerError;

/// `qingjian-cloud usage` 打印的一行。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UsageRow {
    pub day: String,

    pub device: String,

    pub requests: i64,

    pub cached: i64,

    pub prompt_tokens: i64,

    pub completion_tokens: i64,
}

impl Store {
    pub fn record_usage(
        &self,
        device: &Device,
        cached: bool,
        prompt_tokens: i64,
        completion_tokens: i64,
    ) -> Result<(), ServerError> {
        self.conn().execute(
            "INSERT INTO usage (day, device_id, requests, cached, prompt_tokens, completion_tokens)
             VALUES (date('now'), ?1, 1, ?2, ?3, ?4)
             ON CONFLICT (day, device_id) DO UPDATE SET
                requests = requests + 1, cached = cached + ?2,
                prompt_tokens = prompt_tokens + ?3, completion_tokens = completion_tokens + ?4",
            params![
                device.id,
                i64::from(cached),
                prompt_tokens,
                completion_tokens
            ],
        )?;
        Ok(())
    }

    /// 最近 `days` 天，按日期、设备排序。
    pub fn usage(&self, days: u32) -> Result<Vec<UsageRow>, ServerError> {
        let conn = self.conn();
        let mut statement = conn.prepare(
            "SELECT u.day, COALESCE(d.name, 'device-' || u.device_id), u.requests, u.cached,
                    u.prompt_tokens, u.completion_tokens
             FROM usage u LEFT JOIN devices d ON d.id = u.device_id
             WHERE u.day >= date('now', ?1) ORDER BY u.day, 2",
        )?;
        let rows = statement.query_map([format!("-{days} days")], |row| {
            Ok(UsageRow {
                day: row.get(0)?,
                device: row.get(1)?,
                requests: row.get(2)?,
                cached: row.get(3)?,
                prompt_tokens: row.get(4)?,
                completion_tokens: row.get(5)?,
            })
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }
}
