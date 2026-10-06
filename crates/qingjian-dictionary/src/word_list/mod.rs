//! 英文词表：按小写编码查原样大小写的词，也能按前缀补全。两种形态——TSV 解析进内存（个人词表、桌面壳），
//! 或 `.qj` 容器 mmap 零拷贝（键盘扩展，`from_path` 按文件头自动选）。

mod mapped;
mod tsv;

use std::path::Path;

use qingjian_format::{Kind, Metadata, Writer};
use zerocopy::IntoBytes;

use self::mapped::{CODE_TAG, COFF_TAG, FREQ_TAG, Mapped, Span, WOFF_TAG, WORD_TAG};
use self::tsv::Tsv;
use crate::error::DictionaryError;

/// 英文词表。查询方法在两种形态上语义逐条相同（等价测试守着）。
#[derive(Debug, Default)]
pub struct WordList {
    source: Source,
}

#[derive(Debug)]
enum Source {
    Tsv(Tsv),
    Mapped(Mapped),
}

impl Default for Source {
    fn default() -> Self {
        Self::Tsv(Tsv::default())
    }
}

impl WordList {
    /// 文件格式 TSV：`词\t编码[\t词频]`（编码缺省为词的小写，词频缺省 0）。
    pub fn parse(source: &str) -> Result<Self, DictionaryError> {
        Ok(Self {
            source: Source::Tsv(Tsv::parse(source)?),
        })
    }

    /// 按文件内容选加载方式：`.qj` 容器直接映射，否则当 TSV 解析。
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, DictionaryError> {
        let path = path.as_ref();
        if qingjian_format::Container::is_qj(path) {
            Ok(Self {
                source: Source::Mapped(Mapped::open(path)?),
            })
        } else {
            Self::parse(&std::fs::read_to_string(path)?)
        }
    }

    /// 写成 `.qj`：条目（已去重、按编码升序）原样落进五个分节。`metadata.entries` 填成条目数。
    pub fn write_qj(&self, path: &Path, metadata: &Metadata) -> Result<(), DictionaryError> {
        let entries: Vec<(&str, &str, u32)> = self.entries().collect();
        let metadata = Metadata {
            entries: entries.len() as u64,
            ..metadata.clone()
        };
        let mut codes = String::new();
        let mut words = String::new();
        let mut code_spans: Vec<Span> = Vec::with_capacity(entries.len());
        let mut word_spans: Vec<Span> = Vec::with_capacity(entries.len());
        let mut freqs: Vec<u32> = Vec::with_capacity(entries.len());
        for (code, word, frequency) in entries {
            code_spans.push(Span {
                start: codes.len() as u32,
                len: code.len() as u32,
            });
            codes.push_str(code);
            word_spans.push(Span {
                start: words.len() as u32,
                len: word.len() as u32,
            });
            words.push_str(word);
            freqs.push(frequency);
        }
        Writer::new(Kind::WordList, &metadata)?
            .section(CODE_TAG, codes.as_bytes())
            .section(COFF_TAG, code_spans.as_bytes())
            .section(WORD_TAG, words.as_bytes())
            .section(WOFF_TAG, word_spans.as_bytes())
            .section(FREQ_TAG, freqs.as_bytes())
            .write_to(path)?;
        Ok(())
    }

    /// 输入（已小写）对应的英文词。
    pub fn get(&self, code: &str) -> Option<&str> {
        match &self.source {
            Source::Tsv(list) => list.get(code),
            Source::Mapped(list) => list.get(code),
        }
    }

    /// 输入（已小写）对应的词频；词表里没有为 `None`。
    pub fn frequency(&self, code: &str) -> Option<u32> {
        match &self.source {
            Source::Tsv(list) => list.frequency(code),
            Source::Mapped(list) => list.frequency(code),
        }
    }

    /// 以 `prefix` 开头（不含正好相等的）的词里词频最高的 `limit` 个，按词频降序、同频按编码升序。
    pub fn complete(&self, prefix: &str, limit: usize) -> Vec<&str> {
        match &self.source {
            Source::Tsv(list) => list.complete(prefix, limit),
            Source::Mapped(list) => list.complete(prefix, limit),
        }
    }

    /// 全部词目：(小写编码, 原样词, 词频)，按编码字节序。
    pub fn entries(&self) -> impl Iterator<Item = (&str, &str, u32)> {
        match &self.source {
            Source::Tsv(list) => Box::new(
                list.entries
                    .iter()
                    .map(|(code, word, frequency)| (code.as_str(), word.as_str(), *frequency)),
            ) as Box<dyn Iterator<Item = (&str, &str, u32)>>,
            Source::Mapped(list) => Box::new(list.entries()),
        }
    }

    pub fn len(&self) -> usize {
        match &self.source {
            Source::Tsv(list) => list.entries.len(),
            Source::Mapped(list) => list.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use self::mapped::{CODE_TAG, COFF_TAG, FREQ_TAG, Span, WOFF_TAG, WORD_TAG};
    use super::*;

    /// 临时文件路径（用后删）。
    fn temp(name: &str) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!("qj-wordlist-{}-{name}", std::process::id()));
        let _ = std::fs::remove_file(&path);
        path
    }

    fn metadata() -> Metadata {
        Metadata {
            name: "测试英文词表".to_owned(),
            license: "MIT".to_owned(),
            ..Metadata::default()
        }
    }

    /// 覆盖语义边角的样例：重复编码取第一个、编码缺省、词频缺省、前缀恰好相等要排除、同频按编码升序。
    const SAMPLE: &str = "GitHub\tgithub\t100\nhello\thello\niPhone\n\
        compass\tcompass\t300\ncompany\tcompany\t900\ncompare\tcompare\t500\ncom\tcom\t100\ncomma\tcomma\n\
        GitHub\tgithub\t9999\n";

    #[test]
    fn looks_up_by_lowercase_code() {
        let list = WordList::parse("GitHub\tgithub\nhello\thello\niPhone\n").unwrap();
        assert_eq!(list.get("github"), Some("GitHub"));
        assert_eq!(list.get("iphone"), Some("iPhone"));
        assert_eq!(list.get("hello"), Some("hello"));
        assert_eq!(list.get("nope"), None);
    }

    #[test]
    fn completes_prefixes_by_frequency() {
        let list = WordList::parse(
            "compass\tcompass\t300\ncompany\tcompany\t900\ncompare\tcompare\t500\ncom\tcom\t100\ncomma\tcomma\n",
        )
        .unwrap();
        assert_eq!(list.complete("comp", 2), ["company", "compare"]);
        assert_eq!(
            list.complete("com", 10),
            ["company", "compare", "compass", "comma"]
        );
        assert!(list.complete("zzz", 3).is_empty());
        assert!(list.complete("", 3).is_empty());
    }

    #[test]
    fn qj_roundtrip_matches_tsv_semantics() {
        let parsed = WordList::parse(SAMPLE).unwrap();
        let path = temp("roundtrip.qj");
        parsed.write_qj(&path, &metadata()).unwrap();
        let mapped = WordList::from_path(&path).unwrap();
        assert_eq!(mapped.len(), parsed.len());
        // 条目序列逐条相等（编码序、去重、词频）
        let a: Vec<_> = parsed.entries().collect();
        let b: Vec<_> = mapped.entries().collect();
        assert_eq!(a, b);
        // 查询面：每个编码的 get / frequency，加上查不中的
        for (code, word, frequency) in &a {
            assert_eq!(mapped.get(code), Some(*word), "get {code}");
            assert_eq!(mapped.frequency(code), Some(*frequency), "freq {code}");
        }
        for miss in ["zzz", "co", "", "githubx"] {
            assert_eq!(mapped.get(miss), None);
            assert_eq!(mapped.frequency(miss), None);
        }
        // 前缀补全：几个前缀 × 几个上限
        for prefix in ["c", "co", "com", "comp", "compa", "h", "g", "i"] {
            for limit in [0, 1, 2, 3, 10] {
                assert_eq!(
                    mapped.complete(prefix, limit),
                    parsed.complete(prefix, limit),
                    "complete {prefix} × {limit}"
                );
            }
        }
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn truncated_and_tampered_files_fail() {
        let parsed = WordList::parse(SAMPLE).unwrap();
        let path = temp("good.qj");
        parsed.write_qj(&path, &metadata()).unwrap();

        // 截断：去掉尾巴，容器头就过不去
        let bytes = std::fs::read(&path).unwrap();
        let cut = temp("cut.qj");
        std::fs::write(&cut, &bytes[..bytes.len() - 10]).unwrap();
        assert!(WordList::from_path(&cut).is_err());

        // 篡改：把 COFF 里一个 span 的起点指到 arena 外，打开时的整体校验要拦下
        let bad = temp("bad-span.qj");
        Writer::new(Kind::WordList, &metadata())
            .unwrap()
            .section(CODE_TAG, b"ab")
            .section(
                COFF_TAG,
                [
                    Span { start: 0, len: 1 },
                    Span {
                        start: u32::MAX,
                        len: 1,
                    },
                ]
                .as_bytes(),
            )
            .section(WORD_TAG, b"AB")
            .section(
                WOFF_TAG,
                [Span { start: 0, len: 1 }, Span { start: 1, len: 1 }].as_bytes(),
            )
            .section(FREQ_TAG, [1u32, 2u32].as_bytes())
            .write_to(&bad)
            .unwrap();
        assert!(WordList::from_path(&bad).is_err());

        // 编码不升序（arena 里 ba 在前、spans 却按 ab 顺序指）
        let unsorted = temp("unsorted.qj");
        Writer::new(Kind::WordList, &metadata())
            .unwrap()
            .section(CODE_TAG, b"ba")
            .section(
                COFF_TAG,
                [Span { start: 0, len: 1 }, Span { start: 1, len: 1 }].as_bytes(),
            )
            .section(WORD_TAG, b"BA")
            .section(
                WOFF_TAG,
                [Span { start: 0, len: 1 }, Span { start: 1, len: 1 }].as_bytes(),
            )
            .section(FREQ_TAG, [1u32, 2u32].as_bytes())
            .write_to(&unsorted)
            .unwrap();
        assert!(WordList::from_path(&unsorted).is_err());

        for file in [path, cut, bad, unsorted] {
            let _ = std::fs::remove_file(file);
        }
    }
}
