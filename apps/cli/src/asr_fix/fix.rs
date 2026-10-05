//! 一处替换：识别结果里的一段换成术语。

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fix {
    /// 在句子里的起点（按字符计）。
    pub start: usize,

    /// 被换掉的原文。
    pub original: String,

    /// 换上的术语。
    pub term: String,

    /// 读音差：同音为 0，每个近音（模糊音）字加 1。
    pub cost: u32,

    /// 原文本身是词库词：可能识别对了只是恰好同音，宽松模式才换。
    pub is_word: bool,

    /// 术语本身是常用词（「统一」）：同音的说法太多，光凭读音不该换，宽松模式才换。
    pub common_term: bool,
}

impl Fix {
    /// 不放宽时该不该留着不换。
    pub fn doubtful(&self) -> bool {
        self.is_word || self.common_term
    }

    /// 留着不换的原因，明细里用。
    pub fn doubt(&self) -> &'static str {
        match (self.is_word, self.common_term) {
            (true, true) => "原文是词库词，术语是常用词",
            (true, false) => "原文是词库词",
            (false, true) => "术语是常用词",
            (false, false) => "",
        }
    }

    pub fn len(&self) -> usize {
        self.term.chars().count()
    }

    pub fn end(&self) -> usize {
        self.start + self.len()
    }
}
