//! 学习数据：服务器只存合并后的当前值（每个键一行），每次改动给这一行一个新的 `seq`。
//! 客户端拉 `seq` 之后变过的行，用「服务器值 − 本机基线」得到别的设备的增量，所以不需要保存操作历史，也不用清理。

use qingjian_cloud_proto::{LearningPage, LearningPush, LearningRow, MAX_LEARNING_PUSH};
use rusqlite::{OptionalExtension, Transaction, params};

use super::{Device, Store};
use crate::ServerError;

/// 学习数据序号在 `counters` 表里的名字。
const SEQ_COUNTER: &str = "learning";

/// 表名与键的长度上限（字节）。
const MAX_TABLE_BYTES: usize = 32;
const MAX_KEY_BYTES: usize = 1024;

impl Store {
    /// 应用一批变化，返回之后的最新 `seq`。整批在一个事务里，要么全成要么全不成。
    pub fn push_learning(&self, device: &Device, push: &LearningPush) -> Result<u64, ServerError> {
        if push.len() > MAX_LEARNING_PUSH {
            return Err(ServerError::BadRequest(format!(
                "at most {MAX_LEARNING_PUSH} changes per push"
            )));
        }
        let mut conn = self.conn();
        let tx = conn.transaction()?;
        let mut seq = current_seq(&tx)?;
        for change in &push.counts {
            check(&change.table, &change.key)?;
            seq += 1;
            tx.execute(
                "INSERT INTO learning (tbl, key, count, value, seq, device_id)
                 VALUES (?1, ?2, MAX(0, ?3), ?4, ?5, ?6)
                 ON CONFLICT (tbl, key) DO UPDATE SET
                    count = MAX(0, count + ?3),
                    value = COALESCE(value, ?4),
                    deleted = 0, seq = ?5, device_id = ?6",
                params![
                    change.table,
                    change.key,
                    change.delta,
                    change.value,
                    seq as i64,
                    device.id
                ],
            )?;
        }
        for put in &push.puts {
            check(&put.table, &put.key)?;
            seq += 1;
            tx.execute(
                "INSERT INTO learning (tbl, key, value, seq, device_id) VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT (tbl, key) DO UPDATE SET value = ?3, deleted = 0, seq = ?4, device_id = ?5",
                params![put.table, put.key, put.value, seq as i64, device.id],
            )?;
        }
        for delete in &push.deletes {
            check(&delete.table, &delete.key)?;
            seq += 1;
            tx.execute(
                "INSERT INTO learning (tbl, key, deleted, seq, device_id) VALUES (?1, ?2, 1, ?3, ?4)
                 ON CONFLICT (tbl, key) DO UPDATE SET value = NULL, deleted = 1, seq = ?3, device_id = ?4",
                params![delete.table, delete.key, seq as i64, device.id],
            )?;
        }
        tx.execute(
            "INSERT INTO counters (name, value) VALUES (?1, ?2)
             ON CONFLICT (name) DO UPDATE SET value = ?2",
            params![SEQ_COUNTER, seq as i64],
        )?;
        tx.commit()?;
        Ok(seq)
    }

    /// `since` 之后变过的行，最多 `limit` 条。
    pub fn learning_since(&self, since: u64, limit: usize) -> Result<LearningPage, ServerError> {
        let mut conn = self.conn();
        let tx = conn.transaction()?;
        let rows = {
            let mut statement = tx.prepare(
                "SELECT tbl, key, count, value, deleted, seq FROM learning
                 WHERE seq > ?1 ORDER BY seq LIMIT ?2",
            )?;
            statement
                .query_map(params![since as i64, limit as i64], |row| {
                    Ok(LearningRow {
                        table: row.get(0)?,
                        key: row.get(1)?,
                        count: row.get(2)?,
                        value: row.get(3)?,
                        deleted: row.get::<_, i64>(4)? != 0,
                        seq: row.get::<_, i64>(5)? as u64,
                    })
                })?
                .collect::<Result<Vec<_>, _>>()?
        };
        let latest = current_seq(&tx)?;
        tx.commit()?;
        Ok(LearningPage { rows, latest })
    }
}

fn current_seq(tx: &Transaction<'_>) -> Result<u64, ServerError> {
    let value: Option<i64> = tx
        .query_row(
            "SELECT value FROM counters WHERE name = ?1",
            [SEQ_COUNTER],
            |row| row.get(0),
        )
        .optional()?;
    Ok(value.unwrap_or(0) as u64)
}

fn check(table: &str, key: &str) -> Result<(), ServerError> {
    if table.is_empty()
        || table.len() > MAX_TABLE_BYTES
        || key.is_empty()
        || key.len() > MAX_KEY_BYTES
    {
        return Err(ServerError::BadRequest("bad table or key".to_owned()));
    }
    Ok(())
}
