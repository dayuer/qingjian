//! 纠错原型的开关。

#[derive(Debug, Default, Clone, Copy)]
pub struct Options {
    /// 原文本身是词库词也换（接近 sherpa-onnx 同音替换的行为：规则里有就换）。
    pub loose: bool,

    /// 留一场会议：只学自本场会议的术语不用来纠本场，看术语能不能迁移到别的会议。
    pub holdout: bool,

    /// 只用确认过的纠错词，不用候选。
    pub confirmed_only: bool,
}
