//! 同音快照的读写与比对。
//!
//! 快照是纯文本 TSV（进仓库），一行一个候选：`音节\t名次\t词\t词频`。
//! 比对只认「同一组里的顺序与成员有没有变」，变没变与词库怎么改无关 —— 词库改了就在同一个提交里重生成快照。

use std::collections::BTreeMap;
use std::io::Write;
use std::path::Path;

use crate::error::CliError;

/// 每组留几名。
pub const TOP: usize = 5;

/// 一首同音组：音节串 + 前几名（按词频降序，同频按词面）。
pub type Groups = BTreeMap<String, Vec<Candidate>>;

/// 一个候选。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    pub word: String,

    pub frequency: u32,
}

/// 首位被换掉、且旧首位词频至少是新首位这么多倍时报警。
pub const DISPLACE_RATIO: u32 = 3;

/// 一处差异。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Difference {
    /// 多了一组（这一组以前只有一条，现在有竞争了）。
    GroupAdded { pinyin: String, first: String },

    /// 少了一组。
    GroupRemoved { pinyin: String, first: String },

    /// 组里进了新词。
    WordAdded {
        pinyin: String,
        word: String,
        rank: usize,
    },

    /// 组里少了某个词。
    WordRemoved {
        pinyin: String,
        word: String,
        rank: usize,
    },

    /// 名次变了（同一个词在两份快照里的名次不同）。
    RankChanged {
        pinyin: String,
        word: String,
        from: usize,
        to: usize,
    },
}

impl Difference {
    /// 是不是「高频词被挤下首位」那种要报警的差异。
    pub fn alarm(&self, old: &Groups, new: &Groups) -> bool {
        let (Difference::RankChanged {
            pinyin,
            from: 1,
            to,
            ..
        }
        | Difference::WordAdded {
            pinyin, rank: to, ..
        }) = self
        else {
            return false;
        };
        let Some(old_first) = old.get(pinyin).and_then(|list| list.first()) else {
            return false;
        };
        let Some(new_first) = new.get(pinyin).and_then(|list| list.first()) else {
            return false;
        };
        // 新首位得是换上去的那个词（名次变了也可能是别的词挪位）
        if new_first.word == old_first.word {
            return false;
        }
        let _ = to;
        old_first.frequency >= new_first.frequency.saturating_mul(DISPLACE_RATIO)
            && old_first.frequency >= 200
    }

    pub fn text(&self) -> String {
        match self {
            Self::GroupAdded { pinyin, first } => {
                format!("{pinyin}\t新出现同音竞争，首位 {first}")
            }
            Self::GroupRemoved { pinyin, first } => {
                format!("{pinyin}\t同音竞争消失，原首位 {first}")
            }
            Self::WordAdded { pinyin, word, rank } => {
                format!("{pinyin}\t第 {rank} 名新增 {word}")
            }
            Self::WordRemoved { pinyin, word, rank } => {
                format!("{pinyin}\t第 {rank} 名去掉 {word}")
            }
            Self::RankChanged {
                pinyin,
                word,
                from,
                to,
            } => format!("{pinyin}\t{word} 名次 {from} → {to}"),
        }
    }
}

/// 写快照。
pub fn write(groups: &Groups, fingerprint: &str, path: &Path) -> Result<(), CliError> {
    let mut file = std::io::BufWriter::new(std::fs::File::create(path)?);
    writeln!(
        file,
        "# 青简同音快照：按无调音节串分组，每组前 {TOP} 名（词频降序，同频按词面）。\n\
         # 由 `qingjian-cli --homophone-snapshot <文件>` 生成，改词库后要在同一个提交里重生成；\n\
         # CI 跑 `--homophone-snapshot <文件> --check`，不一致就失败（被高频词让位的那种会在输出里标报警）。\n\
         # 词库指纹：{fingerprint}\n\
         #   指纹 = 词库全部条目（词 / 读音 / 词频）的 64 位哈希 + 条目数 —— 快照是从**引擎实际加载的词库**\n\
         #   算的（`--dict` 解析到什么就是什么），没有指纹时 `--check` 只是在比两份不同的词库（2026-10-09 踩过）。\n\
         # 音节\t名次\t词\t词频"
    )?;
    for (pinyin, list) in groups {
        for (index, candidate) in list.iter().enumerate() {
            writeln!(
                file,
                "{pinyin}\t{}\t{}\t{}",
                index + 1,
                candidate.word,
                candidate.frequency
            )?;
        }
    }
    Ok(())
}

/// 读快照。
pub fn read(path: &Path) -> Result<(Option<String>, Groups), CliError> {
    let source = std::fs::read_to_string(path)?;
    let fingerprint = source.lines().find_map(|line| {
        line.strip_prefix("# 词库指纹：")
            .map(|rest| rest.trim().to_owned())
    });
    let mut groups: Groups = BTreeMap::new();
    for line in source.lines() {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        let (Some(pinyin), Some(rank), Some(word), Some(frequency)) =
            (fields.first(), fields.get(1), fields.get(2), fields.get(3))
        else {
            continue;
        };
        let rank: usize = rank.trim().parse().unwrap_or(0);
        if rank == 0 || rank > TOP {
            continue;
        }
        let frequency: u32 = frequency.trim().parse().unwrap_or(0);
        groups
            .entry((*pinyin).to_owned())
            .or_default()
            .push(Candidate {
                word: (*word).to_owned(),
                frequency,
            });
    }
    Ok((fingerprint, groups))
}

/// 比两份快照，按组交回差异（组内按名次、组间按音节，稳定）。
pub fn diff(old: &Groups, new: &Groups) -> Vec<Difference> {
    let mut differences = Vec::new();
    for (pinyin, list) in old {
        match new.get(pinyin) {
            None => {
                if let Some(first) = list.first() {
                    differences.push(Difference::GroupRemoved {
                        pinyin: pinyin.clone(),
                        first: first.word.clone(),
                    });
                }
            }
            Some(new_list) => {
                for (index, candidate) in list.iter().enumerate() {
                    match new_list.iter().position(|c| c.word == candidate.word) {
                        None => differences.push(Difference::WordRemoved {
                            pinyin: pinyin.clone(),
                            word: candidate.word.clone(),
                            rank: index + 1,
                        }),
                        Some(to) if to != index => differences.push(Difference::RankChanged {
                            pinyin: pinyin.clone(),
                            word: candidate.word.clone(),
                            from: index + 1,
                            to: to + 1,
                        }),
                        Some(_) => {}
                    }
                }
                for (index, candidate) in new_list.iter().enumerate() {
                    if !list.iter().any(|c| c.word == candidate.word) {
                        differences.push(Difference::WordAdded {
                            pinyin: pinyin.clone(),
                            word: candidate.word.clone(),
                            rank: index + 1,
                        });
                    }
                }
            }
        }
    }
    for (pinyin, list) in new {
        if !old.contains_key(pinyin)
            && let Some(first) = list.first()
        {
            differences.push(Difference::GroupAdded {
                pinyin: pinyin.clone(),
                first: first.word.clone(),
            });
        }
    }
    differences
}

#[cfg(test)]
mod tests {
    use super::*;

    fn groups(entries: &[(&str, &[(&str, u32)])]) -> Groups {
        entries
            .iter()
            .map(|(pinyin, list)| {
                (
                    (*pinyin).to_owned(),
                    list.iter()
                        .map(|(word, frequency)| Candidate {
                            word: (*word).to_owned(),
                            frequency: *frequency,
                        })
                        .collect(),
                )
            })
            .collect()
    }

    #[test]
    fn round_trips_through_the_file() {
        let dir = std::env::temp_dir().join(format!("qingjian-homophone-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("snap.tsv");
        let original = groups(&[("bai gei", &[("败给", 1413), ("白给", 1331)])]);
        write(&original, "abcd1234 条目 2", &path).unwrap();
        let (fingerprint, read_back) = read(&path).unwrap();
        assert_eq!(read_back, original);
        assert_eq!(
            fingerprint.as_deref(),
            Some("abcd1234 条目 2"),
            "指纹要原样带回"
        );
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_new_word_taking_the_first_slot_is_an_alarm() {
        // 白给（1331）挤掉败给（1413）：旧首位词频是新首位的 3 倍以上就报警
        let old = groups(&[("bai gei", &[("败给", 1413)])]);
        let new = groups(&[("bai gei", &[("白给", 1331), ("败给", 1413)])]);
        let differences = diff(&old, &new);
        assert_eq!(differences.len(), 2, "{differences:?}");
        let alarms = differences
            .iter()
            .filter(|difference| difference.alarm(&old, &new))
            .count();
        assert_eq!(alarms, 0, "1331 与 1413 差不多，不该报警：{differences:?}");

        // 新词弱得多的时候要报警（这里新词只有旧首位的十分之一）
        let old = groups(&[("zhen xiang", &[("真相", 9162)])]);
        let new = groups(&[("zhen xiang", &[("真香", 100), ("真相", 9162)])]);
        let differences = diff(&old, &new);
        assert!(
            differences
                .iter()
                .any(|difference| difference.alarm(&old, &new)),
            "{differences:?}"
        );
    }

    #[test]
    fn identical_snapshots_have_no_differences() {
        let one = groups(&[("nei juan", &[("内卷", 100), ("内眷", 2)])]);
        assert!(diff(&one, &one).is_empty());
    }
}
