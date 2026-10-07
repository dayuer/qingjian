/// 整句转换的打分来源：词级语言模型。实现放兄弟 crate（`qingjian-lm`），Core 只认这个 trait。
pub trait LanguageModel: Send {
    /// `log P(word | previous)`；`previous` 为 `None` 表示句首。模型不认识 `word` 时返回 `None`，
    /// 由 Core 用词库词频兜底。
    fn log_prob(&self, previous: Option<&str>, word: &str) -> Option<f64>;

    /// `log P(word)`（一元，不看前词）。判上文语域用（[`super::RegisterMix`]），缺省不提供。
    fn unigram_log_prob(&self, _word: &str) -> Option<f64> {
        None
    }

    /// 引擎每次查询前告知左侧上文（壳给的光标前文，没有就是本会话最近上屏的字）。
    /// 按上文调整的模型（[`super::RegisterMix`]）据此定权重，缺省不理会。
    fn observe_context(&self, _context: &str) {}
}

/// 没接语言模型：一律兜底，整句转换退化为一元词频。
#[derive(Debug, Default, Clone, Copy)]
pub struct NoLanguageModel;

impl LanguageModel for NoLanguageModel {
    fn log_prob(&self, _previous: Option<&str>, _word: &str) -> Option<f64> {
        None
    }
}
