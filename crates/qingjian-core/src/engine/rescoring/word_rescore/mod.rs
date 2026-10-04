//! 知微给词级候选按前文打分：第一页里与首选同一结构档（精确 / 覆盖 / 简拼数 / 末音节完整都相同）的前几个词，
//! 按「静态分 + λ_w·(神经分 − 静态分)」重排，只在这几格之间换位；整句（路径分按词累加，与词不在一个尺度）、
//! 英文 / 快捷 / emoji 与别档的词都不动。打分走与整句重排同样的「缓存 + 后台线程 + 停顿后请求 + 结果到了再查一次」
//! （见 ../mod.rs），按键回调不等模型。素笺分叉，见 cloud/docs/specs/2026-10-04-context-prediction-design.md。

#[cfg(test)]
mod tests;

use std::collections::HashMap;

use crate::candidate::{Candidate, CandidateKind};
use crate::engine::Engine;
use crate::engine::rescoring::{NeuralCache, RescoreWorker};
use crate::ranking::Scored;
use crate::sentence::SentenceScorer;

/// 词级重排里神经分的缺省权重 λ_w，按 `--eval-context` 扫 {0.3, 0.5, 0.8} 取（见计划文件 Task 4）。
pub const WORD_NEURAL_WEIGHT: f64 = 0.5;

/// 最多给几个词级候选打分。
const WORD_RESCORE_CANDIDATES: usize = 6;

impl Engine {
    /// 挂上同步的知微（查询里当场打，评测用）。`weight` 是 λ_w，`None` 用 [`WORD_NEURAL_WEIGHT`]。
    pub fn with_word_scorer(
        mut self,
        scorer: Box<dyn SentenceScorer>,
        weight: Option<f64>,
    ) -> Self {
        self.word_scorer = Some(scorer);
        self.word_rescorer = None;
        self.word_weight = weight.unwrap_or(WORD_NEURAL_WEIGHT).clamp(0.0, 1.0);
        self
    }

    /// 挂上异步的知微（后台线程，壳里用）。
    pub fn with_async_word_scorer(
        mut self,
        scorer: Box<dyn SentenceScorer>,
        weight: Option<f64>,
    ) -> Self {
        self.set_async_word_scorer(Some(scorer));
        self.word_weight = weight.unwrap_or(WORD_NEURAL_WEIGHT).clamp(0.0, 1.0);
        self
    }

    /// 运行时换 / 卸异步知微（壳里模型后台加载完才接上，配置关掉就卸）。
    pub fn set_async_word_scorer(&mut self, scorer: Option<Box<dyn SentenceScorer>>) {
        self.word_scorer = None;
        self.word_rescorer = scorer.map(RescoreWorker::spawn);
        self.word_awaiting = None;
        *self.word_cache.borrow_mut() = NeuralCache::default();
    }

    pub fn has_word_scorer(&self) -> bool {
        self.word_scorer.is_some()
            || self
                .word_rescorer
                .as_ref()
                .is_some_and(RescoreWorker::is_alive)
    }

    pub fn set_word_weight(&mut self, weight: f64) {
        self.word_weight = weight.clamp(0.0, 1.0);
    }

    /// 排好序的词级命中里要给知微打分的那一档：与第一条同结构档的前几条，带静态分。没接知微返回空。
    pub(in crate::engine) fn word_tier(
        &self,
        scored: &[Scored<'_>],
        scores: &[f64],
    ) -> Vec<(String, f64)> {
        let (true, Some(head)) = (self.has_word_scorer(), scored.first()) else {
            return Vec::new();
        };
        let same = |s: &Scored<'_>| {
            (s.hit.exact, s.coverage, s.abbreviated, s.full_last)
                == (
                    head.hit.exact,
                    head.coverage,
                    head.abbreviated,
                    head.full_last,
                )
        };
        scored
            .iter()
            .zip(scores)
            .take(WORD_RESCORE_CANDIDATES)
            .take_while(|(s, _)| same(s))
            .map(|(s, score)| (s.hit.text.to_owned(), *score))
            .collect()
    }

    /// 把第一页里的同档词按知微重排。异步时缺分就记下等壳来取、这次不动；
    /// 任何一条没分也不动（半截重排比不重排还糟）。
    pub(in crate::engine) fn rescore_first_page(
        &self,
        items: &mut [Candidate],
        tier: &[(String, f64)],
    ) {
        if !self.has_word_scorer() || tier.len() < 2 {
            return;
        }
        let statics: HashMap<&str, f64> = tier.iter().map(|(t, s)| (t.as_str(), *s)).collect();
        let slots: Vec<usize> = items
            .iter()
            .enumerate()
            .filter(|(_, c)| {
                c.kind == CandidateKind::Chinese && statics.contains_key(c.text.as_str())
            })
            .map(|(index, _)| index)
            .collect();
        if slots.len() < 2 {
            return;
        }
        let context = self.rescoring_context();
        let keys = self.composition.scope().to_owned();
        let mut cache = self.word_cache.borrow_mut();
        cache.ensure_condition(&context, &keys);
        let missing: Vec<String> = slots
            .iter()
            .map(|&i| items[i].text.clone())
            .filter(|text| cache.get(text).is_none())
            .collect();
        if !missing.is_empty() {
            match &self.word_scorer {
                Some(scorer) => {
                    let texts: Vec<&str> = missing.iter().map(String::as_str).collect();
                    let scores = scorer.score(&context, &keys, &texts);
                    if scores.len() != texts.len() {
                        return;
                    }
                    for (text, score) in texts.iter().zip(scores) {
                        cache.insert(text, score);
                    }
                }
                None => {
                    for text in &missing {
                        cache.want(text);
                    }
                    return;
                }
            }
        }
        let lambda = self.word_weight;
        let rescored = |index: usize| {
            let text = items[index].text.as_str();
            let static_score = statics[text];
            static_score + lambda * (cache.get(text).expect("filled above") - static_score)
        };
        let mut order = slots.clone();
        order.sort_by(|&a, &b| {
            rescored(b)
                .partial_cmp(&rescored(a))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        if order == slots {
            return;
        }
        let moved: Vec<Candidate> = order.iter().map(|&i| items[i].clone()).collect();
        for (slot, candidate) in slots.into_iter().zip(moved) {
            items[slot] = candidate;
        }
        self.last_rescored.set(true);
    }

    /// 知微那边有没分的候选等着送后台。
    pub(super) fn word_rescoring_pending(&self) -> bool {
        self.word_rescorer.is_some() && self.word_cache.borrow().has_wanted()
    }

    /// 把攒着的候选送去知微的后台线程；没什么要送返回 `false`。
    pub(super) fn request_word_rescoring(&mut self) -> bool {
        let Some(worker) = &self.word_rescorer else {
            return false;
        };
        let mut cache = self.word_cache.borrow_mut();
        let wanted = cache.take_wanted();
        if wanted.is_empty() {
            return false;
        }
        tracing::debug!(texts = wanted.len(), "知微词级请求");
        self.word_awaiting = Some(worker.submit(
            cache.context().to_owned(),
            cache.keys().to_owned(),
            wanted,
            None,
        ));
        true
    }

    /// 收知微打好的分；有新分进了缓存返回 `true`。前文或按键已经变了的结果丢掉。
    pub(super) fn poll_word_rescoring(&mut self) -> bool {
        let Some(worker) = &self.word_rescorer else {
            return false;
        };
        let mut updated = false;
        while let Some(scored) = worker.poll() {
            if self.word_awaiting.is_some_and(|id| scored.id >= id) {
                self.word_awaiting = None;
            }
            let mut cache = self.word_cache.borrow_mut();
            if scored.context != cache.context()
                || scored.keys != cache.keys()
                || scored.scores.len() != scored.texts.len()
            {
                continue;
            }
            for (text, score) in scored.texts.iter().zip(scored.scores) {
                cache.insert(text, score);
            }
            updated = true;
        }
        updated
    }
}
