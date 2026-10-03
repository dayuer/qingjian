//! SQLite 存储：设备表与事件表。单连接加互斥锁；一人一台服务器，每次查询都是毫秒以内，不值得上连接池。

mod config;
mod device;
mod devices;
mod events;
mod learning;
mod token;

use std::path::Path;
use std::sync::{Mutex, MutexGuard};

use rusqlite::Connection;

use crate::ServerError;

pub use device::Device;
pub use token::{generate_token, hash_token};

/// 数据库结构版本，存在 `PRAGMA user_version`。
const SCHEMA_VERSION: i64 = 2;

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS devices (
    id         INTEGER PRIMARY KEY,
    name       TEXT    NOT NULL UNIQUE,
    token_hash TEXT    NOT NULL UNIQUE,
    created_at INTEGER NOT NULL,
    last_seen  INTEGER
);
CREATE TABLE IF NOT EXISTS events (
    seq       INTEGER PRIMARY KEY AUTOINCREMENT,
    device_id INTEGER NOT NULL,
    at        INTEGER NOT NULL,
    kind      TEXT    NOT NULL,
    client_id TEXT,
    target    INTEGER,
    text      TEXT,
    UNIQUE (device_id, client_id)
);
CREATE INDEX IF NOT EXISTS events_kind_at ON events (kind, at);
CREATE TABLE IF NOT EXISTS learning (
    tbl       TEXT    NOT NULL,
    key       TEXT    NOT NULL,
    count     INTEGER NOT NULL DEFAULT 0,
    value     TEXT,
    deleted   INTEGER NOT NULL DEFAULT 0,
    seq       INTEGER NOT NULL,
    device_id INTEGER NOT NULL,
    PRIMARY KEY (tbl, key)
);
CREATE INDEX IF NOT EXISTS learning_seq ON learning (seq);
CREATE TABLE IF NOT EXISTS counters (
    name  TEXT    PRIMARY KEY,
    value INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS config (
    id        INTEGER PRIMARY KEY CHECK (id = 1),
    text      TEXT    NOT NULL,
    version   INTEGER NOT NULL,
    device_id INTEGER NOT NULL,
    at        INTEGER NOT NULL
);
";

pub struct Store {
    conn: Mutex<Connection>,
}

impl Store {
    /// 打开（不存在就建）数据库文件。
    pub fn open(path: &Path) -> Result<Self, ServerError> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        Self::init(Connection::open(path)?)
    }

    /// 内存库，测试用。
    pub fn in_memory() -> Result<Self, ServerError> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(conn: Connection) -> Result<Self, ServerError> {
        // WAL 让备份时拷文件、读写并发都更安全；busy_timeout 防止 CLI 与服务同时写时直接报错
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "busy_timeout", 5000)?;
        conn.execute_batch(SCHEMA)?;
        let version: i64 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;
        if version > SCHEMA_VERSION {
            return Err(ServerError::BadRequest(format!(
                "database schema {version} is newer than this server ({SCHEMA_VERSION})"
            )));
        }
        conn.pragma_update(None, "user_version", SCHEMA_VERSION)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// 一致快照：`VACUUM INTO` 在一个读事务里写出完整的新库，不挡正在跑的服务。
    pub fn backup_to(&self, path: &Path) -> Result<(), ServerError> {
        if path.exists() {
            return Err(ServerError::BadRequest(format!(
                "{} already exists",
                path.display()
            )));
        }
        let target = path
            .to_str()
            .ok_or_else(|| ServerError::BadRequest("non UTF-8 path".to_owned()))?;
        self.conn().execute("VACUUM INTO ?1", [target])?;
        Ok(())
    }

    /// 拿连接；锁中毒（某次查询 panic）时照样拿来用，SQLite 自己保证事务完整。
    pub(super) fn conn(&self) -> MutexGuard<'_, Connection> {
        self.conn
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}
