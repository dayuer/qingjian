//! `memory/state.json`：键盘当前的场景与对象，以及各场景上次选的人（键盘写，App 不改）。提示开关在各个对象上（`Contact`），不在这里。

use std::collections::BTreeMap;

use qingjian_cloud_proto::Scene;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ScopeState {
    pub scene: Scene,

    /// 当前对象；必须是当前场景的人。
    pub contact_id: Option<String>,

    /// 各场景上次选的人：切场景不指定人时回到这里记的。旧文件没有这个字段，读时由 `sanitized_scope` 按当前对象补上。
    pub last: BTreeMap<Scene, String>,
}

impl Default for ScopeState {
    fn default() -> Self {
        Self {
            scene: Scene::Daily,
            contact_id: None,
            last: BTreeMap::new(),
        }
    }
}
