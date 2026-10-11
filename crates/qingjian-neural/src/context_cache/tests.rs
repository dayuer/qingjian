//! 前缀缓存的复用规则与数值：命中逐位相等；增量与整段算的差在容差内（f32 1e-4，8 位与 Metal f16 路径 0.08），名次一致。
//! 模型不在 `data/models/` 下就跳过（与 scorer.rs 的测试同口径）。

use std::path::{Path, PathBuf};

use candle_core::Tensor;

use super::ContextCache;
use crate::model::PrefixCache;
use crate::vocab::EOS;
use crate::{CharLm, CharScorer};

fn model_path(dir: &str) -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/models")
        .join(dir);
    path.exists().then_some(path)
}

fn zhiwei() -> Option<CharScorer> {
    model_path("hanzhang-zhiwei").map(|p| CharScorer::load(&p).unwrap())
}

/// f32 CPU 路径的容差；Metal 上是 f16，放宽到与 8 位同口径。
fn tolerance(model: &CharLm) -> f32 {
    if model.device().is_metal() {
        0.08
    } else {
        1e-4
    }
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

fn max_diff(a: &[f32], b: &[f32]) -> f32 {
    a.iter()
        .zip(b)
        .map(|(x, y)| (x - y).abs())
        .fold(0.0, f32::max)
}

fn ids(scorer: &CharScorer, text: &str) -> Vec<u32> {
    std::iter::once(EOS)
        .chain(scorer.vocab().encode(text))
        .collect()
}

#[test]
fn sync_reuses_the_common_prefix() {
    let Some(scorer) = zhiwei() else {
        eprintln!("没有知微模型，跳过");
        return;
    };
    let model = scorer.model();
    let probe = |cache: &PrefixCache| next_log_probs(model, cache, EOS);
    let reference = |seq: &[u32]| probe(&model.prefix_cache(seq).unwrap());
    let mut cache = ContextCache::new(usize::MAX);
    let a = ids(&scorer, "我今");
    let b = ids(&scorer, "我今天");
    let c = ids(&scorer, "我今天想");
    let d = ids(&scorer, "我们");

    // 命中：同一序列两次，逐位相等
    let first = probe(&cache.sync(model, &a).unwrap());
    assert_eq!(cache.last_reuse(), (0, a.len()));
    let again = probe(&cache.sync(model, &a).unwrap());
    assert_eq!(cache.last_reuse(), (a.len(), a.len()));
    assert_eq!(first, again);

    // 追加：只算新增的那个字
    let appended = probe(&cache.sync(model, &b).unwrap());
    assert_eq!(cache.last_reuse(), (a.len(), b.len()));
    assert!(max_diff(&appended, &reference(&b)) < tolerance(model));

    // 退格：截断，零前向
    cache.sync(model, &c).unwrap();
    let shortened = probe(&cache.sync(model, &b).unwrap());
    assert_eq!(cache.last_reuse(), (b.len(), b.len()));
    assert!(max_diff(&shortened, &reference(&b)) < tolerance(model));

    // 分叉：留公共前缀 `<eos>我`
    let forked = probe(&cache.sync(model, &d).unwrap());
    assert_eq!(cache.last_reuse(), (2, d.len()));
    assert!(max_diff(&forked, &reference(&d)) < tolerance(model));

    // 没有公共前缀：整段重算
    let other: Vec<u32> = d[1..].to_vec();
    cache.sync(model, &other).unwrap();
    assert_eq!(cache.last_reuse(), (0, other.len()));

    // 空前缀也能算（候选很长时打分的前缀会被截空）
    assert!(cache.sync(model, &[]).unwrap().is_empty());
}

#[test]
fn over_budget_cache_is_not_kept() {
    let Some(scorer) = zhiwei() else {
        eprintln!("没有知微模型，跳过");
        return;
    };
    let model = scorer.model();
    let seq = ids(&scorer, "我今天");
    let mut cache = ContextCache::new(1);
    let first = next_log_probs(model, &cache.sync(model, &seq).unwrap(), EOS);
    cache.settle();
    assert!(cache.is_empty());
    let second = next_log_probs(model, &cache.sync(model, &seq).unwrap(), EOS);
    assert_eq!(cache.last_reuse(), (0, seq.len()), "超上限的每次现算");
    assert_eq!(first, second);
}

/// 通变：有 8 位的先用 8 位（iOS 键盘装的就是它），否则用 fp16 那份。
fn tongbian() -> Option<CharScorer> {
    model_path("hanzhang-tongbian-q8")
        .or_else(|| model_path("hanzhang-tongbian"))
        .map(|p| CharScorer::load(&p).unwrap())
}

/// 通变逐键：每键只接新字母，分数与整段算的差在 0.08 内、名次一致；第 2 键起复用上一键的前缀。
#[test]
fn p2c_keystrokes_reuse_the_pinyin_prefix() {
    let Some(scorer) = tongbian() else {
        eprintln!("没有通变模型，跳过");
        return;
    };
    let texts = ["我们明天", "我们", "我门明天", "沃们"];
    let keys = "womenmingtian";
    let mut previous_head = 0;
    // 每键先按缓存算，再清空整段算一遍对拍；下一键接着整段算出来的前缀长，与真实打字一样
    for end in 1..=keys.len() {
        let typed = &keys[..end];
        let warm = scorer.score_p2c(typed, &texts).unwrap();
        let (reused, head) = scorer.prefix_reuse();
        if end > 1 {
            assert_eq!(reused, previous_head, "{typed}");
        }
        previous_head = head;
        scorer.forget();
        let cold = scorer.score_p2c(typed, &texts).unwrap();
        assert_eq!(scorer.prefix_reuse().0, 0);
        for (w, c) in warm.iter().zip(&cold) {
            assert!((w - c).abs() < 0.08, "{typed}: {warm:?} vs {cold:?}");
        }
        assert_eq!(rank(&warm), rank(&cold), "{typed}");
    }
    // 退格：截断
    scorer.score_p2c("womenming", &texts).unwrap();
    scorer.score_p2c("womenmin", &texts).unwrap();
    let (reused, head) = scorer.prefix_reuse();
    assert_eq!(reused, head);
}

fn rank(scores: &[f64]) -> Vec<usize> {
    let mut order: Vec<usize> = (0..scores.len()).collect();
    order.sort_by(|&a, &b| scores[b].total_cmp(&scores[a]));
    order
}

/// 预填后打分直接命中，分数与不预填一致；追加上文（刚上屏）时只算新增的字；清空后不留。
#[test]
fn prefill_then_score_hits_the_cache() {
    let Some(scorer) = zhiwei() else {
        eprintln!("没有知微模型，跳过");
        return;
    };
    let texts = ["上海", "伤害", "吃饭"];
    let cold = scorer.score("我今天想去", &texts).unwrap();
    scorer.forget();
    assert!(!scorer.prefix_cached());
    scorer.prefill("我今天想").unwrap();
    scorer.prefill("我今天想去").unwrap();
    let (reused, head) = scorer.prefix_reuse();
    assert!(reused > 0 && reused < head, "追加只算新增：{reused}/{head}");
    let warm = scorer.score("我今天想去", &texts).unwrap();
    let (reused, head) = scorer.prefix_reuse();
    assert_eq!(reused, head, "打分时整段命中");
    for (w, c) in warm.iter().zip(&cold) {
        assert!(
            (w - c).abs() < f64::from(tolerance(scorer.model())) * 10.0,
            "{warm:?} vs {cold:?}"
        );
    }
    assert_eq!(rank(&warm), rank(&cold));
    scorer.forget();
    assert!(!scorer.prefix_cached());
}
