//! 拆包报告：每本领域包删前删后多少条、按哪条规则删的、有多少条从基础词库移出去，
//! 外加每本最多 20 条被删样本（按文档频次降序取，专挑最「可惜」的那些，便于人工复核有没有误伤）。
//! 报告随第 1 步的规则一起产出（`data/generated/domain-report.tsv`），不写进 git（在 data/ 下）。

use std::collections::BTreeMap;
use std::io::Write;
use std::path::Path;

use super::domain_filter::Reason;
use crate::error::ConvertError;

/// 每本包最多留几条样本。
const SAMPLES_PER_PACK: usize = 20;

/// 一本包的账。
#[derive(Debug, Default)]
struct Tally {
    /// 进这本包（或留在基础词库）的条数。
    kept: usize,

    /// 按规则丢掉的条数，按规则分。
    dropped: BTreeMap<&'static str, usize>,

    /// 改派到别的包（比如地名 → places-extended）的条数。
    moved: usize,

    /// 本来会进基础词库、被 R4 移到包里的条数（诗句）。
    moved_from_base: usize,

    /// 被丢掉的样本：词、文档频次、规则。
    samples: Vec<(String, u64, Reason)>,
}

/// 拆包报告。
#[derive(Debug, Default)]
pub struct DomainReport {
    packs: BTreeMap<String, Tally>,
}

impl DomainReport {
    fn tally(&mut self, stem: &str) -> &mut Tally {
        self.packs.entry(stem.to_owned()).or_default()
    }

    /// 一条留在原处（进本包，或按语料次数进基础词库）。
    pub fn record_kept(&mut self, stem: &str) {
        self.tally(stem).kept += 1;
    }

    /// 一条被规则丢掉。
    pub fn record_drop(&mut self, stem: &str, word: &str, df: u64, reason: Reason) {
        let tally = self.tally(stem);
        *tally.dropped.entry(reason.as_str()).or_default() += 1;
        tally.samples.push((word.to_owned(), df, reason));
    }

    /// 一条被改派。`to` 与来源同一本时（诗词名句整本移出词库）算「留用」，不算改派。
    /// `from_base` 表示它本来会因为语料常见而留在基础词库。
    pub fn record_move(&mut self, stem: &str, to: &str, from_base: bool) {
        let tally = self.tally(stem);
        if to == stem {
            tally.kept += 1;
        } else {
            tally.moved += 1;
            self.tally(to).kept += 1;
        }
        if from_base {
            self.tally(stem).moved_from_base += 1;
        }
    }

    /// 写出报告。
    pub fn write(&mut self, path: &Path) -> Result<(), ConvertError> {
        let mut text = String::from(
            "# 青简领域包拆包报告（词库分层第 1 步，规则见 docs/plan/dictionary-layering.md）\n\
# 包\t送到规则\t留用条数\t删掉\tR1 长度\tR2 寄主学名\tR3 改派\tR4 移出基础库\n",
        );
        for (stem, tally) in &self.packs {
            let dropped: usize = tally.dropped.values().sum();
            let by = |name: &str| tally.dropped.get(name).copied().unwrap_or(0);
            text.push_str(&format!(
                "{stem}\t{}\t{}\t{dropped}\t{}\t{}\t{}\t{}\n",
                tally.kept + dropped + tally.moved,
                tally.kept,
                by(Reason::TooLong.as_str()),
                by(Reason::HostAndTaxon.as_str()),
                tally.moved,
                tally.moved_from_base,
            ));
        }
        text.push_str(
            "\n# 被删样本（每本最多 20 条，按文档频次降序；规则见第 1 步）\n# 包\t规则\t文档频次\t词\n",
        );
        for (stem, tally) in &mut self.packs {
            tally
                .samples
                .sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
            for (word, df, reason) in tally.samples.iter().take(SAMPLES_PER_PACK) {
                text.push_str(&format!("{stem}\t{}\t{df}\t{word}\n", reason.as_str()));
            }
        }
        let mut file = std::io::BufWriter::new(std::fs::File::create(path)?);
        file.write_all(text.as_bytes())?;
        tracing::info!(path = %path.display(), "领域包拆包报告已写出");
        Ok(())
    }
}
