/// 整句路径的第二打分来源：给 Viterbi 出的前几条路径再打一次分，与路径本身的得分对数线性插值。
/// Core 只认这个 trait，实现在 `qingjian-neural`。
pub trait SentenceScorer: Send {
    /// 每条 `texts` 的 log 概率，与 `texts` 一一对应。算不了（模型出错）返回空 Vec，调用方就当没有这个打分。
    ///
    /// 两个条件都给，实现按自己训练时的条件挑用、忽略另一个：
    /// `context` 是光标前文（字级模型用），`keys` 是这批路径共同解释的那段用户按键（P2C 用）。
    fn score(&self, context: &str, keys: &str, texts: &[&str]) -> Vec<f64>;

    /// 不经词图，直接从按键生成整句，最好的在前。生成不了（字级模型没有这个能力、或模型出错）返回空 Vec。
    ///
    /// 词图只会把整段按键读成拼音，中英混输（`yongdockerbushuhenfangbian`）与生词在它那里根本没有路径；
    /// P2C 训练时见过的就是「按键 → 汉字」，这条路不受词图的读法限制。
    fn generate(&self, _keys: &str, _beam: usize, _max_chars: usize) -> Vec<String> {
        Vec::new()
    }

    /// 接着 `before` 往下写最多 `max_chars` 个字：(文本, 每字平均 log 概率)。做不了（P2C 模型、模型出错）返回 `None`。
    /// 素笺的本地续写用，见 `engine/prediction/local_continuation`。
    fn continue_text(&self, _before: &str, _max_chars: usize) -> Option<(String, f64)> {
        None
    }

    /// 上文变了，先在后台把上文的模型状态算好，下一次 [`Self::score`] 直接用（参考实现的 `AutoInferTextBeforeCursor`）。
    /// 不看前文的实现（P2C）什么都不做。
    fn prefill(&self, _context: &str) {}

    /// 丢掉模型里缓存的前文 / 按键状态：私密输入、换学习范围时调。
    fn forget(&self) {}

    /// 分数是否依赖用户按键。字级模型不依赖：按键变了分数表不必清。
    fn uses_keys(&self) -> bool {
        true
    }

    /// 最近一次打分的前缀复用：（复用的 token 数, 前缀总长），真机日志看命中率；没有缓存的实现返回 (0, 0)。
    fn prefix_reuse(&self) -> (usize, usize) {
        (0, 0)
    }
}
