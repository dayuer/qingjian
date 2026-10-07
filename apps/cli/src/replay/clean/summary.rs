//! 干净口径的统计：回放照常逐条跑完（上文与内存学习不变），只在这里把剔除的条目拿掉再算。

use std::collections::HashMap;
use std::fmt;

use qingjian_core::InputSource;

use super::Rule;

/// 一条参与评测的上屏。
#[derive(Debug, Clone, Copy)]
struct Row {
    line: usize,

    source: InputSource,

    /// 现在的名次（从 1 起），不在候选为 `None`。
    rank: Option<usize>,
}

/// 剔除表与逐条结果。
#[derive(Debug, Default)]
pub struct Summary {
    excluded: HashMap<usize, Rule>,

    rows: Vec<Row>,

    /// `--clean2`：回放遇到撤销时按真实使用回滚了学习。
    rollback: bool,
}

impl Summary {
    pub fn new(excluded: HashMap<usize, Rule>, rollback: bool) -> Self {
        Self {
            excluded,
            rows: Vec::new(),
            rollback,
        }
    }

    pub fn push(&mut self, line: usize, source: InputSource, rank: Option<usize>) {
        self.rows.push(Row { line, source, rank });
    }

    /// 前半是参与评测的条目（各来源合起来、按日志顺序）的前一半，后半是其余；剔除在切半之后做，两半各自剔。
    fn write_source(
        &self,
        f: &mut fmt::Formatter<'_>,
        name: &str,
        source: InputSource,
    ) -> fmt::Result {
        let half = self.rows.len() / 2;
        let mut parts = Vec::new();
        for (label, range) in [
            ("全部", 0..self.rows.len()),
            ("前半", 0..half),
            ("后半", half..self.rows.len()),
        ] {
            let kept: Vec<&Row> = self.rows[range]
                .iter()
                .filter(|r| r.source == source && !self.excluded.contains_key(&r.line))
                .collect();
            let top1 = kept.iter().filter(|r| r.rank == Some(1)).count();
            let missing = kept.iter().filter(|r| r.rank.is_none()).count();
            let rate = if kept.is_empty() {
                "-".to_owned()
            } else {
                format!("{:.1}%", top1 as f64 * 100.0 / kept.len() as f64)
            };
            parts.push(format!(
                "{label} {top1}/{} = {rate}（不在候选 {missing}）",
                kept.len()
            ));
        }
        writeln!(f, "  {name:<4} {}", parts.join("  "))
    }
}

impl fmt::Display for Summary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let counts: Vec<String> = Rule::ALL
            .iter()
            .map(|rule| {
                let n = self.excluded.values().filter(|r| *r == rule).count();
                format!("{} {n}", rule.label())
            })
            .collect();
        writeln!(
            f,
            "{}（剔除 {} 行：{}）",
            if self.rollback {
                "干净口径 2（撤销回滚学习）"
            } else {
                "干净口径"
            },
            self.excluded.len(),
            counts.join("，")
        )?;
        self.write_source(f, "词", InputSource::Word)?;
        self.write_source(f, "整句", InputSource::Sentence)
    }
}
