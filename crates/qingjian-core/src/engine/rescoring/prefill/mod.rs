//! 上文的模型状态跨按键缓存（B4）在 Core 这一侧的两件事：上文变了把预填任务送去后台，
//! 私密输入 / 换学习范围时让模型丢掉缓存的前文与按键状态。模型那一侧见 qingjian-neural 的 `ContextCache`。

#[cfg(test)]
mod tests;

use crate::engine::Engine;

impl Engine {
    /// 当前给模型看的前文送去两个打分线程预填（同一段只送一次，见 `RescoreWorker::prefill`）。
    /// 私密输入不送；同步打分器（CLI）查询时当场算，不用预填。
    pub(super) fn prefill_rescoring_context(&self) {
        if self.private {
            return;
        }
        let context = self.rescoring_context();
        if context.is_empty() {
            return;
        }
        for worker in [&self.rescorer, &self.word_rescorer].into_iter().flatten() {
            worker.prefill(&context);
        }
    }

    /// 让模型丢掉缓存的前文与按键状态：进出私密输入时 Core 自己调；壳换学习范围（换聊天对象）时调。
    pub fn forget_neural_context(&mut self) {
        for worker in [&self.rescorer, &self.word_rescorer].into_iter().flatten() {
            worker.forget();
        }
        for scorer in [&self.sentence_scorer, &self.word_scorer]
            .into_iter()
            .flatten()
        {
            scorer.forget();
        }
    }
}
