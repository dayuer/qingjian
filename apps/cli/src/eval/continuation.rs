//! 本地续写评测：在留出文本上每隔若干字取一个位置，前文给模型、紧接着的几个字当真值，
//! 看续写的头两个字对不对（代理精度），并按平均 log 概率的门槛分档，挑让精度 ≥60% 的 τ。

use std::fmt;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use qingjian_neural::CharScorer;

use crate::eval::EvalError;
use crate::latency::Latencies;

/// 每隔多少字取一个位置。
pub const STRIDE: usize = 20;

/// 前文给多少字（与壳读的前文一致）。
pub const BEFORE_CHARS: usize = 64;

/// 续写长度上限（与 Core 的 `CONTINUATION_MAX_CHARS` 一致）。
pub const MAX_CHARS: usize = 8;

/// 真值至少几个字；代理精度算续写与真值的共同前缀至少几个字。
pub const MIN_TRUTH: usize = 2;
pub const MATCH_CHARS: usize = 2;

/// 报告里分档的门槛 τ（平均 log 概率 ≥ τ 才显示）。
pub const THRESHOLDS: [f64; 7] = [-0.5, -1.0, -1.5, -2.0, -2.5, -3.0, f64::NEG_INFINITY];

/// 一个评测位置。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sample {
    pub before: String,
    pub truth: String,
}

/// 一个位置的续写结果。
#[derive(Debug, Clone)]
pub struct Outcome {
    pub average: f64,
    pub length: usize,

    /// 续写与真值的共同前缀 ≥ [`MATCH_CHARS`] 字。
    pub correct: bool,

    /// 续写的第一个字就对（宽松口径）。
    pub first_correct: bool,
}

/// 从文本里取位置：每行独立；位置前要是汉字，真值是紧接着的汉字（遇非汉字停，最多 [`MAX_CHARS`] 个）且至少 [`MIN_TRUTH`] 个。
pub fn samples(text: &str) -> Vec<Sample> {
    let mut out = Vec::new();
    for line in text.lines() {
        let chars: Vec<char> = line.chars().collect();
        let mut position = STRIDE;
        while position + MIN_TRUTH <= chars.len() {
            let before: String = chars[position.saturating_sub(BEFORE_CHARS)..position]
                .iter()
                .collect();
            let truth: String = chars[position..]
                .iter()
                .take_while(|c| is_han(**c))
                .take(MAX_CHARS)
                .collect();
            if before.chars().last().is_some_and(is_han) && truth.chars().count() >= MIN_TRUTH {
                out.push(Sample { before, truth });
            }
            position += STRIDE;
        }
    }
    out
}

fn is_han(c: char) -> bool {
    matches!(c as u32, 0x3400..=0x4DBF | 0x4E00..=0x9FFF | 0xF900..=0xFAFF | 0x20000..=0x323AF)
}

/// 续写与真值的共同前缀是否够长。
pub fn matches(continued: &str, truth: &str) -> bool {
    continued
        .chars()
        .zip(truth.chars())
        .take_while(|(a, b)| a == b)
        .count()
        >= MATCH_CHARS
}

#[derive(Debug, Default)]
pub struct ContinuationReport {
    pub samples: usize,

    /// 模型写出来了的位置数。
    pub produced: usize,

    pub outcomes: Vec<Outcome>,

    /// 每个位置一次 `continue_text` 的耗时（写没写出来都算）。
    pub latency: Latencies,
}

impl ContinuationReport {
    /// 门槛 τ 下显示的条数、其中正确的条数（共同前缀 ≥2 字）与首字就对的条数。
    pub fn at(&self, threshold: f64) -> (usize, usize, usize) {
        let shown: Vec<&Outcome> = self
            .outcomes
            .iter()
            .filter(|o| o.average >= threshold)
            .collect();
        let correct = shown.iter().filter(|o| o.correct).count();
        let first = shown.iter().filter(|o| o.first_correct).count();
        (shown.len(), correct, first)
    }

    pub fn p90(&self) -> Duration {
        self.latency.quantile(0.9)
    }
}

impl fmt::Display for ContinuationReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "本地续写评测（代理精度 = 续写与真值共同前缀 ≥ {MATCH_CHARS} 字）"
        )?;
        let mean_length = if self.outcomes.is_empty() {
            0.0
        } else {
            self.outcomes.iter().map(|o| o.length).sum::<usize>() as f64
                / self.outcomes.len() as f64
        };
        writeln!(
            f,
            "位置 {}  写出 {}  平均长度 {mean_length:.1} 字  时延 p50 {:.0} ms / p90 {:.0} ms",
            self.samples,
            self.produced,
            self.latency.quantile(0.5).as_secs_f64() * 1000.0,
            self.p90().as_secs_f64() * 1000.0,
        )?;
        writeln!(
            f,
            "{:>8}  {:>8}  {:>10}  {:>10}  {:>8}",
            "τ", "显示率", "代理精度", "首字命中", "条数"
        )?;
        for threshold in THRESHOLDS {
            let (shown, correct, first) = self.at(threshold);
            let shown_rate = if self.samples == 0 {
                0.0
            } else {
                shown as f64 * 100.0 / self.samples as f64
            };
            let precision = if shown == 0 {
                0.0
            } else {
                correct as f64 * 100.0 / shown as f64
            };
            let first_rate = if shown == 0 {
                0.0
            } else {
                first as f64 * 100.0 / shown as f64
            };
            let label = if threshold.is_finite() {
                format!("{threshold:.1}")
            } else {
                "不设".to_owned()
            };
            writeln!(
                f,
                "{label:>8}  {shown_rate:>7.1}%  {precision:>9.1}%  {first_rate:>9.1}%  {shown:>8}"
            )?;
        }
        Ok(())
    }
}

pub fn run(scorer: &CharScorer, paths: &[PathBuf]) -> Result<ContinuationReport, EvalError> {
    let mut report = ContinuationReport::default();
    for path in paths {
        let text = std::fs::read_to_string(path).map_err(|source| EvalError::Read {
            path: path.clone(),
            source,
        })?;
        for sample in samples(&text) {
            report.samples += 1;
            let started = Instant::now();
            let result = scorer.continue_text(&sample.before, MAX_CHARS)?;
            report.latency.push(started.elapsed());
            let Some((continued, average)) = result else {
                continue;
            };
            report.produced += 1;
            report.outcomes.push(Outcome {
                average,
                length: continued.chars().count(),
                correct: matches(&continued, &sample.truth),
                first_correct: continued.chars().next() == sample.truth.chars().next(),
            });
        }
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn samples_need_han_before_and_two_han_after() {
        let line: String = format!("{}输入法的候选排序。", "我".repeat(STRIDE));
        let found = samples(&line);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].truth, "输入法的候选排序");
        assert!(found[0].before.ends_with('我'));
        assert!(samples(&format!("{}abc 输入法", "我".repeat(STRIDE))).is_empty());
    }

    #[test]
    fn match_needs_two_common_leading_characters() {
        assert!(matches("输入法", "输入"));
        assert!(!matches("输出", "输入法"));
        assert!(!matches("", "输入"));
    }
}
