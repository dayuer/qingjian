//! 设备的登记、列出、移除与令牌鉴权。

use rusqlite::{OptionalExtension, params};

use super::{Device, Store, generate_token, hash_token};
use crate::ServerError;

/// 设备名最长多少个字符。
const MAX_NAME_CHARS: usize = 64;

impl Store {
    /// 登记一台设备，返回明文令牌（只有这一次能看到）。
    pub fn add_device(&self, name: &str) -> Result<String, ServerError> {
        let name = name.trim();
        if name.is_empty()
            || name.chars().count() > MAX_NAME_CHARS
            || name.chars().any(char::is_control)
        {
            return Err(ServerError::BadRequest(format!(
                "device name must be 1-{MAX_NAME_CHARS} printable characters"
            )));
        }
        let token = generate_token()?;
        let inserted = self.conn().execute(
            "INSERT INTO devices (name, token_hash, created_at) VALUES (?1, ?2, ?3)
             ON CONFLICT (name) DO NOTHING",
            params![name, hash_token(&token), crate::now_ms()],
        )?;
        if inserted == 0 {
            return Err(ServerError::DeviceExists(name.to_owned()));
        }
        Ok(token)
    }

    pub fn list_devices(&self) -> Result<Vec<Device>, ServerError> {
        let conn = self.conn();
        let mut statement =
            conn.prepare("SELECT id, name, created_at, last_seen FROM devices ORDER BY id")?;
        let rows = statement.query_map([], |row| {
            Ok(Device {
                id: row.get(0)?,
                name: row.get(1)?,
                created_at: row.get(2)?,
                last_seen: row.get(3)?,
            })
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    /// 移除设备，它的令牌立即失效；它产生过的事件保留。
    pub fn remove_device(&self, name: &str) -> Result<(), ServerError> {
        let removed = self
            .conn()
            .execute("DELETE FROM devices WHERE name = ?1", params![name.trim()])?;
        if removed == 0 {
            return Err(ServerError::NotFound);
        }
        Ok(())
    }

    /// 按令牌找设备，顺带记下最近使用时间。
    pub fn authenticate(&self, token: &str) -> Result<Device, ServerError> {
        let conn = self.conn();
        let now = crate::now_ms();
        conn.query_row(
            "UPDATE devices SET last_seen = ?2 WHERE token_hash = ?1
             RETURNING id, name, created_at, last_seen",
            params![hash_token(token), now],
            |row| {
                Ok(Device {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    created_at: row.get(2)?,
                    last_seen: row.get(3)?,
                })
            },
        )
        .optional()?
        .ok_or(ServerError::Unauthorized)
    }
}
