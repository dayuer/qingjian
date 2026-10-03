//! 事件流：写入剪贴板事件、按 `seq` 拉取、按保留策略清理。

use qingjian_cloud_proto::{Event, EventKind, EventPage, MAX_CLIP_BYTES, PushClip};
use rusqlite::{OptionalExtension, Row, params};

use super::{Device, Store};
use crate::ServerError;

const KIND_CLIP_ADDED: &str = "clip_added";
const KIND_CLIP_DELETED: &str = "clip_deleted";

/// 设备被移除后，它留下的事件用这个名字加 id 显示。
const SELECT_EVENT: &str = "SELECT e.seq, COALESCE(d.name, 'device-' || e.device_id), e.at, e.kind,
        e.client_id, e.target, e.text
 FROM events e LEFT JOIN devices d ON d.id = e.device_id";

impl Store {
    /// 记一条剪贴板。同一设备重发同一个 `client_id` 返回原来那条，第二个值为 `false`（不要再广播）。
    pub fn push_clip(
        &self,
        device: &Device,
        clip: &PushClip,
    ) -> Result<(Event, bool), ServerError> {
        if clip.text.is_empty() {
            return Err(ServerError::BadRequest("empty clipboard text".to_owned()));
        }
        if clip.text.len() > MAX_CLIP_BYTES {
            return Err(ServerError::TooLarge(MAX_CLIP_BYTES));
        }
        if clip.client_id.is_empty() || clip.client_id.len() > 64 {
            return Err(ServerError::BadRequest(
                "client_id must be 1-64 bytes".to_owned(),
            ));
        }
        // 先查后插而不用 ON CONFLICT DO NOTHING：后者撞上时 AUTOINCREMENT 的计数照样加一，latest 会虚高。
        // 整个过程在连接锁里，没有并发插入
        let conn = self.conn();
        let existing = conn
            .query_row(
                &format!("{SELECT_EVENT} WHERE e.device_id = ?1 AND e.client_id = ?2"),
                params![device.id, clip.client_id],
                to_event,
            )
            .optional()?;
        if let Some(event) = existing {
            return Ok((event, false));
        }
        conn.execute(
            "INSERT INTO events (device_id, at, kind, client_id, text) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                device.id,
                crate::now_ms(),
                KIND_CLIP_ADDED,
                clip.client_id,
                clip.text
            ],
        )?;
        let event = conn.query_row(
            &format!("{SELECT_EVENT} WHERE e.seq = ?1"),
            [conn.last_insert_rowid()],
            to_event,
        )?;
        Ok((event, true))
    }

    /// 删掉一条剪贴板：原记录连同文本一起删除，再记一条 `ClipDeleted` 让别的设备也删。
    pub fn delete_clip(&self, device: &Device, target: u64) -> Result<Event, ServerError> {
        let mut conn = self.conn();
        let tx = conn.transaction()?;
        let removed = tx.execute(
            "DELETE FROM events WHERE seq = ?1 AND kind = ?2",
            params![target as i64, KIND_CLIP_ADDED],
        )?;
        if removed == 0 {
            return Err(ServerError::NotFound);
        }
        tx.execute(
            "INSERT INTO events (device_id, at, kind, target) VALUES (?1, ?2, ?3, ?4)",
            params![device.id, crate::now_ms(), KIND_CLIP_DELETED, target as i64],
        )?;
        let seq = tx.last_insert_rowid();
        let event = tx.query_row(&format!("{SELECT_EVENT} WHERE e.seq = ?1"), [seq], to_event)?;
        tx.commit()?;
        Ok(event)
    }

    /// `since` 之后的事件，最多 `limit` 条，按 `seq` 升序。
    pub fn events_since(&self, since: u64, limit: usize) -> Result<EventPage, ServerError> {
        let conn = self.conn();
        let mut statement = conn.prepare(&format!(
            "{SELECT_EVENT} WHERE e.seq > ?1 ORDER BY e.seq LIMIT ?2"
        ))?;
        let events = statement
            .query_map(params![since as i64, limit as i64], to_event)?
            .collect::<Result<Vec<_>, _>>()?;
        drop(statement);
        Ok(EventPage {
            events,
            latest: latest_seq(&conn)?,
        })
    }

    /// 服务端当前最新的 `seq`。删掉的事件不让它回退，所以读 AUTOINCREMENT 的计数而不是 `MAX(seq)`。
    pub fn latest_seq(&self) -> Result<u64, ServerError> {
        latest_seq(&self.conn())
    }

    /// 只留最新的 `keep` 条剪贴板，且删掉 `cutoff`（Unix 毫秒）之前的；返回删了几条事件。
    pub fn prune(&self, keep: usize, cutoff: i64) -> Result<usize, ServerError> {
        let conn = self.conn();
        let removed = conn.execute(
            "DELETE FROM events WHERE at < ?1
                OR (kind = ?2 AND seq NOT IN
                    (SELECT seq FROM events WHERE kind = ?2 ORDER BY seq DESC LIMIT ?3))",
            params![cutoff, KIND_CLIP_ADDED, keep as i64],
        )?;
        Ok(removed)
    }
}

fn latest_seq(conn: &rusqlite::Connection) -> Result<u64, ServerError> {
    let seq: Option<i64> = conn
        .query_row(
            "SELECT seq FROM sqlite_sequence WHERE name = 'events'",
            [],
            |row| row.get(0),
        )
        .optional()?;
    Ok(seq.unwrap_or(0) as u64)
}

fn to_event(row: &Row<'_>) -> rusqlite::Result<Event> {
    let kind: String = row.get(3)?;
    let kind = match kind.as_str() {
        KIND_CLIP_DELETED => EventKind::ClipDeleted {
            target: row.get::<_, i64>(5)? as u64,
        },
        KIND_CLIP_ADDED => EventKind::ClipAdded {
            client_id: row.get(4)?,
            text: row.get(6)?,
        },
        other => {
            return Err(rusqlite::Error::InvalidColumnType(
                3,
                format!("unknown event kind {other}"),
                rusqlite::types::Type::Text,
            ));
        }
    };
    Ok(Event {
        seq: row.get::<_, i64>(0)? as u64,
        device: row.get(1)?,
        at: row.get(2)?,
        kind,
    })
}
