//! 跨次运行记住的东西：审过的用户词不再审，加过的词不再加，省大模型的钱。

use std::collections::BTreeSet;
use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TunerState {
    /// 体检过的用户词。
    pub audited: BTreeSet<String>,

    /// 已经提议并推送过、或被大模型否掉的词。
    pub seen: BTreeSet<String>,
}

impl TunerState {
    pub fn load(path: &Path) -> Self {
        std::fs::read(path)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        let temp = path.with_extension("json.tmp");
        std::fs::write(&temp, serde_json::to_vec(self).unwrap_or_default())?;
        std::fs::rename(temp, path)
    }
}
