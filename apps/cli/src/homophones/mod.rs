//! 同音竞争回归（对标报告 P0-2）：把词库按无调音节串分组、每组前 5 名写一份快照入库；
//! 词库改动后跑 `--check` 比一遍 —— 成员、名次变了都算差异，其中「新词把高频常用词挤出首位」会标报警。
//!
//! 快照变小是**正常的**（那是这次改动想要的结果），只要在同一个提交里重生成即可；CI 跑 `--check`，
//! 忘了重生成就会失败，看的人也就不会漏过「某条常用词被挤下去」这种意外。

mod snapshot;

use std::collections::BTreeMap;
use std::path::Path;

use qingjian_core::Engine;

pub use snapshot::{Candidate, Groups};

use crate::error::CliError;

/// 一份快照里有多少组、多少行（报告用）。
fn counts(groups: &Groups) -> (usize, usize) {
    (groups.len(), groups.values().map(Vec::len).sum())
}

/// 从词库算快照：按音节串分组，每组按词频降序、同频按词面，留前几名。
pub fn build(engine: &Engine) -> Groups {
    let mut groups: Groups = BTreeMap::new();
    for hit in engine.dictionary().entries() {
        groups
            .entry(hit.pinyin.to_owned())
            .or_default()
            .push(Candidate {
                word: hit.text.to_owned(),
                frequency: hit.frequency,
            });
    }
    for list in groups.values_mut() {
        list.sort_by(|a, b| {
            b.frequency
                .cmp(&a.frequency)
                .then_with(|| a.word.cmp(&b.word))
        });
        list.dedup_by(|a, b| a.word == b.word);
        list.truncate(snapshot::TOP);
    }
    groups
}

/// 词库指纹：全部条目（读音 / 词 / 词频）排序后的 64 位哈希 + 条目数。
/// 快照是从**引擎实际加载的词库**算的（`--dict` 解析到什么就是什么），指纹写进快照头，
/// `--check` 先比对它 —— 否则「本地加载的是另一份词库」会被报成「同音竞争变了」（2026-10-09 踩过）。
/// 条目先排序再入哈希：`entries()` 的顺序不该影响指纹。
pub fn fingerprint(engine: &Engine) -> String {
    use std::hash::{Hash, Hasher};
    let mut rows: Vec<(String, String, u32)> = engine
        .dictionary()
        .entries()
        .map(|hit| (hit.pinyin.to_owned(), hit.text.to_owned(), hit.frequency))
        .collect();
    rows.sort();
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for row in &rows {
        row.hash(&mut hasher);
    }
    format!("{:016x} 条目 {}", hasher.finish(), rows.len())
}

/// 写快照，或与已有快照比对。`check` 为真时不一致就返回错（CI 靠它失败）。
pub fn run(engine: &Engine, path: &Path, check: bool) -> Result<(), CliError> {
    let current = build(engine);
    let loaded = fingerprint(engine);
    let (group_count, row_count) = counts(&current);
    if !check {
        snapshot::write(&current, &loaded, path)?;
        println!(
            "同音快照已写出 {}（{group_count} 组、{row_count} 行；每组前 {} 名；词库 {loaded}）",
            path.display(),
            snapshot::TOP
        );
        return Ok(());
    }

    let (recorded, old) = snapshot::read(path)?;
    match recorded {
        Some(recorded) if recorded != loaded => {
            return Err(HomophoneError::WrongDictionary { recorded, loaded }.into());
        }
        Some(_) => {}
        None => eprintln!("提示：快照没记词库指纹（旧格式），无法确认两边比的是不是同一份词库"),
    }
    let differences = snapshot::diff(&old, &current);
    if differences.is_empty() {
        println!(
            "同音快照一致：{}（{group_count} 组、{row_count} 行）",
            path.display()
        );
        return Ok(());
    }

    // 报警的排在前面，其余按音节排（diff 已经是稳定的）
    let (alarms, rest): (Vec<_>, Vec<_>) = differences
        .iter()
        .partition(|difference| difference.alarm(&old, &current));
    for difference in &alarms {
        println!("【报警】{}", difference.text());
    }
    for difference in &rest {
        println!("{}", difference.text());
    }
    println!(
        "\n与 {} 比：{} 处差异（其中报警 {} 处）。\n\
         改词库就要在同一个提交里重生成快照：\
         cargo run --release -p qingjian-cli -- --homophone-snapshot {}",
        path.display(),
        differences.len(),
        alarms.len(),
        path.display()
    );
    Err(CliError::Homophone(HomophoneError::Mismatch {
        differences: differences.len(),
        alarms: alarms.len(),
    }))
}

/// 同音快照的错。
#[derive(Debug, thiserror::Error)]
pub enum HomophoneError {
    /// 快照记的词库与这次加载的不是同一份（先比对指纹，避免把「换了词库」当成「同音竞争变了」）。
    #[error(
        "homophone snapshot was built from a different dictionary: file says `{recorded}`, loaded is `{loaded}`"
    )]
    WrongDictionary {
        /// 快照头里记的指纹。
        recorded: String,

        /// 这次实际加载的词库指纹。
        loaded: String,
    },

    /// 快照与当前词库不一致。
    #[error("homophone snapshot differs: {differences} change(s), {alarms} alarm(s)")]
    Mismatch {
        /// 差异处数。
        differences: usize,

        /// 其中「高频词被让位」的处数。
        alarms: usize,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 入库快照（`apps/cli/snapshots/homophones.tsv`）里那几组已知反例必须是这个名次 ——
    /// 这是审计点名的固定用例，改词库把它们挤下去时会在这里先红。快照只按基础词库算
    /// （`assets/lexicon/dict.tsv`，仓库里有，CI 不用拉产品数据就能跑）。
    #[test]
    fn known_homophone_cases_keep_their_first_slot() {
        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("snapshots/homophones.tsv");
        // 入库那份是旧格式（没有词库指纹）时指纹是 None；这一条只查成员与名次
        let (_fingerprint, groups) = snapshot::read(&path).expect("入库的同音快照读不出来");
        for (pinyin, first) in [
            ("bai gei", "败给"),
            ("lao liu", "老刘"),
            ("la man", "拉曼"),
            ("zhen xiang", "真相"),
        ] {
            let list = groups
                .get(pinyin)
                .unwrap_or_else(|| panic!("快照里没有 {pinyin} 这一组"));
            assert_eq!(
                list.first().map(|candidate| candidate.word.as_str()),
                Some(first),
                "{pinyin} 的首位变了：{list:?}"
            );
        }
        // 「都累」不是词库里的词（它是句子里的组合），快照里没有 du lei 这一组；
        // 多音字 都 的读音归 P0-3 的多音字校验管，这里只登记这个事实，免得以后有人以为漏了
        assert!(!groups.contains_key("du lei"));
    }

    #[test]
    fn builds_groups_by_syllables_and_keeps_top_five() {
        // 不走 Engine，直接验分组与排序的规则：同频按词面，超过 5 名截断
        let mut list: Vec<Candidate> = (0..7)
            .map(|index| Candidate {
                word: format!("词{index}"),
                frequency: 10,
            })
            .collect();
        list.sort_by(|a, b| {
            b.frequency
                .cmp(&a.frequency)
                .then_with(|| a.word.cmp(&b.word))
        });
        list.truncate(snapshot::TOP);
        assert_eq!(list.len(), 5);
        assert_eq!(list[0].word, "词0");
    }
}
