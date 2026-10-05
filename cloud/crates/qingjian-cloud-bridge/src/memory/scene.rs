//! `memory/scenes.json` 的一项：用户自己建、自己起名的分组。
//! id 建好不变（**改名不动任何文件**——`state.last` 的 key 按 id 走），名字随用户改，场景之间没有行为差异。
//! 老版本的场景是写死的三个（`daily`/`dating`/`work`，还各有一套行为差异），迁移时并成一个 `daily`（见 `MemoryStore::migrate`）。

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use super::MemoryError;

/// 场景名最多几个字。
pub const MAX_SCENE_NAME_CHARS: usize = 8;

/// 新装时唯一那个场景的 id，也是老数据归并后的落点。
pub const DEFAULT_SCENE_ID: &str = "daily";

/// 新装时那个场景的名字。
pub const DEFAULT_SCENE_NAME: &str = "日常";

/// 场景 id 会成为 `state.json` 的 key，所以只收小写字母与数字：
/// 老数据里的 `daily`/`dating`/`work` 与新建的 32 位 hex 都合，`../` 这类一律不认。
pub fn is_scene_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Scene {
    pub id: String,

    /// 用户起的名字。
    pub name: String,

    /// 建这个场景时的 Unix 秒。
    pub created_at: i64,
}

impl Scene {
    pub fn new(id: String, name: String, created_at: i64) -> Self {
        Self {
            id,
            name,
            created_at,
        }
    }

    /// 写盘前收拾名字：去掉首尾空白。
    pub fn normalized(mut self) -> Self {
        self.name = self.name.trim().to_owned();
        self
    }
}

/// 名单上的场景至少要一个、id 合法且不重复、名字非空且不超长。
pub fn validate_scenes(scenes: &[Scene]) -> Result<(), MemoryError> {
    if scenes.is_empty() {
        return Err(MemoryError::Invalid("至少要留一个场景"));
    }
    let mut seen = HashSet::new();
    for scene in scenes {
        if !is_scene_id(&scene.id) {
            return Err(MemoryError::Invalid("场景编号不对"));
        }
        if scene.name.is_empty() {
            return Err(MemoryError::Invalid("场景名不能是空的"));
        }
        if scene.name.chars().count() > MAX_SCENE_NAME_CHARS {
            return Err(MemoryError::Invalid("场景名最多 8 个字"));
        }
        if !seen.insert(scene.id.as_str()) {
            return Err(MemoryError::Invalid("同一个场景出现了两次"));
        }
    }
    Ok(())
}
