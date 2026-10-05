//! `memory/state.json`：键盘当前的场景与对象、各场景上次选的人、各人上次被选的时间（键盘写，App 不改）。
//! 提示开关在各个对象上（`Contact`），不在这里。场景是用户自建的分组，按 id 记（见 `memory/scene.rs`）。

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::memory::DEFAULT_SCENE_ID;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ScopeState {
    /// 场景 id。
    pub scene: String,

    /// 当前对象；必须是当前场景的人。
    pub contact_id: Option<String>,

    /// 各场景上次选的人（场景 id → 对象 id）：切场景不指定人时回到这里记的。
    /// 旧文件没有这个字段，读时由 `sanitized_scope` 按当前对象补上。
    pub last: BTreeMap<String, String>,

    /// 对象 id → 上次在键盘里选中这个人的 Unix 秒（选择面板的「今天 / 3 天前」）；旧文件没有时为空。
    pub used: BTreeMap<String, i64>,
}

impl Default for ScopeState {
    fn default() -> Self {
        Self {
            scene: DEFAULT_SCENE_ID.to_owned(),
            contact_id: None,
            last: BTreeMap::new(),
            used: BTreeMap::new(),
        }
    }
}
