//! 一条纠错对提炼不出术语的原因。

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Skip {
    /// 错对两边一样（只改了大小写之外的东西也算不上）。
    Unchanged,

    /// 改动处不全是汉字（英文大小写、空格、数字）。
    NotHan,

    /// 改动前后字数不同（增删字、改语序），按读音一一替换修不了。
    LengthChanged,

    /// 改动处读音对不上（「的 / 地」「他们 / 他们是」这类），不是同音错。
    NotHomophone,

    /// 只是虚词、代词的润色（「的 / 得 / 地」「他 / 她 / 它」），不是识别错。
    Grammar,

    /// 只改了一个字，又扩不到包住它的词库词（「自动的 → 自动地」），凑出来的两个字多半不是词。
    NoWord,

    /// 纯术语太长（多半是带上下文的句子片段）或只有一个字。
    BadLength,
}

impl Skip {
    pub fn label(self) -> &'static str {
        match self {
            Self::Unchanged => "没改动",
            Self::NotHan => "改的不是汉字",
            Self::LengthChanged => "增删了字",
            Self::NotHomophone => "读音不同",
            Self::Grammar => "的得地、他她它润色",
            Self::NoWord => "单字改动扩不成词",
            Self::BadLength => "纯术语过长或单字",
        }
    }
}
