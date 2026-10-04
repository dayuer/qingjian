//! `[model] scorers`：加载哪几个本地模型（素笺分叉）。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ScorerSet {
    /// 只有含章·通变（上游行为）：按键 → 整句路径重排与生成。
    Tongbian,

    /// 通变照旧，再加含章·知微按前文给词级候选打分、做本地续写（素笺缺省；多约 56 MB 常驻内存）。
    #[default]
    Both,
}
