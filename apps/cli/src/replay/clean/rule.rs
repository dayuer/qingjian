//! 干净口径剔除一条上屏的理由。

/// 规则编号与说明见 [`super::scan`]。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rule {
    /// R1：后来被撤销（retract）。
    Retracted,

    /// R2：上屏后整个删掉、按相近但不同的键重打（跨上屏的 retype）。
    Retyped,

    /// R3：汉字夹着不超过两个字母的残段，且不能由词库词拼成。
    LatinTail,
}

impl Rule {
    pub const ALL: [Rule; 3] = [Rule::Retracted, Rule::Retyped, Rule::LatinTail];

    pub fn label(self) -> &'static str {
        match self {
            Rule::Retracted => "R1 撤销",
            Rule::Retyped => "R2 删掉重打",
            Rule::LatinTail => "R3 汉字夹短字母",
        }
    }
}
