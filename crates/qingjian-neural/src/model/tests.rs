//! `PrefixCache::truncate` 与 `CharLm::extend_cache`：截到前 k 个位置再接着推进，与整段算的前缀一致。
//! 知微模型不在 `data/models/` 下就跳过。

use std::path::Path;

use candle_core::Tensor;

use super::{CharLm, PrefixCache};
use crate::CharScorer;
use crate::vocab::EOS;

fn zhiwei() -> Option<CharScorer> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/models/hanzhang-zhiwei");
    dir.exists().then(|| CharScorer::load(&dir).unwrap())
}

/// 接在缓存后面喂一个 token，取下一个 token 的 log 概率分布。
fn next_log_probs(model: &CharLm, cache: &PrefixCache, token: u32) -> Vec<f32> {
    let idx = Tensor::from_vec(vec![token], (1, 1), model.device()).unwrap();
    model
        .log_probs_after(cache, &idx)
        .unwrap()
        .flatten_all()
        .unwrap()
        .to_vec1::<f32>()
        .unwrap()
}

#[test]
fn truncate_then_extend_matches_full_prefix() {
    let Some(scorer) = zhiwei() else {
        eprintln!("没有知微模型，跳过");
        return;
    };
    let model = scorer.model();
    // f32 CPU 路径 1e-4；Metal 上是 f16，放宽到 8 位同口径
    let tolerance = if model.device().is_metal() {
        0.08
    } else {
        1e-4
    };
    let all: Vec<u32> = std::iter::once(EOS)
        .chain(scorer.vocab().encode("我今天想去上海看看"))
        .collect();
    let full = model.prefix_cache(&all).unwrap();
    let expected = next_log_probs(model, &full, all[1]);
    for keep in [1, 3, all.len() - 1] {
        let part = full.truncate(keep).unwrap();
        assert_eq!(part.len(), keep);
        let grown = model.extend_cache(&part, &all[keep..]).unwrap();
        assert_eq!(grown.len(), all.len());
        let got = next_log_probs(model, &grown, all[1]);
        let diff = expected
            .iter()
            .zip(&got)
            .map(|(a, b)| (a - b).abs())
            .fold(0.0f32, f32::max);
        assert!(diff < tolerance, "keep={keep}: {diff}");
    }
    assert!(full.truncate(all.len() + 1).is_err());
    assert!(full.truncate(0).unwrap().is_empty());
    assert!(full.bytes() > 0);
    assert_eq!(full.truncate(0).unwrap().bytes(), 0);
}
