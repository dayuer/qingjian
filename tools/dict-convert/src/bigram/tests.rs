//! bigram 的测试：统计与截断的行为（搬自原文件的 `mod tests`，只搬家不改内容）。

use super::*;

/// 合成行不能超过预算：每个短语会把它首尾成分的所有邻居都合成一遍，常用成分（的 / 了 / 是）
/// 的邻居上百万 —— 不限量的话 5838 条短语能合成出上千万行，把 500 万上限的模型撑到 1568 万。
#[test]
fn synthesized_phrases_respect_their_budget() {
    let key = |a: u32, b: u32| (u64::from(a) << 32) | u64::from(b);
    let mut bigram: HashMap<u64, u32> = HashMap::new();
    // 短语 = [1, 2]：它前面的一万个词（prev → 1），它后面的一万个词（2 → next）
    for prev in 100u32..10_100 {
        bigram.insert(key(prev, 1), 10);
    }
    for next in 20_000u32..30_000 {
        bigram.insert(key(2, next), 10);
    }
    bigram.insert(key(1, 2), 50); // 短语自身的成分对
    let mut unigram = vec![0u64; 40_000];
    unigram[1] = 100; // 首词总次数：合成计数 = 邻居次数 × 短语次数 / 总次数 = 10 × 50 / 100 = 5
    unigram[2] = 100;
    let phrases = vec![(9u32, vec![1u32, 2u32])];
    let rows = synthesize_phrases(&phrases, &mut unigram, &bigram, 3, 100, 1_000);
    assert!(!rows.is_empty(), "预算内应该还有合成行");
    assert!(
        rows.len() <= 100,
        "合成行超过预算：{} 行（上限 100）",
        rows.len()
    );
}

/// 每条短语只合成前 K 个邻居：不设 K 时，常用成分（的 / 了 / 是）的上百万邻居会把
/// 预留份额吃光，真正高频的那批反而进不来。
#[test]
fn synthesized_phrases_keep_only_top_k_neighbors() {
    let key = |a: u32, b: u32| (u64::from(a) << 32) | u64::from(b);
    let mut bigram: HashMap<u64, u32> = HashMap::new();
    for prev in 100u32..1_100 {
        bigram.insert(key(prev, 1), 10 + prev);
    }
    bigram.insert(key(1, 2), 50);
    let mut unigram = vec![0u64; 2_000];
    unigram[1] = 100;
    let phrases = vec![(9u32, vec![1u32, 2u32])];
    let rows = synthesize_phrases(&phrases, &mut unigram, &bigram, 3, 10_000, 5);
    assert_eq!(rows.len(), 5, "只该合成前 5 个邻居：{}", rows.len());
    // 前 5 个应该是计数最高的那几个（1099…1095），对应前词 1099…1095
    let mut seconds: Vec<u32> = rows.iter().map(|(k, _)| (*k >> 32) as u32).collect();
    seconds.sort_unstable_by(|a, b| b.cmp(a));
    assert_eq!(seconds, vec![1099, 1098, 1097, 1096, 1095]);
}

/// 贴着数字的汉字串是日期/范围的碎片（「3月1日至10日」里的「日至」），统计时不计数；
/// 不贴着数字的正常词照旧。2026-10-08 加：新语料下 日至 52634 次，把真词 日志 2380 次压得看不见。
#[test]
fn han_runs_skips_fragments_glued_to_digits() {
    assert_eq!(
        han_runs("3月1日至10日"),
        Vec::<&str>::new(),
        "「日至」夹在两个数字之间，丢"
    );
    assert_eq!(han_runs("2019年3月至5月").len(), 0, "被数字夹住的都该丢");
    assert_eq!(
        han_runs("参见参考资料"),
        vec!["参见参考资料"],
        "正常词不受影响"
    );
    assert_eq!(
        han_runs("2019年至2020年"),
        Vec::<&str>::new(),
        "日期范围里夹出来的都丢"
    );
    assert_eq!(
        han_runs("第3章的正文"),
        vec!["章的正文"],
        "只夹了一边数字的不算碎片"
    );
    assert_eq!(han_runs("5G手机"), vec!["手机"], "字母夹着的不算数字");
}

#[test]
fn vocabulary_skips_hidden_temp_and_non_file_dicts() {
    let dir = std::env::temp_dir().join(format!(
        "qingjian-dict-convert-bigram-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("dicts")).unwrap();
    let base = dir.join("dict.tsv");
    std::fs::write(&base, "词\t拼音\t词频\n").unwrap();
    for name in ["law.tsv", ".hidden.tsv", "~$law.tsv", "notes.txt"] {
        std::fs::write(dir.join("dicts").join(name), "词\t拼音\t词频\n").unwrap();
    }

    let files = Vocabulary::files(&base);
    let names: Vec<String> = files
        .iter()
        .filter_map(|p| p.file_name().and_then(|n| n.to_str()).map(str::to_owned))
        .collect();
    assert_eq!(names, ["dict.tsv", "law.tsv"]);
    let _ = std::fs::remove_dir_all(&dir);
}
