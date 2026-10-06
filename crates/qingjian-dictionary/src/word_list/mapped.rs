//! `.qj` 形态的英文词表：mmap 零拷贝视图，键盘扩展用它（解析 TSV 要驻留十几 MB，映射的 clean 页不计足迹）。
//!
//! 布局五个分节：`CODE` 编码 arena、`COFF` 编码的定长偏移表、`WORD` 词 arena、`WOFF` 词偏移表、
//! `FREQ` 词频表（u32）。条目按编码字节序升序（唯一），`get` 与前缀补全都二分，不需要 HashMap。
//! 打开时一次校验完（见 [`Mapped::open`]），坏文件报错、之后查询永不 panic。

use std::path::Path;

use qingjian_format::{Container, Kind, Table, Text};
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

use crate::error::DictionaryError;

/// arena 里一段变长字符串的位置。
#[derive(Debug, Clone, Copy, FromBytes, Immutable, IntoBytes, KnownLayout)]
#[repr(C)]
pub struct Span {
    /// 起始字节（在各自 arena 里）。
    pub start: u32,

    /// 字节长（u32 保持无填充，零拷贝读写都要这个性质）。
    pub len: u32,
}

/// 分节标签。
pub const CODE_TAG: [u8; 4] = *b"CODE";
/// 分节标签。
pub const COFF_TAG: [u8; 4] = *b"COFF";
/// 分节标签。
pub const WORD_TAG: [u8; 4] = *b"WORD";
/// 分节标签。
pub const WOFF_TAG: [u8; 4] = *b"WOFF";
/// 分节标签。
pub const FREQ_TAG: [u8; 4] = *b"FREQ";

/// 映射进来的词表。视图各持一份 mmap 的份额，文件不会被改（数据文件只整体替换）。
#[derive(Debug)]
pub struct Mapped {
    codes: Text,
    code_spans: Table<Span>,
    words: Text,
    word_spans: Table<Span>,
    freqs: Table<u32>,
}

impl Mapped {
    /// 映射五个分节并整体校验：三张表等长、每个偏移落在各自 arena 的字符边界上、编码严格升序（含唯一）。
    /// UTF-8 与分节边界由 [`Container`] 保证。坏文件全部走到 `Err`，之后查询只碰已校验过的数据。
    pub fn open(path: &Path) -> Result<Self, DictionaryError> {
        let container = Container::open(path, Kind::WordList)?;
        let mapped = Self {
            codes: container.text(CODE_TAG)?,
            code_spans: container.table(COFF_TAG)?,
            words: container.text(WORD_TAG)?,
            word_spans: container.table(WOFF_TAG)?,
            freqs: container.table(FREQ_TAG)?,
        };
        if mapped.code_spans.len() != mapped.word_spans.len()
            || mapped.code_spans.len() != mapped.freqs.len()
        {
            return Err(DictionaryError::Corrupt(
                "code, word and frequency tables disagree in length",
            ));
        }
        for index in 0..mapped.len() {
            let (Some(code), Some(word) /* 借用两个 arena 不能同时进 if let */) =
                (mapped.code(index), mapped.word(index))
            else {
                return Err(DictionaryError::Corrupt("span points outside its arena"));
            };
            let _ = word;
            if index + 1 < mapped.len() && code >= mapped.code(index + 1).unwrap_or_default() {
                return Err(DictionaryError::Corrupt("codes are not strictly ascending"));
            }
        }
        Ok(mapped)
    }

    fn code(&self, index: usize) -> Option<&str> {
        let span = self.code_spans.get(index)?;
        let start = usize::try_from(span.start).ok()?;
        self.codes
            .get(start..start.checked_add(usize::try_from(span.len).ok()?)?)
    }

    fn word(&self, index: usize) -> Option<&str> {
        let span = self.word_spans.get(index)?;
        let start = usize::try_from(span.start).ok()?;
        self.words
            .get(start..start.checked_add(usize::try_from(span.len).ok()?)?)
    }

    fn freq_at(&self, index: usize) -> u32 {
        self.freqs.get(index).copied().unwrap_or(0)
    }

    /// 条目数（[super::WordList::len] 用）。
    pub fn len(&self) -> usize {
        self.code_spans.len()
    }

    /// 第一个编码不小于 `code` 的下标（编码升序，二分）。
    fn lower_bound(&self, code: &str) -> usize {
        // 打开时校验过偏移都在 arena 内；万一越界（不可能），判它更大，二分结果仍保守
        self.code_spans.partition_point(|span| {
            let start = usize::try_from(span.start).unwrap_or(usize::MAX);
            let end = start.saturating_add(usize::try_from(span.len).unwrap_or(0));
            self.codes.get(start..end).is_none_or(|text| text < code)
        })
    }

    /// 输入（已小写）对应的英文词。
    pub fn get(&self, code: &str) -> Option<&str> {
        let index = self.lower_bound(code);
        if index < self.len() && self.code(index) == Some(code) {
            self.word(index)
        } else {
            None
        }
    }

    /// 输入（已小写）对应的词频；词表里没有为 `None`。
    pub fn frequency(&self, code: &str) -> Option<u32> {
        let index = self.lower_bound(code);
        (index < self.len() && self.code(index) == Some(code)).then(|| self.freq_at(index))
    }

    /// 以 `prefix` 开头（不含正好相等的）的词里词频最高的 `limit` 个，按词频降序、同频按编码升序。
    /// 与 TSV 形态的语义逐条相同（等价测试守着）。
    pub fn complete(&self, prefix: &str, limit: usize) -> Vec<&str> {
        if prefix.is_empty() || limit == 0 {
            return Vec::new();
        }
        let start = self.lower_bound(prefix);
        let mut hits: Vec<usize> = (start..self.len())
            .take_while(|&index| {
                self.code(index)
                    .is_some_and(|code| code.starts_with(prefix))
            })
            .filter(|&index| self.code(index) != Some(prefix))
            .collect();
        hits.sort_by(|&a, &b| {
            self.freq_at(b)
                .cmp(&self.freq_at(a))
                .then_with(|| self.code(a).cmp(&self.code(b)))
        });
        hits.into_iter()
            .take(limit)
            .filter_map(|index| self.word(index))
            .collect()
    }

    /// 全部词目：(小写编码, 原样词, 词频)，按编码字节序。
    pub fn entries(&self) -> impl Iterator<Item = (&str, &str, u32)> {
        (0..self.len()).filter_map(move |index| {
            Some((self.code(index)?, self.word(index)?, self.freq_at(index)))
        })
    }
}
