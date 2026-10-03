//! 设置页里的一个随包领域词库：文件名、给人看的名字、开没开。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DomainSetting {
    /// 文件名（不含 `.qj`），写进 `[dictionaries] domains`。
    pub id: String,

    /// 词库元数据里的名字，去掉「青简领域词库：」前缀（「法律」）。
    pub label: String,

    pub enabled: bool,
}
