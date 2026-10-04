//! 同拼音不同上文的词级评测：`拼音\t前文\t期望` 三列，逐行冷启动、写入前文、查拼音，看首选是不是期望的词；
//! 同一份数据再跑一遍不给前文，两个数字的差就是上文带来的提升。前文走 `Engine::history_mut()`，
//! 与 `--eval-text` 一样（引擎没有壳给的前文时就用本会话历史，见 `Engine::rescoring_context`）。
//! 另报按键同步部分（第一次 `query()`，不含等后台模型）的 p50 / p99。

use std::fmt;
use std::io::{BufWriter, Write};
use std::path::Path;
use std::time::{Duration, Instant};

use qingjian_core::Engine;

use crate::eval::EvalError;
use crate::latency::Latencies;

/// 一条评测对。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextPair {
    pub pinyin: String,

    /// 光标前的文字；空串就是句首。
    pub before: String,

    /// 期望的首选。
    pub expected: String,
}

impl ContextPair {
    /// 解析一行；空行、`#` 开头的注释、列数不对的行返回 `None`。
    pub fn parse(line: &str) -> Option<Self> {
        let line = line.trim_end_matches('\r');
        if line.trim().is_empty() || line.starts_with('#') {
            return None;
        }
        let mut columns = line.split('\t');
        let pinyin = columns.next()?.trim();
        let before = columns.next()?.trim();
        let expected = columns.next()?.trim();
        (!pinyin.is_empty() && !expected.is_empty()).then(|| Self {
            pinyin: pinyin.to_owned(),
            before: before.to_owned(),
            expected: expected.to_owned(),
        })
    }
}

/// 一对在一种设置下的结果。
#[derive(Debug, Clone, Default)]
struct Outcome {
    /// 期望在候选里的名次（0 是首选）；`None` 是不在候选里。
    position: Option<usize>,

    /// 拼音切不动。
    unparsable: bool,

    top: Vec<String>,

    /// 第一次 `query()` 的耗时：按键回调里同步做的那部分。
    sync: Duration,

    /// 含等后台模型、再查一次的总耗时。
    total: Duration,
}

/// 评测报告。
#[derive(Debug, Default)]
pub struct ContextReport {
    pub total: usize,

    /// 拼音切不动的对数（两种设置下一样）。
    pub unparsable: usize,

    /// 有前文时首选命中。
    pub with_context: usize,

    /// 不给前文时首选命中。
    pub without_context: usize,

    /// 有前文时期望根本不在候选里：多半是评测集的拼音或期望写错了。
    pub missing: usize,

    /// 有前文那一遍的同步耗时与总耗时。
    pub sync: Latencies,
    pub total_time: Duration,

    /// 有前文仍没命中首选的例子。
    pub misses: Vec<String>,
}

impl ContextReport {
    pub fn evaluated(&self) -> usize {
        self.total - self.unparsable
    }

    fn percent(part: usize, whole: usize) -> f64 {
        if whole == 0 {
            0.0
        } else {
            part as f64 * 100.0 / whole as f64
        }
    }
}

impl fmt::Display for ContextReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let evaluated = self.evaluated();
        let with = Self::percent(self.with_context, evaluated);
        let without = Self::percent(self.without_context, evaluated);
        writeln!(f, "同拼音不同上文评测（冷启动，不学习）")?;
        writeln!(
            f,
            "对数 {:>5}  有前文首选 {with:>5.1}%  无前文首选 {without:>5.1}%  提升 {:+.1} 个百分点  期望不在候选 {}",
            self.total,
            with - without,
            self.missing,
        )?;
        if evaluated > 0 {
            writeln!(
                f,
                "按键同步部分 {}，含等模型的平均 {:.1} ms",
                self.sync.summary(),
                self.total_time.as_secs_f64() * 1000.0 / evaluated as f64,
            )?;
        }
        if self.unparsable > 0 {
            writeln!(f, "其中 {} 对拼音切不动", self.unparsable)?;
        }
        if !self.misses.is_empty() {
            writeln!(f, "\n有前文仍没命中首选的例子：")?;
            for miss in &self.misses {
                writeln!(f, "  {miss}")?;
            }
        }
        Ok(())
    }
}

/// 读评测集。
pub fn load(path: &Path) -> Result<Vec<ContextPair>, EvalError> {
    let text = std::fs::read_to_string(path).map_err(|source| EvalError::Read {
        path: path.to_owned(),
        source,
    })?;
    Ok(text.lines().filter_map(ContextPair::parse).collect())
}

/// 跑一遍：每对先给前文查一次，再不给前文查一次。`details` 给了就逐对写 JSONL。
pub fn run(
    engine: &mut Engine,
    path: &Path,
    show_misses: usize,
    details: Option<&Path>,
) -> Result<ContextReport, EvalError> {
    let pairs = load(path)?;
    let mut writer = details
        .map(|path| {
            std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(path)
                .map(BufWriter::new)
                .map_err(|source| EvalError::Write {
                    path: path.to_owned(),
                    source,
                })
        })
        .transpose()?;
    let mut report = ContextReport::default();
    for pair in &pairs {
        report.total += 1;
        let with = evaluate(engine, pair, Some(&pair.before));
        let without = evaluate(engine, pair, None);
        if with.unparsable {
            report.unparsable += 1;
            continue;
        }
        report.sync.push(with.sync);
        report.total_time += with.total;
        report.with_context += usize::from(with.position == Some(0));
        report.without_context += usize::from(without.position == Some(0));
        report.missing += usize::from(with.position.is_none());
        if with.position != Some(0) && report.misses.len() < show_misses {
            report.misses.push(format!(
                "{:<14} {:<16} 期望 {:<6} 有前文前三 {}{}",
                pair.pinyin,
                pair.before,
                pair.expected,
                with.top.join(" / "),
                with.position
                    .map_or(String::from("（不在候选里）"), |i| format!(
                        "（第 {} 位）",
                        i + 1
                    )),
            ));
        }
        if let Some(writer) = &mut writer {
            let row = serde_json::json!({
                "pinyin": pair.pinyin, "before": pair.before, "expected": pair.expected,
                "position_with_context": with.position, "top_with_context": with.top,
                "position_without_context": without.position, "top_without_context": without.top,
                "sync_ms": with.sync.as_secs_f64() * 1000.0,
                "total_ms": with.total.as_secs_f64() * 1000.0,
            });
            writeln!(writer, "{row}").map_err(|source| EvalError::Write {
                path: details.expect("writer has path").to_owned(),
                source,
            })?;
        }
    }
    if let Some(writer) = &mut writer {
        writer.flush().map_err(|source| EvalError::Write {
            path: details.expect("writer has path").to_owned(),
            source,
        })?;
    }
    Ok(report)
}

/// 评一对：清空引擎、写（或不写）前文、喂拼音、看期望在第几位。异步重打分像壳一样请求并等结果。
fn evaluate(engine: &mut Engine, pair: &ContextPair, before: Option<&str>) -> Outcome {
    engine.clear();
    engine.break_chain();
    engine.history_mut().clear();
    if let Some(before) = before {
        engine.history_mut().record(before);
    }
    engine.set_input(&pair.pinyin);
    let started = Instant::now();
    let Ok(query) = engine.query() else {
        engine.clear();
        return Outcome {
            unparsable: true,
            ..Outcome::default()
        };
    };
    let sync = started.elapsed();
    let query = if crate::rescoring::settle(engine) {
        engine.query().unwrap_or(query)
    } else {
        query
    };
    let total = started.elapsed();
    let items = &query.candidates.items;
    let outcome = Outcome {
        position: items.iter().position(|c| c.text == pair.expected),
        unparsable: false,
        top: items.iter().take(3).map(|c| c.text.clone()).collect(),
        sync,
        total,
    };
    engine.clear();
    outcome
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_three_columns_and_skips_comments() {
        assert_eq!(ContextPair::parse("# 注释"), None);
        assert_eq!(ContextPair::parse(""), None);
        assert_eq!(ContextPair::parse("youxiang\t汽车"), None);
        assert_eq!(
            ContextPair::parse("youxiang\t汽车\t油箱\r"),
            Some(ContextPair {
                pinyin: "youxiang".into(),
                before: "汽车".into(),
                expected: "油箱".into(),
            })
        );
        // 前文可以为空（句首那组）
        assert_eq!(ContextPair::parse("ba\t\t把").unwrap().before, "");
    }
}
