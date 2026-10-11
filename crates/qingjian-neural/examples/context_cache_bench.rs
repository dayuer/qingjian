//! 前缀缓存（B4）的延迟：同一上文下连续 10 键的重排耗时，缓存留着（现在）对每键清空（等同改前的整段重算）。
//!
//! 通变：拼音每键多一个字母，`score_p2c` 6 条路径；知微：64 字前文上屏两个字后第一次打分，增量 / 预填 / 整段重算三种。
//! 用法：`cargo run --release -p qingjian-neural --example context_cache_bench -- [通变模型] [知微模型]`
//! 缺省 `data/models/hanzhang-tongbian-q8`（iOS 装的 8 位）与 `data/models/hanzhang-zhiwei`。

use std::path::PathBuf;
use std::time::Instant;

use qingjian_neural::CharScorer;

/// 连续敲的 10 键：这串拼音从第 10 个字母敲到第 19 个。
const KEYS: &str = "womenmingtiankaihui";

/// 每键重排的整句路径（iOS 一次重排 6 条）。
const PATHS: [&str; 6] = [
    "我们明天开会",
    "我们明天",
    "我门明天开会",
    "我们名天开会",
    "我们明天看会",
    "沃们明天开会",
];

const ROUNDS: usize = 20;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let tongbian: PathBuf = args
        .next()
        .unwrap_or_else(|| "data/models/hanzhang-tongbian-q8".into())
        .into();
    let zhiwei: PathBuf = args
        .next()
        .unwrap_or_else(|| "data/models/hanzhang-zhiwei".into())
        .into();

    let scorer = CharScorer::load(&tongbian)?;
    println!(
        "== 通变 {}：连续 10 键，每键重排 {} 条 ==",
        tongbian.display(),
        PATHS.len()
    );
    println!("  键   拼音                   缓存留着 ms   每键清空 ms   复用/前缀");
    let mut warm = vec![Vec::new(); 10];
    let mut cold = vec![Vec::new(); 10];
    let mut reuse = [(0, 0); 10];
    for _ in 0..ROUNDS {
        scorer.forget();
        for (key, end) in (10..=KEYS.len()).enumerate() {
            let typed = &KEYS[..end];
            warm[key].push(time(|| {
                scorer.score_p2c(typed, &PATHS).unwrap();
            }));
            reuse[key] = scorer.prefix_reuse();
        }
        for (key, end) in (10..=KEYS.len()).enumerate() {
            let typed = &KEYS[..end];
            scorer.forget();
            cold[key].push(time(|| {
                scorer.score_p2c(typed, &PATHS).unwrap();
            }));
        }
    }
    for key in 0..10 {
        println!(
            "  {:2}   {:20}   {:10.2}   {:10.2}   {}/{}",
            key + 1,
            &KEYS[..10 + key],
            median(&mut warm[key]),
            median(&mut cold[key]),
            reuse[key].0,
            reuse[key].1
        );
    }
    let mut warm_rest: Vec<f64> = warm[1..].iter().flatten().copied().collect();
    let mut cold_rest: Vec<f64> = cold[1..].iter().flatten().copied().collect();
    println!(
        "  第 2 键起中位：缓存留着 {:.2} ms，每键清空 {:.2} ms",
        median(&mut warm_rest),
        median(&mut cold_rest)
    );

    let scorer = CharScorer::load(&zhiwei)?;
    let before: String = "今天下午的会议讨论了输入法的排序问题，大家觉得整句转换还可以再准一些，"
        .repeat(2)
        .chars()
        .take(62)
        .collect();
    let after = format!("{before}我们");
    let words = ["明天", "名天", "明田", "鸣天", "明添", "冥天"];
    println!(
        "\n== 知微 {}：62 字前文上屏两个字后第一次打分 {} 个词 ==",
        zhiwei.display(),
        words.len()
    );
    let mut full = Vec::new();
    let mut incremental = Vec::new();
    let mut prefilled = Vec::new();
    for _ in 0..ROUNDS {
        scorer.forget();
        full.push(time(|| {
            scorer.score(&after, &words).unwrap();
        }));
        scorer.score(&before, &words)?;
        incremental.push(time(|| {
            scorer.score(&after, &words).unwrap();
        }));
        scorer.score(&before, &words)?;
        scorer.prefill(&after)?;
        prefilled.push(time(|| {
            scorer.score(&after, &words).unwrap();
        }));
    }
    println!("  整段重算（改前）  {:6.2} ms", median(&mut full));
    println!("  增量（只算两个字）{:6.2} ms", median(&mut incremental));
    println!("  预填过（关键路径）{:6.2} ms", median(&mut prefilled));
    Ok(())
}

fn time(f: impl FnOnce()) -> f64 {
    let started = Instant::now();
    f();
    started.elapsed().as_secs_f64() * 1e3
}

fn median(samples: &mut [f64]) -> f64 {
    samples.sort_by(f64::total_cmp);
    samples[samples.len() / 2]
}
