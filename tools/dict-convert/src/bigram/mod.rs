//! 从纯文本语料统计词级一元 / 二元计数。
//!
//! 短语层（`assets/lexicon/phrases.tsv`，我的 / 不对 这类）不参与分词：它们进了词库，但语言模型里要是当 token 统计，
//! 「而 + 是」的二元证据就没了，二十 会压过 而是。所以分词时把短语从词表里摘掉，统计完再给每条短语**合成**计数：
//! 一元 = 成分二元计数 c(a,b)，前接 c(v,短语) = c(v,a)·c(a,b)/c(a)，后接 c(短语,w) = c(a,b)·c(b,w)/c(b)。
//! 这样 P(短语|v) = P(a|v)·P(b|a)、P(w|短语) = P(w|b)，短语在整句词图和词级排序里的得分与原来走 a / b 两个词的路径一模一样，
//! 只是多了一个能整块选的词。三词短语的一元按 c(a,b)·c(b,c)/c(b) 估。
//!
//! 分词用青简自己的词库做一元最大概率切分（与输入法词图同一套词表，统计出来的词才能在整句转换里用上）；
//! 只统计连续的汉字段，段与段之间（标点、数字、字母）算句子边界，句首用 `<s>` 标记；空格忽略（预分词语料）。
//! 词库里没有的字跳过，并切断前后的二元关系。

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

use crate::error::ConvertError;
use crate::lexicon::pack::is_source_tsv;
use crate::oov_filter::OovFilter;

/// 句首标记。
const SENTENCE_START: &str = "<s>";

/// 品牌词次数里几分之一算在句首（请柬 的句首占比约 1/8）。
const BRAND_START_SHARE: u32 = 8;

/// 分词时一个词最多几个汉字。
const MAX_WORD_CHARS: usize = 8;

/// 词库里没有的单字的 log 概率（相对于词频 1 的词再扣这么多）。
const UNKNOWN_PENALTY: f64 = -12.0;

/// 分词用的词表：词 → 编号与 log 词频。
pub(crate) struct Vocabulary {
    /// 词 → 编号。
    pub(crate) ids: HashMap<String, u32>,

    /// 编号 → 词。
    pub(crate) words: Vec<String>,

    /// 编号 → log 概率：log(词频 + 1) − log(总词频)。不减总频的话多字词会输给它的单字。
    log_frequency: Vec<f64>,

    /// 未知单字的 log 概率。
    unknown: f64,
}

impl Vocabulary {
    /// 词表文件：`path` 加上同目录 `dicts/` 下的领域词库（拆分后基础词库不含领域词，分词仍要用全部词）。
    pub(crate) fn files(path: &Path) -> Vec<PathBuf> {
        let mut files = vec![path.to_path_buf()];
        if let Some(dir) = path.parent().map(|p| p.join("dicts"))
            && let Ok(entries) = std::fs::read_dir(&dir)
        {
            let mut extra: Vec<_> = entries
                .filter_map(Result::ok)
                .map(|e| e.path())
                .filter(|p| is_source_tsv(p))
                .collect();
            extra.sort();
            tracing::info!(dir = %dir.display(), files = extra.len(), "分词也用领域词库");
            files.extend(extra);
        }
        files
    }

    /// 读分词词表（见 [`Self::files`]）。
    pub(crate) fn load(path: &Path) -> Result<Self, ConvertError> {
        let files = Self::files(path);
        let mut total = 0.0_f64;
        let mut ids: HashMap<String, u32> = HashMap::new();
        let mut words = vec![SENTENCE_START.to_owned()];
        let mut log_frequency = vec![0.0];
        ids.insert(SENTENCE_START.to_owned(), 0);
        for line in files
            .iter()
            .map(|file| File::open(file).map(BufReader::new))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .flat_map(BufRead::lines)
        {
            let line = line?;
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let mut fields = line.split('\t');
            let (Some(text), Some(_), Some(frequency)) =
                (fields.next(), fields.next(), fields.next())
            else {
                continue;
            };
            let frequency: f64 = frequency.trim().parse().unwrap_or(0.0);
            total += frequency;
            let log = (frequency + 1.0).ln();
            match ids.get(text) {
                // 同一个词多个读音：取最高词频
                Some(&id) => {
                    if log > log_frequency[id as usize] {
                        log_frequency[id as usize] = log;
                    }
                }
                None => {
                    ids.insert(text.to_owned(), words.len() as u32);
                    words.push(text.to_owned());
                    log_frequency.push(log);
                }
            }
        }
        let log_total = total.max(1.0).ln();
        for log in &mut log_frequency {
            *log -= log_total;
        }
        Ok(Self {
            ids,
            words,
            log_frequency,
            unknown: UNKNOWN_PENALTY - log_total,
        })
    }

    /// 一段连续汉字按最大概率切成词编号；词库里没有的字用 `None` 占位。
    pub(crate) fn segment(&self, run: &str, output: &mut Vec<Option<u32>>) {
        output.clear();
        let offsets: Vec<usize> = run
            .char_indices()
            .map(|(i, _)| i)
            .chain(std::iter::once(run.len()))
            .collect();
        let n = offsets.len() - 1;
        let mut best = vec![f64::NEG_INFINITY; n + 1];
        let mut back: Vec<(usize, Option<u32>)> = vec![(0, None); n + 1];
        best[0] = 0.0;
        for start in 0..n {
            if best[start] == f64::NEG_INFINITY {
                continue;
            }
            let mut any = false;
            for end in start + 1..=n.min(start + MAX_WORD_CHARS) {
                let slice = &run[offsets[start]..offsets[end]];
                if let Some(&id) = self.ids.get(slice) {
                    any = true;
                    let score = best[start] + self.log_frequency[id as usize];
                    if score > best[end] {
                        best[end] = score;
                        back[end] = (start, Some(id));
                    }
                }
            }
            if !any {
                let score = best[start] + self.unknown;
                if score > best[start + 1] {
                    best[start + 1] = score;
                    back[start + 1] = (start, None);
                }
            }
        }
        let mut position = n;
        while position > 0 {
            let (start, id) = back[position];
            output.push(id);
            position = start;
        }
        output.reverse();
    }
}

/// 同音错误榜 `正确词\t错词\t次数` → 正确词 id → 与它同音的**错词** id 列表。
/// 只收「错的次数 ≥ min_count」的对，一次两次的当个例、不算规律。榜由留出集挖错得到，不是手写名单。
fn load_homophones(
    path: &Path,
    vocabulary: &Vocabulary,
    min_count: u32,
) -> Result<HashMap<u32, Vec<u32>>, ConvertError> {
    let mut out: HashMap<u32, Vec<u32>> = HashMap::new();
    for line in std::fs::read_to_string(path)?.lines() {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut fields = line.split('\t');
        let (Some(right), Some(wrong), Some(count)) = (fields.next(), fields.next(), fields.next())
        else {
            continue;
        };
        let Ok(count) = count.trim().parse::<u32>() else {
            continue;
        };
        if count < min_count {
            continue;
        }
        if let (Some(&right_id), Some(&wrong_id)) = (
            vocabulary.ids.get(right.trim()),
            vocabulary.ids.get(wrong.trim()),
        ) {
            let rivals = out.entry(right_id).or_default();
            if !rivals.contains(&wrong_id) {
                rivals.push(wrong_id);
            }
        }
    }
    Ok(out)
}

pub(crate) fn is_han(c: char) -> bool {
    matches!(c, '\u{4E00}'..='\u{9FFF}' | '\u{3400}'..='\u{4DBF}')
}

/// `mine` 的参数。
pub struct MineOptions {
    /// 语料文件。
    pub corpus: Vec<PathBuf>,

    /// 分词用的词库。
    pub dict: PathBuf,

    /// 次数下限。
    pub min_count: u32,

    /// 最多几个字。
    pub max_chars: usize,

    /// 一元表（算 PMI 用）。
    pub frequency: PathBuf,

    /// PMI 下限，0 不过滤。
    pub min_pmi: f64,

    /// 只过滤这份现成的候选文件，不扫语料。
    pub candidates: Option<PathBuf>,
}

/// 挖词库里没有的词：分词时连续落成单字的那一段（2–`max_chars` 个字）多半是一个词库没收的词，
/// 按出现次数统计，出现 `min_count` 次以上的写到 `oov-candidates.tsv`（`词\t次数`），
/// 再经 [`OovFilter`]（虚词规则 + 相邻字对 PMI）筛成 `oov-filtered.tsv` 与一行一词的 `oov-words.txt`，
/// 后者交给 `gloss-gen pinyin` 标音、前者给 `lexicon --extra-words` 并进词库。
/// 单字词本身在词库里（规范字全收了），所以这里只看「本可以成词却被拆成单字」的连续段：
/// 段内每个字都是词库里的单字词、且整段不在词库里。
pub fn mine(options: &MineOptions, out_dir: &Path) -> Result<(), ConvertError> {
    std::fs::create_dir_all(out_dir)?;
    let counts = match &options.candidates {
        Some(path) => read_candidates(path)?,
        None => {
            let counts = scan_single_runs(options)?;
            write_counts(
                &out_dir.join("oov-candidates.tsv"),
                "# 由 qingjian-dict-convert mine 从语料挖出的词库未收词（未过滤）。词\t次数",
                &sorted_rows(&counts, options.min_count),
            )?;
            counts
        }
    };
    let filter = OovFilter::from_unigram_file(&options.frequency, options.min_pmi)?;
    let kept: Vec<(String, u32)> = sorted_rows(&counts, options.min_count)
        .into_iter()
        .filter(|(word, _)| {
            if options.min_pmi <= 0.0 {
                OovFilter::passes_function_rules(word)
            } else {
                filter.keeps(word, &counts)
            }
        })
        .collect();
    write_counts(
        &out_dir.join("oov-filtered.tsv"),
        &format!(
            "# mine 结果经虚词规则 + 相邻字对 PMI≥{} 过滤。词\t次数",
            options.min_pmi
        ),
        &kept,
    )?;
    let words_path = out_dir.join("oov-words.txt");
    let mut writer = BufWriter::new(File::create(&words_path)?);
    for (word, _) in &kept {
        writeln!(writer, "{word}")?;
    }
    writer.flush()?;
    tracing::info!(
        candidates = counts.values().filter(|c| **c >= options.min_count).count(),
        kept = kept.len(),
        path = %words_path.display(),
        "挖词过滤完成"
    );
    Ok(())
}

/// 读上一次写出的候选文件（`词\t次数`，`#` 注释）。
fn read_candidates(path: &Path) -> Result<HashMap<String, u32>, ConvertError> {
    let mut counts = HashMap::new();
    for line in BufReader::new(File::open(path)?).lines() {
        let line = line?;
        if line.starts_with('#') {
            continue;
        }
        if let Some((word, count)) = line.split_once('\t')
            && let Ok(count) = count.parse::<u32>()
        {
            counts.insert(word.to_owned(), count);
        }
    }
    Ok(counts)
}

/// 次数不低于 `min_count` 的候选，按次数降序、同次数按词排。
fn sorted_rows(counts: &HashMap<String, u32>, min_count: u32) -> Vec<(String, u32)> {
    let mut rows: Vec<(String, u32)> = counts
        .iter()
        .filter(|(_, c)| **c >= min_count)
        .map(|(w, c)| (w.clone(), *c))
        .collect();
    rows.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    rows
}

fn write_counts(path: &Path, header: &str, rows: &[(String, u32)]) -> Result<(), ConvertError> {
    let mut writer = BufWriter::new(File::create(path)?);
    writeln!(writer, "{header}")?;
    for (word, count) in rows {
        writeln!(writer, "{word}\t{count}")?;
    }
    writer.flush()?;
    tracing::info!(rows = rows.len(), path = %path.display(), "已写出");
    Ok(())
}

/// 扫语料，数连续单字段里 2–`max_chars` 字的子串。
fn scan_single_runs(options: &MineOptions) -> Result<HashMap<String, u32>, ConvertError> {
    let MineOptions {
        corpus,
        dict,
        max_chars,
        ..
    } = options;
    let max_chars = *max_chars;
    let vocabulary = Vocabulary::load(dict)?;
    tracing::info!(words = vocabulary.words.len(), "词表加载完成");
    let single_ids: std::collections::HashSet<u32> = vocabulary
        .words
        .iter()
        .enumerate()
        .filter(|(_, w)| w.chars().count() == 1)
        .map(|(i, _)| i as u32)
        .collect();
    let mut counts: HashMap<String, u32> = HashMap::new();
    let mut tokens = Vec::new();
    let mut lines = 0u64;
    for path in corpus {
        tracing::info!(path = %path.display(), "扫描语料");
        for line in BufReader::new(File::open(path)?).lines() {
            let line = line?;
            lines += 1;
            if lines.is_multiple_of(500_000) {
                tracing::info!(lines, candidates = counts.len(), "进度");
                // 内存兜底：候选太多先把只出现一次的扔掉
                if counts.len() > 20_000_000 {
                    counts.retain(|_, c| *c > 1);
                }
            }
            let line: String = line.chars().filter(|c| *c != ' ').collect();
            for run in line.split(|c: char| !is_han(c)) {
                if run.chars().count() < 2 {
                    continue;
                }
                vocabulary.segment(run, &mut tokens);
                // 连续单字段：token 是单字词编号的一串
                let mut start = 0usize;
                let chars: Vec<char> = run.chars().collect();
                let mut position = 0usize;
                let mut singles: Vec<usize> = Vec::new();
                for token in &tokens {
                    let width = match token {
                        Some(id) => vocabulary.words[*id as usize].chars().count(),
                        None => 1,
                    };
                    let is_single = token.is_some_and(|id| single_ids.contains(&id));
                    if is_single {
                        if singles.is_empty() {
                            start = position;
                        }
                        singles.push(position);
                    } else if !singles.is_empty() {
                        collect_runs(&chars, start, position, max_chars, &mut counts);
                        singles.clear();
                    }
                    position += width;
                }
                if !singles.is_empty() {
                    collect_runs(&chars, start, position, max_chars, &mut counts);
                }
            }
        }
    }
    tracing::info!(lines, candidates = counts.len(), "扫描完成");
    Ok(counts)
}

/// 一段连续单字 `chars[start..end]` 里所有 2–`max_chars` 字的子串各计一次。
fn collect_runs(
    chars: &[char],
    start: usize,
    end: usize,
    max_chars: usize,
    counts: &mut HashMap<String, u32>,
) {
    let len = end - start;
    if len < 2 {
        return;
    }
    for width in 2..=max_chars.min(len) {
        for from in start..=end - width {
            let word: String = chars[from..from + width].iter().collect();
            *counts.entry(word).or_insert(0) += 1;
        }
    }
}

/// 一行语料里可以计数的汉字串：长度 ≥ 2，且**不贴着数字**。
///
/// 贴着数字的那些是维基日期与范围的碎片 —— 「3月1日至10日」会切出「日至」，于是它成了词频最高的词之一
/// （2026-10-08 新语料下 日至 52634 次，把真词 日志 2380 次压得看不见）。挡在统计层，不动词库。
fn han_runs(line: &str) -> Vec<&str> {
    let chars: Vec<char> = line.chars().collect();
    let mut runs = Vec::new();
    let mut index = 0usize;
    while index < chars.len() {
        if !is_han(chars[index]) {
            index += 1;
            continue;
        }
        let start = index;
        while index < chars.len() && is_han(chars[index]) {
            index += 1;
        }
        let end = index;
        if end - start < 2 {
            continue;
        }
        // 夹在两个数字之间才算碎片：「3月1日至10日」的「日至」算，「第3章的正文」不算
        let glued_to_digit = start > 0
            && chars[start - 1].is_ascii_digit()
            && end < chars.len()
            && chars[end].is_ascii_digit();
        if glued_to_digit {
            continue;
        }
        runs.push(&line[char_offset(&chars, start)..char_offset(&chars, end)]);
    }
    runs
}

/// 第 n 个字符在字符串里的字节偏移。
fn char_offset(chars: &[char], index: usize) -> usize {
    chars[..index].iter().map(|c| c.len_utf8()).sum()
}

// 参数多是因为统计选项本来就是一堆旋钮；这里不为它单独造一个结构体
#[allow(clippy::too_many_arguments)]
pub fn convert(
    corpus: &[PathBuf],
    dict: &Path,
    phrases: &[PathBuf],
    brand: &[PathBuf],
    min_count: u32,
    max_bigrams: usize,
    phrase_bigram_share: f64,
    phrase_neighbors: usize,
    homophone_boost: f64,
    homophone_errors: Option<&Path>,
    out_dir: &Path,
) -> Result<(), ConvertError> {
    let mut vocabulary = Vocabulary::load(dict)?;
    tracing::info!(words = vocabulary.words.len(), "词表加载完成");
    // 短语不参与分词：先摘掉，记下每条的成分，统计完再合成它们的计数
    let mut phrase_parts = Vec::new();
    for path in phrases {
        phrase_parts.extend(phrase_components(path, &mut vocabulary)?);
    }
    let mut unigram: Vec<u64> = vec![0; vocabulary.words.len()];
    let mut bigram: HashMap<u64, u32> = HashMap::new();
    // 字级频次：单字读不出音的那些（嗯 / 诶）在词级一元表里根本不存在，
    // 建库要按**字频**报警就得单独数一份（2026-10-09）
    let mut char_counts: HashMap<char, u64> = HashMap::new();
    let mut tokens = Vec::new();
    let mut lines = 0u64;
    let mut runs = 0u64;
    for path in corpus {
        tracing::info!(path = %path.display(), "统计语料");
        for line in BufReader::new(File::open(path)?).lines() {
            let line = line?;
            lines += 1;
            if lines.is_multiple_of(200_000) {
                tracing::info!(lines, bigrams = bigram.len(), "进度");
            }
            // LCCC 这类语料是按词用空格分好的；空格不是句子边界，去掉再切
            let line: String = line.chars().filter(|c| *c != ' ').collect();
            for run in han_runs(&line) {
                runs += 1;
                for ch in run.chars() {
                    *char_counts.entry(ch).or_insert(0) += 1;
                }
                vocabulary.segment(run, &mut tokens);
                unigram[0] += 1;
                let mut previous: Option<u32> = Some(0);
                for token in &tokens {
                    match token {
                        Some(id) => {
                            unigram[*id as usize] += 1;
                            if let Some(prev) = previous {
                                *bigram
                                    .entry((u64::from(prev) << 32) | u64::from(*id))
                                    .or_insert(0) += 1;
                            }
                            previous = Some(*id);
                        }
                        None => previous = None,
                    }
                }
            }
        }
    }
    tracing::info!(
        lines,
        sentences = runs,
        distinct_bigrams = bigram.len(),
        "统计完成"
    );
    // 品牌词（青简）与中英混杂词（C盘）语料里没有：按文件给的次数写进一元，句首二元给八分之一（请柬 209 次里 25 次在句首，同一比例），
    // 让词级排序不把它当模型不认识的词扣分、能与同音词（请柬）平起平坐，又不压过 请见 这种整句路径
    for path in brand {
        let mut added = 0usize;
        for line in std::fs::read_to_string(path)?.lines() {
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let mut fields = line.split('\t');
            if let (Some(text), Some(count)) = (fields.next(), fields.next())
                && let Some(&id) = vocabulary.ids.get(text.trim())
                && let Ok(count) = count.trim().parse::<u32>()
            {
                unigram[id as usize] = u64::from(count);
                bigram.insert(u64::from(id), (count / BRAND_START_SHARE).max(1));
                added += 1;
            }
        }
        tracing::info!(path = %path.display(), words = added, "品牌词一元与句首二元已写入");
    }

    // 同音加分：只在「同一个前文 A 下，正确词 B 被错词 W 压过」的位置动手，且**只拉接近的**
    // —— 计数之比决定胜负，光让低频的 (A,B) 存在没用（试过「只救被截断毁掉的对比」，
    // 留出集 56.3% 对修前 56.4%，等于没动，2026-10-09）。这里把 (A,B) 抬到刚好压过对手。
    // `--homophone-boost` 是「愿意拉多远」的旋钮：只有 nB × boost ≥ nW 的才动，1.0 = 只动平手。
    // 不加限制地全拉会把 B 本来选对的位置也翻过去（误伤），所以旋钮必须在挖错的那一半上扫。
    let mut boosted = 0usize;
    if let Some(path) = homophone_errors.filter(|_| homophone_boost > 1.0) {
        let rivals_of = load_homophones(path, &vocabulary, min_count)?;
        let mut updates: Vec<(u64, u32)> = Vec::new();
        for (&key, &count) in &bigram {
            let Some(corrects) = rivals_of.get(&((key & 0xFFFF_FFFF) as u32)) else {
                continue;
            };
            let first = key >> 32;
            for &correct in corrects {
                let target = (first << 32) | u64::from(correct);
                let current = bigram.get(&target).copied().unwrap_or(0);
                if current > 0
                    && current <= count
                    && f64::from(current) * homophone_boost >= f64::from(count)
                {
                    updates.push((target, count + 1));
                }
            }
        }
        boosted = updates.len();
        for (key, want) in updates {
            bigram.insert(key, want);
        }
        tracing::info!(
            boosted,
            boost = homophone_boost,
            distinct_bigrams = bigram.len(),
            homophone_words = rivals_of.len(),
            "同音加分：把被压过的正确侧拉到刚好压过对手"
        );
    }

    // 二元：按计数降序，砍掉低频与超出上限的；短语的合成行另加，不占真实行的名额
    let mut pairs: Vec<(u64, u32)> = bigram
        .iter()
        .map(|(k, c)| (*k, *c))
        .filter(|(_, count)| *count >= min_count)
        .collect();
    // 短语的合成行**预留固定份额**（`--phrase-bigram-share`，缺省 8%）：真实行占满上限时，
    // 短语层（我的 / 后端）一条合成行都拿不到的话整句会退步。真实行先截到「上限 − 预留」。
    let synth_budget =
        ((max_bigrams as f64) * phrase_bigram_share.clamp(0.0, 0.5)).round() as usize;
    let real_budget = max_bigrams.saturating_sub(synth_budget);
    pairs.sort_unstable_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    pairs.truncate(real_budget);
    if boosted > 0 {
        tracing::info!(boosted, real_rows = pairs.len(), "加分后的真实行");
    }
    let real_rows = pairs.len();
    if !phrase_parts.is_empty() {
        let synthesized = synthesize_phrases(
            &phrase_parts,
            &mut unigram,
            &bigram,
            min_count,
            synth_budget,
            phrase_neighbors,
        );
        pairs.extend(synthesized);
    }
    debug_assert!(pairs.len() <= max_bigrams.max(1));
    tracing::info!(
        real = real_rows,
        synth = pairs.len() - real_rows,
        budget = synth_budget,
        total = pairs.len(),
        limit = max_bigrams,
        "二元写出前（真实行 + 合成行 ≤ 上限）"
    );
    let bigram_path = out_dir.join("lm-bigram.tsv");
    let mut writer = BufWriter::new(File::create(&bigram_path)?);
    writeln!(
        writer,
        "# 由 qingjian-dict-convert bigram 从语料统计。前词\\t后词\\t计数"
    )?;
    for (key, count) in &pairs {
        let first = &vocabulary.words[(key >> 32) as usize];
        let second = &vocabulary.words[(key & 0xFFFF_FFFF) as usize];
        writeln!(writer, "{first}\t{second}\t{count}")?;
    }
    writer.flush()?;

    // 一元：只输出出现过的词
    let char_path = out_dir.join("lm-char.tsv");
    {
        let mut writer = BufWriter::new(File::create(&char_path)?);
        writeln!(
            writer,
            "# 由 qingjian-dict-convert bigram 从语料统计的单字频次（字\t次数）"
        )?;
        let mut rows: Vec<(&char, &u64)> = char_counts.iter().collect();
        rows.sort_unstable_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
        for (ch, count) in rows {
            writeln!(writer, "{ch}\t{count}")?;
        }
        writer.flush()?;
    }
    tracing::info!(path = %char_path.display(), chars = char_counts.len(), "字频表已写出");

    let unigram_path = out_dir.join("lm-unigram.tsv");
    let mut writer = BufWriter::new(File::create(&unigram_path)?);
    writeln!(
        writer,
        "# 由 qingjian-dict-convert bigram 从语料统计。词\\t计数；<s> 是句首标记"
    )?;
    let mut written = 0usize;
    for (id, count) in unigram.iter().enumerate() {
        if *count > 0 {
            writeln!(writer, "{}\t{count}", vocabulary.words[id])?;
            written += 1;
        }
    }
    writer.flush()?;
    tracing::info!(
        unigram = %unigram_path.display(),
        words = written,
        bigram = %bigram_path.display(),
        bigrams = pairs.len(),
        "写出完成"
    );
    Ok(())
}

/// 把 `path` 里的短语从分词词表摘掉（编号留着给合成的 token 用），返回每条短语的 (编号, 成分词编号)。
/// 切不成两个以上词库词的短语跳过。
fn phrase_components(
    path: &Path,
    vocabulary: &mut Vocabulary,
) -> Result<Vec<(u32, Vec<u32>)>, ConvertError> {
    let texts: Vec<String> = std::fs::read_to_string(path)?
        .lines()
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .filter_map(|l| l.split('\t').next().map(str::to_owned))
        .collect();
    let ids: Vec<Option<u32>> = texts
        .iter()
        .map(|t| vocabulary.ids.get(t).copied())
        .collect();
    for text in &texts {
        vocabulary.ids.remove(text);
    }
    let mut parts = Vec::new();
    let mut tokens = Vec::new();
    let mut skipped = 0usize;
    for (text, id) in texts.iter().zip(ids) {
        let Some(id) = id else {
            skipped += 1;
            continue;
        };
        vocabulary.segment(text, &mut tokens);
        let components: Option<Vec<u32>> = tokens.iter().copied().collect();
        match components {
            Some(components) if components.len() >= 2 => parts.push((id, components)),
            _ => skipped += 1,
        }
    }
    tracing::info!(path = %path.display(), phrases = parts.len(), skipped, "短语已从分词词表摘掉，统计完再合成计数");
    Ok(parts)
}

/// 给短语合成一元（写进 `unigram`）与前后接的二元计数（返回，算法见模块注释）。
fn synthesize_phrases(
    phrases: &[(u32, Vec<u32>)],
    unigram: &mut [u64],
    bigram: &HashMap<u64, u32>,
    min_count: u32,
    max_synth_bigrams: usize,
    max_neighbors: usize,
) -> Vec<(u64, u32)> {
    let key = |a: u32, b: u32| (u64::from(a) << 32) | u64::from(b);
    // 按后词 / 前词索引一遍二元表，合成时按成分查前接与后接
    let mut by_second: HashMap<u32, Vec<(u32, u32)>> = HashMap::new();
    let mut by_first: HashMap<u32, Vec<(u32, u32)>> = HashMap::new();
    for (&k, &count) in bigram.iter() {
        let (first, second) = ((k >> 32) as u32, (k & 0xFFFF_FFFF) as u32);
        by_second.entry(second).or_default().push((first, count));
        by_first.entry(first).or_default().push((second, count));
    }
    let pair = |bigram: &HashMap<u64, u32>, a: u32, b: u32| {
        f64::from(bigram.get(&key(a, b)).copied().unwrap_or(0))
    };
    // 邻居表按计数降序，之后每条短语只取前 K 个 —— 常用成分（的 / 了 / 是）的邻居上百万，
    // 全合成会把预留份额吃光，而真正有用的只是最高频的那批
    for list in by_second.values_mut() {
        list.sort_unstable_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        list.truncate(max_neighbors);
    }
    for list in by_first.values_mut() {
        list.sort_unstable_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        list.truncate(max_neighbors);
    }
    let mut added = 0usize;
    let mut rows: Vec<(u64, u32)> = Vec::new();
    for (id, parts) in phrases {
        let (first, last) = (parts[0], parts[parts.len() - 1]);
        // 短语自身的次数：两词就是成分二元，三词按链式估
        let mut count = pair(bigram, parts[0], parts[1]);
        for window in parts.windows(2).skip(1) {
            let middle = unigram[window[0] as usize] as f64;
            if middle <= 0.0 {
                count = 0.0;
                break;
            }
            count *= pair(bigram, window[0], window[1]) / middle;
        }
        if count < f64::from(min_count) {
            continue;
        }
        unigram[*id as usize] = count.round() as u64;
        added += 1;
        let first_total = unigram[first as usize].max(1) as f64;
        let last_total = unigram[last as usize].max(1) as f64;
        for (previous, c) in by_second.get(&first).into_iter().flatten() {
            let synthesized = f64::from(*c) * count / first_total;
            if synthesized >= f64::from(min_count) {
                rows.push((key(*previous, *id), synthesized.round() as u32));
            }
        }
        for (next, c) in by_first.get(&last).into_iter().flatten() {
            let synthesized = count * f64::from(*c) / last_total;
            if synthesized >= f64::from(min_count) {
                rows.push((key(*id, *next), synthesized.round() as u32));
            }
        }
    }
    // 合成行也要有上限：每个短语会把它首尾成分的**所有**邻居都合成一遍，常用成分（的 / 了 / 是）
    // 的邻居上百万，5838 条短语就能合成出上千万行 —— 2026-10-08 实测 1068 万行，把 500 万上限的
    // 语言模型撑到 1568 万，产品侧（iOS 内存预算）与「和旧库同规模对比」都做不到。
    rows.sort_unstable_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    rows.truncate(max_synth_bigrams);
    tracing::info!(phrases = added, bigrams = rows.len(), "短语计数已合成");
    rows
}

#[cfg(test)]
mod tests;
