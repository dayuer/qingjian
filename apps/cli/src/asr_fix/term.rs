//! 一条术语与它每个字可接受的读音。

pub struct Term {
    pub text: String,

    pub chars: Vec<char>,

    /// 每个字可接受的读音（无分隔全拼音节）。
    pub readings: Vec<Vec<String>>,

    /// 学自哪些会议；空为全局。留一场会议评测时，只学自本场的术语不用来纠本场。
    pub scopes: Vec<String>,
}

impl Term {
    /// 留一场会议评测时这条术语能不能用在 `group` 上。
    pub fn usable_in(&self, group: &str) -> bool {
        self.scopes.is_empty() || self.scopes.iter().any(|scope| scope != group)
    }

    /// 能纠的读音差上限：两字词只认同音，三四字词容一个近音字，五字以上容两个。
    pub fn budget(&self) -> u32 {
        ((self.chars.len() as u32).saturating_sub(1)) / 2
    }
}
