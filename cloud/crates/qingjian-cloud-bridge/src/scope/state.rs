//! `memory/state.json`：键盘当前的场景与对象（键盘写，App 不改）。提示开关在各个对象上（`Contact`），不在这里。

use qingjian_cloud_proto::Scene;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ScopeState {
    pub scene: Scene,

    /// 当前对象；只在恋爱场景有。
    pub contact_id: Option<String>,
}

impl Default for ScopeState {
    fn default() -> Self {
        Self {
            scene: Scene::Daily,
            contact_id: None,
        }
    }
}
