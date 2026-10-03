//! 同步的各张表：表名（协议与收件箱里用）、输入法的文件名、键有几列。

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Table {
    /// `user.tsv`：词 → 选择次数。
    User,

    /// `user-choices.tsv`：(输入串, 词) → 次数。
    Choices,

    /// `user-ngram.tsv`：(前词, 词) 或 (再前词, 前词, 词) → 次数。
    Ngram,

    /// `user-typos.tsv`：(敲的, 要的) → 次数。
    Typos,

    /// `user-english.tsv`：英文词（小写为键，保留第一次的写法）→ 次数。
    English,

    /// `user-words.tsv`：词 → 拼音（集合表）。
    Words,
}

impl Table {
    pub const ALL: [Self; 6] = [
        Self::User,
        Self::Choices,
        Self::Ngram,
        Self::Typos,
        Self::English,
        Self::Words,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Choices => "choices",
            Self::Ngram => "ngram",
            Self::Typos => "typos",
            Self::English => "english",
            Self::Words => "words",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|table| table.name() == name)
    }

    pub fn file(self) -> &'static str {
        match self {
            Self::User => "user.tsv",
            Self::Choices => "user-choices.tsv",
            Self::Ngram => "user-ngram.tsv",
            Self::Typos => "user-typos.tsv",
            Self::English => "user-english.tsv",
            Self::Words => "user-words.tsv",
        }
    }

    /// 集合表（值是字符串，不是次数）。
    pub fn is_set(self) -> bool {
        self == Self::Words
    }

    /// 计数表的键允许几列。
    pub fn key_columns(self) -> &'static [usize] {
        match self {
            Self::User | Self::English => &[1],
            Self::Choices | Self::Typos => &[2],
            Self::Ngram => &[2, 3],
            Self::Words => &[1],
        }
    }
}
