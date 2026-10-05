//! `memory/<对象 id>/materials.jsonl` 的读写，`MemoryStore` 的一组方法；锁沿用 `memory/.lock`。
//! 每次都整份原子重写（同目录临时文件加改名），不在文件尾追加：删一条、标已上传 / 已整理、清过期都要重写，
//! 而且键盘扩展随时可能被系统杀掉，追加到一半会留下半行；一个对象最多 200 条未整理的（每条 ≤ 2000 字节），整份重写的量很小。
//! 读写时顺手删掉整理过 30 天的。不建对象目录：名单上没有的人写不进来，忘掉的人目录已删，读出来为空。
//! 有一行解析不了就整份改名备份（与其余记忆文件一样），日志只记行号与错误种类，不记内容。
//!
//! 另有一个**不绑对象**的桶 `memory/unassigned.jsonl`（App 首页「+ 记一条」先快速记下、还没补上归人的）：
//! 它与按对象的那份同格式、同一套切段与上限，只是放在 `memory/` 根上。
//! **不能**放进 `<某个伪对象 id>/materials.jsonl`：`write_snapshot` 会把名单上没了的人的目录连内容一起删
//! （`store.rs` 里那段），每次 App 写快照都会把无主桶清掉。

use std::collections::HashSet;
use std::io::ErrorKind;
use std::path::PathBuf;

use qingjian_cloud_proto::MemoryKind;

use super::{
    MAX_UNPROCESSED_MATERIALS, Material, MaterialSource, PROCESSED_KEEP_DAYS, split_note,
    unprocessed,
};
use crate::cloud_config::write_atomic;
use crate::memory::store::quarantine;
use crate::memory::{MemoryError, MemoryStore, new_id};
use crate::scope::is_contact_id;

const MATERIALS_FILE: &str = "materials.jsonl";

/// 不绑对象的那个桶；放在 `memory/` 根上（理由见文件头）。
const UNASSIGNED_FILE: &str = "unassigned.jsonl";

const DAY_SECS: i64 = 86_400;

impl MemoryStore {
    /// 「记一笔」：原话切成每条不超过 2000 字节的几条素材存下（时间都是 `now`），返回存下的。
    /// 没整理的加上这几条超过 [`MAX_UNPROCESSED_MATERIALS`] 就整次不写、返回 [`MemoryError::MaterialLimit`]（带剩几个空位与这次几条）。
    pub fn add_material(
        &self,
        contact_id: &str,
        text: &str,
        source: MaterialSource,
        now: i64,
    ) -> Result<Vec<Material>, MemoryError> {
        let pieces = split_note(text);
        if pieces.is_empty() {
            return Err(MemoryError::Invalid("没有要记的文字"));
        }
        check_contact_id(contact_id)?;
        let _lock = self.lock()?;
        self.require_contact(contact_id)?;
        let mut materials = self.read_materials(contact_id)?;
        prune(&mut materials, now);
        let remaining = MAX_UNPROCESSED_MATERIALS.saturating_sub(unprocessed(&materials));
        if pieces.len() > remaining {
            return Err(MemoryError::MaterialLimit {
                remaining,
                needed: pieces.len(),
            });
        }
        let mut added = Vec::with_capacity(pieces.len());
        for text in pieces {
            added.push(Material {
                client_id: new_id()?,
                kind: MemoryKind::Note,
                text,
                at: now,
                source,
                uploaded: false,
                processed: false,
                processed_at: None,
            });
        }
        materials.extend(added.iter().cloned());
        self.write_materials(contact_id, &materials)?;
        tracing::debug!(added = added.len(), "记一笔存成素材");
        Ok(added)
    }

    /// 这个人留着的素材（没整理的与整理不到 30 天的），按写入顺序；整理过 30 天的顺手删掉。
    /// 没有文件（旧目录、忘掉的人）时为空。
    pub fn materials(&self, contact_id: &str, now: i64) -> Result<Vec<Material>, MemoryError> {
        check_contact_id(contact_id)?;
        let _lock = self.lock()?;
        let mut materials = self.read_materials(contact_id)?;
        if prune(&mut materials, now) {
            // 只是顺手清理，写不回去也照样把读到的给出去
            if let Err(error) = self.write_materials(contact_id, &materials) {
                tracing::warn!(code = error.code(), "过期素材没清掉");
            }
        }
        Ok(materials)
    }

    /// 删一条；没有这条（已经删过）也算成功。
    pub fn delete_material(
        &self,
        contact_id: &str,
        client_id: &str,
        now: i64,
    ) -> Result<(), MemoryError> {
        check_contact_id(contact_id)?;
        self.rewrite_materials(contact_id, now, |materials| {
            let before = materials.len();
            materials.retain(|m| m.client_id != client_id);
            before != materials.len()
        })
    }

    /// 2B 上传成功后标 `uploaded`（M3 调用）。
    pub fn mark_materials_uploaded(
        &self,
        contact_id: &str,
        client_ids: &[&str],
        now: i64,
    ) -> Result<(), MemoryError> {
        check_contact_id(contact_id)?;
        let ids: HashSet<&str> = client_ids.iter().copied().collect();
        self.rewrite_materials(contact_id, now, |materials| {
            let mut changed = false;
            for m in materials
                .iter_mut()
                .filter(|m| !m.uploaded && ids.contains(m.client_id.as_str()))
            {
                m.uploaded = true;
                changed = true;
            }
            changed
        })
    }

    /// 2C 下发的卡确认后标 `processed`，从 `now` 起再留 30 天（M4 调用）。
    pub fn mark_materials_processed(
        &self,
        contact_id: &str,
        client_ids: &[&str],
        now: i64,
    ) -> Result<(), MemoryError> {
        check_contact_id(contact_id)?;
        let ids: HashSet<&str> = client_ids.iter().copied().collect();
        self.rewrite_materials(contact_id, now, |materials| {
            let mut changed = false;
            for m in materials
                .iter_mut()
                .filter(|m| !m.processed && ids.contains(m.client_id.as_str()))
            {
                m.processed = true;
                m.processed_at = Some(now);
                changed = true;
            }
            changed
        })
    }

    /// 锁里读出、清过期、交给 `edit` 改（返回有没有改），有变化才整份写回。
    fn rewrite_materials(
        &self,
        contact_id: &str,
        now: i64,
        edit: impl FnOnce(&mut Vec<Material>) -> bool,
    ) -> Result<(), MemoryError> {
        let _lock = self.lock()?;
        let mut materials = self.read_materials(contact_id)?;
        let pruned = prune(&mut materials, now);
        if edit(&mut materials) || pruned {
            self.write_materials(contact_id, &materials)?;
        }
        Ok(())
    }

    /// 不在按空；读不了（开机后还没解锁过）原样报错，不拿空表覆盖真文件。
    fn read_materials(&self, contact_id: &str) -> Result<Vec<Material>, MemoryError> {
        read_material_file(&self.materials_path(contact_id))
    }

    /// 整份写临时文件再改名；对象目录不在（忘掉了）时报 io，不建目录。
    fn write_materials(&self, contact_id: &str, materials: &[Material]) -> Result<(), MemoryError> {
        write_material_file(&self.materials_path(contact_id), materials)
    }

    fn materials_path(&self, contact_id: &str) -> PathBuf {
        self.root().join(contact_id).join(MATERIALS_FILE)
    }

    fn unassigned_path(&self) -> PathBuf {
        self.root().join(UNASSIGNED_FILE)
    }

    /// 无主桶里还没归人的素材（App 首页「+ 记一条」记下的），按写入顺序；顺手清整理过 30 天的。
    pub fn unassigned_materials(&self, now: i64) -> Result<Vec<Material>, MemoryError> {
        let _lock = self.lock()?;
        let mut materials = read_material_file(&self.unassigned_path())?;
        if prune(&mut materials, now) {
            // 只是顺手清理，写不回去也照样把读到的给出去
            if let Err(error) = write_material_file(&self.unassigned_path(), &materials) {
                tracing::warn!(code = error.code(), "过期的无主素材没清掉");
            }
        }
        Ok(materials)
    }

    /// 「+ 记一条」：原话切成几条存进**无主桶**（时间都是 `now`），返回存下的。
    /// 切段与上限跟按对象的那套完全一样（超过 [`MAX_UNPROCESSED_MATERIALS`] 整次不写、返回 [`MemoryError::MaterialLimit`]）。
    pub fn add_unassigned_material(
        &self,
        text: &str,
        source: MaterialSource,
        now: i64,
    ) -> Result<Vec<Material>, MemoryError> {
        let pieces = split_note(text);
        if pieces.is_empty() {
            return Err(MemoryError::Invalid("没有要记的文字"));
        }
        let _lock = self.lock()?;
        let mut materials = read_material_file(&self.unassigned_path())?;
        prune(&mut materials, now);
        let remaining = MAX_UNPROCESSED_MATERIALS.saturating_sub(unprocessed(&materials));
        if pieces.len() > remaining {
            return Err(MemoryError::MaterialLimit {
                remaining,
                needed: pieces.len(),
            });
        }
        let mut added = Vec::with_capacity(pieces.len());
        for text in pieces {
            added.push(Material {
                client_id: new_id()?,
                kind: MemoryKind::Note,
                text,
                at: now,
                source,
                uploaded: false,
                processed: false,
                processed_at: None,
            });
        }
        materials.extend(added.iter().cloned());
        write_material_file(&self.unassigned_path(), &materials)?;
        tracing::debug!(added = added.len(), "「+ 记一条」存成无主素材");
        Ok(added)
    }

    /// 「补上」：把无主桶里的一条素材挪到某个人名下（写进那个人的 `materials.jsonl`）。
    /// 人要在名单上、素材要在无主桶里；两次写都成功才算数。
    pub fn assign_material(
        &self,
        client_id: &str,
        to: &str,
        now: i64,
    ) -> Result<(), MemoryError> {
        check_contact_id(to)?;
        let _lock = self.lock()?;
        self.require_contact(to)?;
        let mut unassigned = read_material_file(&self.unassigned_path())?;
        prune(&mut unassigned, now);
        let Some(index) = unassigned.iter().position(|m| m.client_id == client_id) else {
            return Err(MemoryError::Invalid("无主素材里没有这条"));
        };
        let mut target = self.read_materials(to)?;
        prune(&mut target, now);
        let material = unassigned.remove(index);
        target.push(material);
        write_material_file(&self.unassigned_path(), &unassigned)?;
        self.write_materials(to, &target)?;
        tracing::debug!("一条无主素材归到了人");
        Ok(())
    }
}

/// 读一份素材文件（一行一条 JSONL）；文件不在为空，读到坏行就整份改名备份后按空。
fn read_material_file(path: &std::path::Path) -> Result<Vec<Material>, MemoryError> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error.into()),
    };
    let mut materials = Vec::new();
    for (index, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<Material>(line) {
            Ok(material) => materials.push(material),
            Err(error) => {
                // serde_json 的报错可能带上字段里的原话，只记行号与种类
                tracing::warn!(
                    line = index + 1,
                    kind = ?error.classify(),
                    "素材文件坏了，改名备份后按空处理"
                );
                quarantine(path);
                return Ok(Vec::new());
            }
        }
    }
    Ok(materials)
}

/// 整份写临时文件再改名。不建上级目录：对象目录由建对象时建，无主桶的上级就是 `memory/`。
fn write_material_file(
    path: &std::path::Path,
    materials: &[Material],
) -> Result<(), MemoryError> {
    let mut text = String::new();
    for material in materials {
        let line =
            serde_json::to_string(material).map_err(|_| MemoryError::Invalid("数据编码失败"))?;
        text.push_str(&line);
        text.push('\n');
    }
    write_atomic(path, text.as_bytes(), false)?;
    Ok(())
}

/// 删掉整理过 [`PROCESSED_KEEP_DAYS`] 天的（没有 `processed_at` 的按 `at` 算），删了返回 true。
fn prune(materials: &mut Vec<Material>, now: i64) -> bool {
    let before = materials.len();
    let cutoff = now.saturating_sub(PROCESSED_KEEP_DAYS * DAY_SECS);
    materials.retain(|m| !m.processed || m.processed_at.unwrap_or(m.at) > cutoff);
    let removed = before - materials.len();
    if removed > 0 {
        tracing::debug!(removed, "清掉整理过 30 天的素材");
    }
    removed > 0
}

fn check_contact_id(contact_id: &str) -> Result<(), MemoryError> {
    if is_contact_id(contact_id) {
        Ok(())
    } else {
        Err(MemoryError::Invalid("对象编号不对"))
    }
}
