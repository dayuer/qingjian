//! 语音转录纠错原型：拿术语表按读音把识别结果里的同音、近音片段换回术语，看输入法的读音表能纠回多少。
//!
//! 转录文件一行一句：`识别结果[<Tab>参考文本[<Tab>会议]]`；带参考时报字错率、术语命中的前后变化，
//! 并把每处替换按字对齐到参考文本上判对错。
//! 术语表见 [`Entry::parse_all`]：纯术语，或畅译词库那种「错的片段 → 对的片段」纠错对（先 [`distill`] 成术语）。
//! 原文本身是词库词的片段默认不换（多半本来就识别对了），`--asr-fix-loose` 也换，两种都跑一遍比。

mod corrector;
mod distill;
mod entry;
mod fix;
mod options;
mod phonetic;
mod report;
mod term;
mod text;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use qingjian_core::Engine;

pub use options::Options;
pub use report::Report;

use corrector::Corrector;
use distill::{Distillation, distill};
use entry::Entry;
use phonetic::Phonetic;

pub fn run(
    engine: &Engine,
    paths: &[PathBuf],
    terms_path: &Path,
    options: Options,
) -> Result<Report, AsrFixError> {
    let dictionaries = std::iter::once(engine.dictionary()).chain(engine.extra_dictionaries());
    let phonetic = Phonetic::new(dictionaries);
    let (terms, distillation) = collect_terms(&phonetic, &read(terms_path)?, options);
    let (corrector, skipped) = Corrector::new(&phonetic, terms);
    let terms: Vec<String> = corrector.terms().map(str::to_owned).collect();
    let mut report = Report::new(terms.clone(), skipped, distillation, options);
    for path in paths {
        for line in read(path)?.lines() {
            let mut columns = line.split('\t').map(str::trim);
            let heard = columns.next().unwrap_or_default();
            if heard.is_empty() || heard.starts_with('#') {
                continue;
            }
            let reference = columns.next().filter(|r| !r.is_empty());
            let group = columns.next().filter(|g| !g.is_empty());
            let correction = corrector.correct(&phonetic, heard, group, options);
            report.add(heard, reference, &correction, &terms);
        }
    }
    Ok(report)
}

/// 读术语表、把纠错对提炼成术语，同一术语合并来源会议（有一条是全局的就算全局）。
fn collect_terms(
    phonetic: &Phonetic,
    text: &str,
    options: Options,
) -> (Vec<(String, Vec<String>)>, Distillation) {
    let entries = Entry::parse_all(text);
    let mut distillation = Distillation {
        entries: entries.len(),
        ..Distillation::default()
    };
    let mut order: Vec<String> = Vec::new();
    // 术语 → 来源会议；`None` 为全局
    let mut scopes: HashMap<String, Option<Vec<String>>> = HashMap::new();
    for entry in entries {
        if options.confirmed_only && !entry.confirmed {
            distillation.unconfirmed += 1;
            continue;
        }
        let term = match distill(phonetic, entry.wrong.as_deref(), &entry.right) {
            Ok(term) => term,
            Err(skip) => {
                *distillation.skipped.entry(skip).or_default() += 1;
                continue;
            }
        };
        if !scopes.contains_key(&term) {
            order.push(term.clone());
            if let Some(wrong) = &entry.wrong {
                distillation
                    .learned
                    .push(format!("{wrong} → {} ⇒ {term}", entry.right));
            }
        }
        let slot = scopes.entry(term).or_insert_with(|| Some(Vec::new()));
        match (&entry.scope, slot.as_mut()) {
            (None, _) => *slot = None,
            (Some(scope), Some(list)) if !list.contains(scope) => list.push(scope.clone()),
            _ => {}
        }
    }
    let terms = order
        .into_iter()
        .map(|term| {
            let list = scopes.remove(&term).flatten().unwrap_or_default();
            (term, list)
        })
        .collect();
    (terms, distillation)
}

fn read(path: &Path) -> Result<String, AsrFixError> {
    std::fs::read_to_string(path).map_err(|source| AsrFixError::Read {
        path: path.to_owned(),
        source,
    })
}

#[derive(Debug, thiserror::Error)]
pub enum AsrFixError {
    #[error("cannot read {path}: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}
