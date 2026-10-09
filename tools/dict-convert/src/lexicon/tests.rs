//! lexicon 的测试：读数据包、标音、拆包的行为（搬自原文件的 `mod tests`，只搬家不改内容）。

use super::{Options, convert};
use std::path::{Path, PathBuf};

fn write(path: &Path, body: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, body).unwrap();
}

fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("qingjian-lexicon-{tag}-{}", std::process::id()));
    std::fs::remove_dir_all(&dir).ok();
    dir
}

/// 中英混杂词走 `--mixed-words` 这条正规通路：含非汉字的词（`C盘`）读音由源文件直接给、
/// 只核对其中的汉字部分，重建后必须在基础词库里。
/// 背景：`84e94b9` 是手工往 `dict.tsv` 里加的这 15 条，代码里没有通路 —— 2026-10-07 用新语料重建时
/// 它们被静默丢掉（`accepts_word` 对字母 C 判失败），这条守住。
#[test]
fn mixed_words_survive_a_rebuild() {
    let dir = temp_dir("mixed");
    let pack = dir.join("pack");
    write(
        &pack.join("01_characters/standard_8105.tsv"),
        "词条\t拼音\t排序号\t文档频次\t字表级别\t来源\n\
             盘\tpan2\t1\t\t1\tx\n\
             站\tzhan4\t2\t\t1\tx\n",
    );
    write(
        &pack.join("02_common/modern_chinese_common_words.tsv"),
        "词条\t拼音\t排序号\t文档频次\t字表级别\t来源\n\
             盘\tpan2\t1\t\t1\tx\n",
    );
    write(
        &pack.join("03_domains/places.tsv"),
        "词条\t拼音\t排序号\t文档频次\t字表级别\t来源\n",
    );
    let unihan = dir.join("Unihan_Readings.txt");
    write(&unihan, "U+76D8\tkMandarin\tpán\nU+7AD9\tkMandarin\tzhàn\n");
    let mixed = dir.join("mixed_words.tsv");
    write(
        &mixed,
        "# 词\t次数\t读音\nC盘\t8000\tc pan\nB站\t40000\tb zhan\n",
    );
    let keep = dir.join("domain-keep.tsv");
    write(&keep, "# 词\n");
    let out = dir.join("out");

    convert(&Options {
        pack: &pack,
        unihan: &unihan,
        pinyin: &[],
        frequency: None,
        char_frequency: None,
        emit_ambiguous: None,
        extra_words: &[],
        mixed_words: &mixed,
        domain_keep_per_10m: 10,
        places_min_df: 500,
        keep_file: &keep,
        internet_dir: &dir.join("04_internet_slang"),
        internet_base: None,
        out_dir: &out,
    })
    .expect("建词库失败");

    let dict = std::fs::read_to_string(out.join("dict.tsv")).unwrap();
    assert!(
        dict.contains("C盘\tc pan\t8000"),
        "C盘 没进基础词库：\n{dict}"
    );
    assert!(
        dict.contains("B站\tb zhan\t40000"),
        "B站 没进基础词库：\n{dict}"
    );
    std::fs::remove_dir_all(dir).unwrap();
}
