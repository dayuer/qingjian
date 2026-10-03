//! 合并别的设备的学习增量：青简 Cloud 的常驻程序把服务器上的变化写成收件箱文本，壳交给 [`Learner::merge_remote`]。
//! 增量是相加的，和本机正在学的互不覆盖；设计见 `cloud/docs/design.md`。
//!
//! 一行一条，`表\t操作\t字段…`：
//! - `user` / `choices` / `typos` / `english` / `ngram` 的 `add`：键的各列 + 带符号的增量，减到 0 以下就删；
//! - `words` 的 `put`（词、拼音）与 `del`（词）。

use std::collections::BTreeMap;

use qingjian_core::Learner;
use qingjian_core::sentence::UserNgram;

use super::{FrequencyLearner, data_lines};

impl FrequencyLearner {
    /// 合并收件箱文本，返回认出的行数；认不出的行记一条警告跳过。
    pub fn merge_inbox(&mut self, inbox: &str) -> usize {
        let mut applied = 0;
        let mut skipped = 0;
        let mut ngram: Vec<(String, i64)> = Vec::new();
        let mut english_changed = false;
        let mut words_changed = false;
        for line in data_lines(inbox) {
            let fields: Vec<&str> = line.split('\t').collect();
            let Some((delta, keys)) = fields.split_last() else {
                continue;
            };
            let delta = delta.trim().parse::<i64>().ok();
            match (fields.as_slice(), delta) {
                (["user", "add", text, _], Some(delta)) => {
                    let count = self.counts.entry((*text).to_owned()).or_default();
                    adjust(count, delta);
                    if *count == 0 {
                        self.counts.remove(*text);
                    }
                    self.dirty = true;
                }
                (["choices", "add", input, text, _], Some(delta)) => {
                    let texts = self.choices.entry((*input).to_owned()).or_default();
                    let count = texts.entry((*text).to_owned()).or_default();
                    adjust(count, delta);
                    if *count == 0 {
                        texts.remove(*text);
                    }
                    if texts.is_empty() {
                        self.choices.remove(*input);
                    }
                    self.choices_dirty = true;
                }
                (["typos", "add", typed, intended, _], Some(delta)) => {
                    let intended_counts = self.typos.entry((*typed).to_owned()).or_default();
                    let count = intended_counts.entry((*intended).to_owned()).or_default();
                    adjust(count, delta);
                    if *count == 0 {
                        intended_counts.remove(*intended);
                    }
                    if intended_counts.is_empty() {
                        self.typos.remove(*typed);
                    }
                    self.typos_dirty = true;
                }
                (["english", "add", word, _], Some(delta)) => {
                    let code = word.to_ascii_lowercase();
                    let entry = self
                        .english
                        .entry(code.clone())
                        .or_insert_with(|| ((*word).to_owned(), 0));
                    adjust(&mut entry.1, delta);
                    if entry.1 == 0 {
                        self.english.remove(&code);
                    }
                    english_changed = true;
                }
                (["ngram", "add", ..], Some(delta)) if matches!(keys.len(), 4 | 5) => {
                    ngram.push((keys[2..].join("\t"), delta));
                }
                (["words", "put", text, pinyin], _) if !pinyin.trim().is_empty() => {
                    self.words
                        .insert((*text).to_owned(), pinyin.trim().to_owned());
                    words_changed = true;
                }
                (["words", "del", text], _) => {
                    words_changed |= self.words.remove(*text).is_some();
                }
                _ => {
                    skipped += 1;
                    continue;
                }
            }
            applied += 1;
        }
        if !ngram.is_empty() {
            self.merge_ngram(&ngram);
        }
        if english_changed {
            self.english_dirty = true;
            self.rebuild_english();
        }
        if words_changed {
            self.words_dirty = true;
            self.rebuild_words();
        }
        if skipped > 0 {
            tracing::warn!(skipped, "收件箱里有认不出的行，已跳过");
        }
        applied
    }

    /// n-gram 的二元与三元在 `UserNgram` 里是一起记的，没有单独加减某一条的接口：
    /// 借它的文本格式整体导出、加上增量、再读回来（只在收到别的设备的变化时做一次）。
    fn merge_ngram(&mut self, deltas: &[(String, i64)]) {
        let mut rows: BTreeMap<String, i64> = BTreeMap::new();
        for line in self.ngram.to_tsv().lines() {
            if let Some((key, count)) = line.rsplit_once('\t')
                && let Ok(count) = count.parse::<i64>()
            {
                rows.insert(key.to_owned(), count);
            }
        }
        for (key, delta) in deltas {
            *rows.entry(key.clone()).or_default() += delta;
        }
        let tsv: String = rows
            .iter()
            .filter(|(_, count)| **count > 0)
            .map(|(key, count)| format!("{key}\t{}\n", (*count).min(i64::from(u32::MAX))))
            .collect();
        let (ngram, _) = UserNgram::parse_lenient(&tsv);
        self.ngram = ngram;
        self.ngram_dirty = true;
    }

    /// 合并后马上落盘，确保收件箱删掉之前数据已经在磁盘上。
    pub fn merge_inbox_and_flush(&mut self, inbox: &str) -> usize {
        let applied = self.merge_inbox(inbox);
        self.flush();
        applied
    }
}

/// 计数加上带符号的增量，夹在 0..=u32::MAX。
fn adjust(count: &mut u32, delta: i64) {
    *count = (i64::from(*count) + delta).clamp(0, i64::from(u32::MAX)) as u32;
}

#[cfg(test)]
mod tests {
    use qingjian_core::sentence::Context;
    use qingjian_core::{Candidate, CandidateKind};

    use super::*;

    fn candidate(text: &str) -> Candidate {
        Candidate {
            text: text.to_owned(),
            kind: CandidateKind::Chinese,
            syllables: Vec::new(),
            reading: None,
            translation: None,
            aux_code: None,
        }
    }

    #[test]
    fn adds_remote_counts_on_top_of_local_learning() {
        let mut learner = FrequencyLearner::default();
        learner.record(&candidate("开发"));
        learner.record_choice("kf", "开发");
        learner.record_typo("hs", "shi");
        let applied = learner.merge_inbox(
            "user\tadd\t开发\t3\nuser\tadd\t青简\t2\nchoices\tadd\tkf\t开发\t1\n\
             typos\tadd\ths\tshi\t-1\nenglish\tadd\tGitHub\t4\n",
        );
        assert_eq!(applied, 5);
        assert_eq!(learner.weight("开发"), 4);
        assert_eq!(learner.weight("青简"), 2);
        assert_eq!(learner.choice_weight("kf", "开发"), 2);
        assert_eq!(learner.typo_count("hs", "shi"), 0);
        assert_eq!(learner.english_count(), 1);
        assert!(learner.has_unsaved());
    }

    #[test]
    fn negative_deltas_remove_entries_and_never_underflow() {
        let mut learner = FrequencyLearner::default();
        learner.record(&candidate("开发"));
        learner.learn_english("rust");
        learner.merge_inbox("user\tadd\t开发\t-5\nenglish\tadd\tRust\t-1\nuser\tadd\t没有\t-1\n");
        assert_eq!(learner.weight("开发"), 0);
        assert_eq!(learner.weight("没有"), 0);
        assert_eq!(learner.english_count(), 0);
        assert!(learner.is_empty());
    }

    #[test]
    fn bigram_and_trigram_rows_merge_separately() {
        let mut learner = FrequencyLearner::default();
        learner.record_transition(Context::after("我们"), "开发", 1);
        let before = learner.user_ngram().unwrap().to_tsv();
        assert_eq!(before, "我们\t开发\t1\n<s>\t我们\t开发\t1\n");
        learner.merge_inbox("ngram\tadd\t我们\t开发\t2\nngram\tadd\t今天\t我们\t开发\t1\n");
        assert_eq!(
            learner.user_ngram().unwrap().to_tsv(),
            "我们\t开发\t3\n<s>\t我们\t开发\t1\n今天\t我们\t开发\t1\n"
        );
        learner.merge_inbox("ngram\tadd\t<s>\t我们\t开发\t-1\n");
        assert_eq!(
            learner.user_ngram().unwrap().to_tsv(),
            "我们\t开发\t3\n今天\t我们\t开发\t1\n"
        );
    }

    #[test]
    fn user_words_put_and_delete() {
        let mut learner = FrequencyLearner::default();
        learner.merge_inbox("words\tput\t青简\tqing jian\nwords\tput\t云端\tyun duan\n");
        assert_eq!(learner.word_count(), 2);
        assert!(learner.user_words().is_some());
        learner.merge_inbox("words\tdel\t云端\nwords\tdel\t不存在\n");
        assert_eq!(learner.word_count(), 1);
    }

    #[test]
    fn unknown_lines_are_skipped() {
        let mut learner = FrequencyLearner::default();
        let applied = learner
            .merge_inbox("# 注释\nvocab\tadd\tx\t1\nuser\tadd\t开发\tx\nuser\tadd\t开发\t1\n");
        assert_eq!(applied, 1);
    }

    #[test]
    fn merge_remote_flushes_to_disk() {
        let dir = std::env::temp_dir().join(format!("qingjian-remote-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("user.tsv");
        let mut learner = FrequencyLearner::from_path(&path).unwrap();
        assert_eq!(
            Learner::merge_remote(
                &mut learner,
                "user\tadd\t开发\t2\nwords\tput\t青简\tqing jian\n"
            ),
            2
        );
        assert!(!learner.has_unsaved());
        let restored = FrequencyLearner::from_path(&path).unwrap();
        assert_eq!(restored.weight("开发"), 2);
        assert_eq!(restored.word_count(), 1);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
