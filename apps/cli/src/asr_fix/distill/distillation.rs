//! 术语表提炼的统计：多少条纠错对、提炼出多少术语、各种原因跳过多少。

use std::collections::BTreeMap;
use std::fmt;

use super::skip::Skip;

#[derive(Debug, Default)]
pub struct Distillation {
    /// 读进来的条目（含纯术语）。
    pub entries: usize,

    /// 只用确认过的时被筛掉的候选条目。
    pub unconfirmed: usize,

    pub skipped: BTreeMap<Skip, usize>,

    /// 纠错对提炼出的术语：`错 → 对 ⇒ 术语`，同一术语只记第一次。
    pub learned: Vec<String>,
}

impl fmt::Display for Distillation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "术语表 {} 条", self.entries)?;
        if self.unconfirmed > 0 {
            write!(f, "（候选 {} 条未用）", self.unconfirmed)?;
        }
        if !self.skipped.is_empty() {
            let reasons: Vec<String> = self
                .skipped
                .iter()
                .map(|(skip, count)| format!("{} {count}", skip.label()))
                .collect();
            write!(f, "，提炼时跳过：{}", reasons.join("、"))?;
        }
        Ok(())
    }
}
