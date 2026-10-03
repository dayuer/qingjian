//! 同步的配置文件：整份一行，带版本号做乐观锁。

use qingjian_cloud_proto::{ConfigDoc, PutConfig};
use rusqlite::{OptionalExtension, params};

use super::{Device, Store};
use crate::ServerError;

/// 配置文件大小上限（字节）。
const MAX_CONFIG_BYTES: usize = 512 * 1024;

impl Store {
    pub fn config(&self) -> Result<Option<ConfigDoc>, ServerError> {
        Ok(self
            .conn()
            .query_row(
                "SELECT c.text, c.version, COALESCE(d.name, 'device-' || c.device_id), c.at
                 FROM config c LEFT JOIN devices d ON d.id = c.device_id WHERE c.id = 1",
                [],
                |row| {
                    Ok(ConfigDoc {
                        text: row.get(0)?,
                        version: row.get::<_, i64>(1)? as u64,
                        device: row.get(2)?,
                        at: row.get(3)?,
                    })
                },
            )
            .optional()?)
    }

    /// 写入新版本；服务器上的版本不是 `if_version` 时返回 [`ServerError::Conflict`]。
    pub fn put_config(&self, device: &Device, put: &PutConfig) -> Result<ConfigDoc, ServerError> {
        if put.text.len() > MAX_CONFIG_BYTES {
            return Err(ServerError::TooLarge(MAX_CONFIG_BYTES));
        }
        {
            let mut conn = self.conn();
            let tx = conn.transaction()?;
            let current: u64 = tx
                .query_row("SELECT version FROM config WHERE id = 1", [], |row| {
                    row.get::<_, i64>(0)
                })
                .optional()?
                .unwrap_or(0) as u64;
            if current != put.if_version {
                return Err(ServerError::Conflict(current));
            }
            tx.execute(
                "INSERT INTO config (id, text, version, device_id, at) VALUES (1, ?1, ?2, ?3, ?4)
                 ON CONFLICT (id) DO UPDATE SET text = ?1, version = ?2, device_id = ?3, at = ?4",
                params![put.text, (current + 1) as i64, device.id, crate::now_ms()],
            )?;
            tx.commit()?;
        }
        self.config()?.ok_or(ServerError::NotFound)
    }
}
