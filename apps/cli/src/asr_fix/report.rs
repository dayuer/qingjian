//! 纠错报告：替换处数、字错率与术语命中的前后对比、逐处替换按字对齐到参考文本上的对错与明细。

use std::fmt;

use super::corrector::Correction;
use super::distill::Distillation;
use super::fix::Fix;
use super::options::Options;
use super::text::{align, edit_distance, normalize};

#[derive(Debug, Default)]
pub struct Report {
    /// 能纠的术语。
    terms: Vec<String>,

    /// 读不出音（非汉字、生僻字）而跳过的术语。
    skipped_terms: Vec<String>,

    distillation: Distillation,

    options: Options,

    lines: usize,

    /// 换了 / 留着没换的处数。
    changed: usize,

    kept: usize,

    /// 带参考文本的句数。
    referenced: usize,

    /// 换了的处数按参考文本分：参考在那儿是术语 / 仍是原文 / 改写成别的或对不齐。
    applied: Tally,

    /// 没换的处数，同样按参考文本分；参考是术语的就是漏掉的。
    held: Tally,

    /// 参考文本的字数（只算汉字与字母数字），字错率的分母。
    reference_chars: usize,

    errors_before: usize,

    errors_after: usize,

    /// 参考文本里术语出现的次数，与纠前、纠后识别结果里对上的次数。
    term_reference: usize,

    term_before: usize,

    term_after: usize,

    /// 纠完字错变少 / 变多的句数。
    better: usize,

    worse: usize,

    details: Vec<String>,
}

#[derive(Debug, Default, Clone, Copy)]
struct Tally {
    term: usize,

    original: usize,

    other: usize,
}

/// 参考文本在一处替换的位置上是什么。
#[derive(Debug, Clone, Copy)]
enum Verdict {
    Term,

    Original,

    Other,

    /// 没有参考文本。
    Unknown,
}

impl Tally {
    fn count(&mut self, verdict: Verdict) {
        match verdict {
            Verdict::Term => self.term += 1,
            Verdict::Original => self.original += 1,
            Verdict::Other => self.other += 1,
            Verdict::Unknown => {}
        }
    }

    fn total(self) -> usize {
        self.term + self.original + self.other
    }
}

impl Report {
    pub fn new(
        terms: Vec<String>,
        skipped_terms: Vec<String>,
        distillation: Distillation,
        options: Options,
    ) -> Self {
        Self {
            terms,
            skipped_terms,
            distillation,
            options,
            ..Self::default()
        }
    }

    pub fn add(
        &mut self,
        heard: &str,
        reference: Option<&str>,
        correction: &Correction,
        terms: &[String],
    ) {
        self.lines += 1;
        self.changed += correction.applied.len();
        self.kept += correction.held.len();
        let heard_chars: Vec<char> = heard.chars().collect();
        let reference_chars: Vec<char> = reference.map_or_else(Vec::new, |r| r.chars().collect());
        let mapping = reference.map(|_| align(&heard_chars, &reference_chars));
        let verdict = |fix: &Fix| {
            let Some(mapping) = &mapping else {
                return Verdict::Unknown;
            };
            let positions: Option<Vec<usize>> =
                mapping[fix.start..fix.end()].iter().copied().collect();
            let Some(positions) = positions else {
                return Verdict::Other;
            };
            let there: String = positions.iter().map(|&j| reference_chars[j]).collect();
            if there == fix.term {
                Verdict::Term
            } else if there == fix.original {
                Verdict::Original
            } else {
                Verdict::Other
            }
        };
        for fix in &correction.applied {
            let verdict = verdict(fix);
            self.applied.count(verdict);
            let mark = match verdict {
                Verdict::Term => "✓",
                Verdict::Original => "✗",
                Verdict::Other => "?",
                Verdict::Unknown => " ",
            };
            self.details.push(format!(
                "  {mark} 换 {} → {}（读音差 {}{}）  {}",
                fix.original,
                fix.term,
                fix.cost,
                if fix.is_word {
                    "，原文是词库词"
                } else {
                    ""
                },
                context(&heard_chars, fix),
            ));
        }
        for fix in &correction.held {
            let verdict = verdict(fix);
            self.held.count(verdict);
            let mark = match verdict {
                Verdict::Term => "漏",
                Verdict::Original => "对",
                Verdict::Other => "?",
                Verdict::Unknown => " ",
            };
            self.details.push(format!(
                "  {mark} 留 {} ≈ {}（读音差 {}，{}）  {}",
                fix.original,
                fix.term,
                fix.cost,
                fix.doubt(),
                context(&heard_chars, fix),
            ));
        }
        let Some(reference) = reference else {
            return;
        };
        self.referenced += 1;
        let target = normalize(reference);
        let before = edit_distance(&normalize(heard), &target);
        let after = edit_distance(&normalize(&correction.text), &target);
        self.reference_chars += target.len();
        self.errors_before += before;
        self.errors_after += after;
        if after < before {
            self.better += 1;
        } else if after > before {
            self.worse += 1;
        }
        for term in terms {
            let expected = reference.matches(term.as_str()).count();
            self.term_reference += expected;
            self.term_before += heard.matches(term.as_str()).count().min(expected);
            self.term_after += correction.text.matches(term.as_str()).count().min(expected);
        }
    }
}

/// 替换处前后各几个字，明细里给人看上下文。
fn context(chars: &[char], fix: &Fix) -> String {
    const AROUND: usize = 8;
    let start = fix.start.saturating_sub(AROUND);
    let end = (fix.end() + AROUND).min(chars.len());
    let before: String = chars[start..fix.start].iter().collect();
    let inside: String = chars[fix.start..fix.end()].iter().collect();
    let after: String = chars[fix.end()..end].iter().collect();
    format!("…{before}【{inside}】{after}…")
}

impl fmt::Display for Report {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{}", self.distillation)?;
        write!(f, "能纠的术语 {} 条", self.terms.len())?;
        if !self.skipped_terms.is_empty() {
            write!(f, "，读不出音跳过 {} 条", self.skipped_terms.len())?;
        }
        writeln!(
            f,
            "；{}{}",
            if self.options.loose {
                "宽松：原文是词库词、术语是常用词也换"
            } else {
                "原文是词库词或术语是常用词的不换"
            },
            if self.options.holdout {
                "；留一场会议：只学自本场的术语不纠本场"
            } else {
                ""
            },
        )?;
        writeln!(
            f,
            "句子 {} 条，换了 {} 处，留着没换 {} 处",
            self.lines, self.changed, self.kept
        )?;
        if self.referenced > 0 {
            let rate = |errors: usize| 100.0 * errors as f64 / self.reference_chars.max(1) as f64;
            writeln!(
                f,
                "带参考 {} 条：字错率 {:.2}% → {:.2}%，术语命中 {}/{} → {}/{}，变好 {} 句、变差 {} 句",
                self.referenced,
                rate(self.errors_before),
                rate(self.errors_after),
                self.term_before,
                self.term_reference,
                self.term_after,
                self.term_reference,
                self.better,
                self.worse,
            )?;
            writeln!(
                f,
                "换的 {} 处：参考里是术语 {}（✓）、仍是原文 {}（✗ 改错）、改写或对不齐 {}（?）",
                self.applied.total(),
                self.applied.term,
                self.applied.original,
                self.applied.other,
            )?;
            if self.held.total() > 0 {
                writeln!(
                    f,
                    "留的 {} 处：参考里是术语 {}（漏）、仍是原文 {}（对）、其他 {}",
                    self.held.total(),
                    self.held.term,
                    self.held.original,
                    self.held.other,
                )?;
            }
        }
        if !self.distillation.learned.is_empty() {
            writeln!(f, "纠错对提炼出的术语：")?;
            for line in &self.distillation.learned {
                writeln!(f, "  {line}")?;
            }
        }
        if !self.details.is_empty() {
            writeln!(f, "明细：")?;
            for line in &self.details {
                writeln!(f, "{line}")?;
            }
        }
        Ok(())
    }
}
