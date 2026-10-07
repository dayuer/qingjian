//! 青简自己的词库：从「输入法字词库_分类整理版」数据包（规范字 + 现代汉语常用词 + THUOCL 领域词）建 `dict.tsv`。
//!
//! 通用词自带拼音，直接规范化；规范字与领域词没有拼音，读音来自 Unihan（Unicode 许可）：
//! 单字按 kHanyuPinlu / kXHC1983 / kMandarin 给全部读音与权重，多字词按字拼接，多音字先看 LLM 标注
//! （`gloss-gen pinyin` 的 JSONL，逐字对照 Unihan 校验）、再看通用词里该字最常见的读音、再看 kHanyuPinlu。
//! 词频来自自己的语料统计（`bigram` 子命令的 lm-unigram.tsv），没统计到的按词表排序号 / 文档频次给一个很小的底值。
//!
//! 两遍跑：第一遍没有词频，只为分词与 `--emit-ambiguous` 出待标注词表；标注、统计完再跑一遍写最终 dict.tsv。
//!
//! 领域词拆开出：语料里出现够多的（`domain_keep_min`）是通用词，留在基础词库 `dict.tsv`；其余按来源文件各写一本
//! `dicts/<领域>.tsv` 与带元数据的 `dicts/<领域>.qj`，随包分发、缺省关闭，用户按需打开。基础词库与各领域词库互不重叠。
//! 拆之前先过第 1 步的规则（`domain_filter`：长度门槛、寄主 + 学名、小地名、诗词名句），
//! 删了什么、改派到哪一本、哪几条从基础词库移出去，写进 `data/generated/domain-report.tsv`。

mod annotations;
mod corpus;
mod domain_filter;
mod domain_report;
mod entry;
mod internet;
pub(crate) mod pack;
mod readings;
mod tone;

use std::collections::{BTreeMap, HashMap, HashSet};
use std::io::{BufWriter, Write};
use std::path::Path;

use qingjian_core::parser::is_syllable;
use qingjian_dictionary::Dictionary;
use qingjian_format::Metadata;

pub use domain_filter::DEFAULT_PLACES_MIN_DF;

use domain_filter::{Sink, Whitelist};
use domain_report::DomainReport;

use crate::error::ConvertError;
use entry::LexiconEntry;
use pack::Pack;
use readings::CharReadings;

/// 领域词最长几个字：再长是法规名、诗句，输入法用不上。
const MAX_WORD_CHARS: usize = 10;

/// 没统计到词频时通用词的底值区间上限（按排序号线性递减到 1）。
const COMMON_FLOOR: f64 = 100.0;

/// 没统计到词频时领域词的底值上限（按文档频次取对数）。
const DOMAIN_FLOOR: u32 = 30;

/// 没统计到词频时单字按字表级别给的底值。
const CHAR_FLOOR: [u32; 3] = [30, 10, 3];

/// 单字不在 kHanyuPinlu 里的次要读音占主读音权重的比例。
const MINOR_READING_SHARE: f64 = 0.05;

/// 常用词表自带的读音与 LLM 标注不一致时，原表读音降到标注读音词频的几分之一保留（怎么打都找得到，但不抢首选）。
const DISPUTED_READING_DIVISOR: u32 = 8;

/// 领域词库的中文名（文件名主干 → 名称），写进 `.qj` 元数据，偏好设置「词库」页显示它。
const DOMAIN_NAMES: [(&str, &str); 14] = [
    ("animals", "动物"),
    ("automotive", "汽车"),
    ("finance", "财经"),
    ("food", "饮食"),
    ("historical_figures", "历史人物"),
    ("idioms", "成语"),
    ("it_computing", "IT 与计算机"),
    ("law", "法律"),
    ("medicine", "医学"),
    ("places", "地名"),
    ("internet_slang", "网络用语"),
    ("internet_slang_coarse", "网络用语（粗口）"),
    ("places-extended", "地名（扩展，含社区与巷弄）"),
    ("poetry_lines", "诗词名句"),
];

/// 领域词库的许可证与署名（THUOCL）。
const DOMAIN_LICENSE: &str = "MIT AND Unicode-3.0";
const DOMAIN_ATTRIBUTION: &str =
    "THUOCL（清华大学自然语言处理实验室，MIT）；读音 Unihan（Unicode）";
const DOMAIN_SOURCE: &str = "https://github.com/thunlp/THUOCL";

/// 网络用语包的许可证与署名（与 THUOCL 那几本不同，单独一套）。
const INTERNET_LICENSE: &str = "CC-BY-SA-4.0";
const INTERNET_ATTRIBUTION: &str =
    "维基百科与维基词典（CC BY-SA 4.0）；CC-CEDICT（CC BY-SA 4.0）；读音 Unihan（Unicode）";
const INTERNET_SOURCE: &str =
    "https://zh.wikipedia.org/wiki/Category:%E4%BA%92%E8%81%94%E7%BD%91%E7%94%A8%E8%AF%AD";

/// `lexicon` 的入参（位置参数已经够多，收成一个结构；逐项含义见字段注释）。
pub struct Options<'a> {
    /// 数据包目录（`01_characters` / `02_common` / `03_domains`）
    pub pack: &'a Path,

    /// Unihan_Readings.txt
    pub unihan: &'a Path,

    /// LLM 标注的多音字词读音 JSONL
    pub pinyin: Option<&'a Path>,

    /// 语料词频（lm-unigram.tsv）
    pub frequency: Option<&'a Path>,

    /// 写出仍靠猜读音的多音字词
    pub emit_ambiguous: Option<&'a Path>,

    /// 额外并入的词（`mine` / `phrases` / 人工挑的那些）
    pub extra_words: &'a [std::path::PathBuf],

    /// 领域词留在基础词库的最低语料次数（没有词频时按领域词处理，全部拆出去）
    pub domain_keep_min: u64,

    /// 地名包只留文档频次不低于这个值的地点（第 1 步 R3；低于它的进 places-extended）
    pub places_min_df: u64,

    /// 长度门槛的白名单（固定书名这类必须留的长专名，一行一个词）
    pub keep_file: &'a Path,

    /// 网络用语包目录（`04_internet_slang/`，一本包一个 TSV）
    pub internet_dir: &'a Path,

    /// 网络用语里够「天天会打」的那份（`internet_base.tsv`）：给了才并进基础词库。
    /// 同音不危险要等语料实测，所以缺省不并；命令行 `--internet-base` 指过来才生效。
    pub internet_base: Option<&'a Path>,

    /// 输出目录
    pub out_dir: &'a Path,
}

/// 建词库：读数据包 → 标音 → 按语料词频写成 `dict.tsv` 与 `dicts/*.qj`。
/// 领域包先过第 1 步的规则（`domain_filter`，见 `docs/plan/dictionary-layering.md`），
/// 再按语料次数决定留基础词库还是拆包；删了什么、改派了什么写进 `domain-report.tsv`。
pub fn convert(options: &Options) -> Result<(), ConvertError> {
    let pack_dir = options.pack;
    let unihan = options.unihan;
    let pinyin = options.pinyin;
    let frequency = options.frequency;
    let emit_ambiguous = options.emit_ambiguous;
    let extra_words = options.extra_words;
    let domain_keep_min = options.domain_keep_min;
    let out_dir = options.out_dir;
    let mut pack = Pack::load(pack_dir)?;
    // 语料挖出来的词当作一类领域词并入：次数当文档频次（也就是没有语料词频时的底值来源）
    for path in extra_words {
        let before = pack.domain.len();
        for word in corpus::extra_words(path)? {
            pack.domain.push(pack::DomainRow {
                text: word.text,
                df: word.count,
                domain: None,
                frequency: None,
                syllables: word.syllables,
            });
        }
        tracing::info!(path = %path.display(), rows = pack.domain.len() - before, "额外词已读取");
    }
    let readings = CharReadings::load(unihan)?;
    let annotations = match pinyin {
        Some(path) => annotations::load(path)?,
        None => HashMap::new(),
    };
    let mut counts = match frequency {
        Some(path) => corpus::load(path)?,
        None => HashMap::new(),
    };
    // 挖出来的词在语料里的次数就是它的词频（分词时它被拆成单字，一元表里没有）
    for path in extra_words {
        for word in corpus::extra_words(path)? {
            counts.entry(word.text).or_insert(word.count);
        }
    }
    tracing::info!(
        chars = pack.chars.len(),
        common = pack.common.len(),
        domain = pack.domain.len(),
        readings = readings.len(),
        annotations = annotations.len(),
        counts = counts.len(),
        "数据加载完成"
    );

    // 通用词里每个字各种读音出现的次数：多音字在词里猜读音的第一依据
    let mut common_stats: HashMap<char, HashMap<String, u32>> = HashMap::new();
    for row in &pack.common {
        for (ch, syllable) in row.text.chars().zip(&row.syllables) {
            *common_stats
                .entry(ch)
                .or_default()
                .entry(syllable.clone())
                .or_default() += 1;
        }
    }

    // 网络用语（第 2 步）：一本 TSV 一本包，当作一类领域词并进去，读音与词频走同一条路
    for (pack_name, rows) in internet::load_dir(options.internet_dir)? {
        let before = pack.domain.len();
        for row in rows {
            let frequency = row.frequency();
            pack.domain.push(pack::DomainRow {
                text: row.text,
                df: 0,
                domain: Some(pack_name.clone()),
                syllables: row.syllables,
                frequency: Some(frequency),
            });
        }
        tracing::info!(pack = %pack_name, rows = pack.domain.len() - before, "网络用语已并入");
    }
    // 够「天天会打」的那份（缺省不并：同音还要实测，见 Options 的注释）
    if let Some(path) = options.internet_base {
        tracing::info!(path = %path.display(), "网络用语底子并入基础词库（--internet-base）");
    }

    let mut entries: BTreeMap<(String, Vec<String>), LexiconEntry> = BTreeMap::new();
    // 拆出去的领域词：领域 → 词条
    let mut domains: BTreeMap<String, BTreeMap<(String, Vec<String>), LexiconEntry>> =
        BTreeMap::new();
    let mut insert = |entry: LexiconEntry| {
        let key = (entry.text.clone(), entry.syllables.clone());
        entries
            .entry(key)
            .and_modify(|e| e.frequency = e.frequency.max(entry.frequency))
            .or_insert(entry);
    };
    let mut dropped = 0usize;
    let keep = Whitelist::load(options.keep_file)?;
    let mut report = DomainReport::default();

    // 单字：规范字表全部 + 通用词里用到而字表没有的
    let mut chars: Vec<(char, Option<u8>)> =
        pack.chars.iter().map(|c| (c.ch, Some(c.level))).collect();
    let listed: HashSet<char> = chars.iter().map(|c| c.0).collect();
    let mut extra: Vec<char> = pack
        .common
        .iter()
        .flat_map(|row| row.text.chars())
        .filter(|c| !listed.contains(c))
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    extra.sort_unstable();
    chars.extend(extra.into_iter().map(|c| (c, None)));
    for (ch, level) in &chars {
        let weighted = readings.weighted(*ch, MINOR_READING_SHARE);
        let valid: Vec<(String, f64)> = weighted
            .into_iter()
            .filter(|(syllable, _)| is_syllable(syllable))
            .collect();
        if valid.is_empty() {
            dropped += 1;
            continue;
        }
        let total = counts.get(&ch.to_string()).copied().unwrap_or_else(|| {
            u64::from(CHAR_FLOOR[usize::from(level.unwrap_or(3).saturating_sub(1).min(2))])
        });
        let sum: f64 = valid.iter().map(|(_, w)| w).sum();
        for (syllable, weight) in valid {
            let frequency = ((total as f64) * weight / sum).round().max(1.0) as u32;
            insert(LexiconEntry {
                text: ch.to_string(),
                syllables: vec![syllable],
                frequency,
            });
        }
    }

    // 通用词：自带拼音，但原表对多音字有错（重庆 zhong qing、长大 chang da），含多音字的词以 LLM 标注为主读音，
    // 原表读音不同时降权保留
    let common_total = pack.common.len().max(1) as f64;
    let char_texts: HashSet<String> = chars.iter().map(|(ch, _)| ch.to_string()).collect();
    let mut common_texts: HashSet<&str> = HashSet::new();
    let mut ambiguous: BTreeMap<String, ()> = BTreeMap::new();
    let mut disputed = 0usize;
    let polyphonic = |text: &str| {
        text.chars().any(|ch| {
            readings
                .all(ch)
                .into_iter()
                .filter(|s| is_syllable(s))
                .count()
                > 1
        })
    };
    for row in &pack.common {
        if !row.syllables.iter().all(|s| is_syllable(s)) {
            dropped += 1;
            continue;
        }
        // 单字以字表那条为准：那里按读音频次分摊了词频；通用词表里 了 有 le / liao 两条，
        // 都给整个字的词频会让 liao 下的 了 压过 聊 / 料
        if char_texts.contains(&row.text) {
            common_texts.insert(&row.text);
            continue;
        }
        common_texts.insert(&row.text);
        let frequency = counts.get(&row.text).map_or_else(
            || (1.0 + COMMON_FLOOR * (1.0 - row.rank as f64 / common_total)).round() as u64,
            |&c| c.max(1),
        );
        let frequency = frequency.min(u64::from(u32::MAX)) as u32;
        let verified = annotations
            .get(&row.text)
            .filter(|a| readings.accepts_word(&row.text, a) && a.iter().all(|s| is_syllable(s)));
        match verified {
            Some(annotation) if *annotation != row.syllables => {
                disputed += 1;
                insert(LexiconEntry {
                    text: row.text.clone(),
                    syllables: annotation.clone(),
                    frequency,
                });
                insert(LexiconEntry {
                    text: row.text.clone(),
                    syllables: row.syllables.clone(),
                    frequency: (frequency / DISPUTED_READING_DIVISOR).max(1),
                });
            }
            Some(_) => insert(LexiconEntry {
                text: row.text.clone(),
                syllables: row.syllables.clone(),
                frequency,
            }),
            None => {
                if polyphonic(&row.text) {
                    ambiguous.insert(row.text.clone(), ());
                }
                insert(LexiconEntry {
                    text: row.text.clone(),
                    syllables: row.syllables.clone(),
                    frequency,
                });
            }
        }
    }

    // 领域词：没有拼音，按字推；多音字先看标注。额外词文件给了读音的（短语层）直接用
    let mut guessed = 0usize;
    let mut given = 0usize;
    let mut annotated = 0usize;
    let mut rejected_annotations = 0usize;
    let mut seen_domain: HashSet<&str> = HashSet::new();
    for row in &pack.domain {
        let text = row.text.as_str();
        let chars_count = text.chars().count();
        if !(2..=MAX_WORD_CHARS).contains(&chars_count)
            || common_texts.contains(text)
            || !seen_domain.insert(text)
        {
            continue;
        }
        // 额外词文件给了读音的（短语层由成分词拼出）直接用；否则先看 LLM 标注，再按字推
        let given_syllables = row
            .syllables
            .as_ref()
            .filter(|s| readings.accepts_word(text, s) && s.iter().all(|s| is_syllable(s)));
        let syllables = match (given_syllables, annotations.get(text)) {
            (Some(candidate), _) => {
                given += 1;
                candidate.clone()
            }
            (None, Some(candidate))
                if readings.accepts_word(text, candidate)
                    && candidate.iter().all(|s| is_syllable(s)) =>
            {
                annotated += 1;
                candidate.clone()
            }
            (None, other) => {
                if other.is_some() {
                    rejected_annotations += 1;
                }
                let mut guess = Vec::with_capacity(chars_count);
                let mut polyphonic = false;
                let mut ok = true;
                for ch in text.chars() {
                    let options: Vec<String> = readings
                        .all(ch)
                        .into_iter()
                        .filter(|s| is_syllable(s))
                        .collect();
                    if options.is_empty() {
                        ok = false;
                        break;
                    }
                    if options.len() > 1 {
                        polyphonic = true;
                    }
                    let best = common_stats
                        .get(&ch)
                        .and_then(|stats| {
                            stats
                                .iter()
                                .filter(|(s, _)| options.contains(s))
                                .max_by_key(|(_, n)| **n)
                                .map(|(s, _)| s.clone())
                        })
                        .unwrap_or_else(|| options[0].clone());
                    guess.push(best);
                }
                if !ok {
                    dropped += 1;
                    continue;
                }
                if polyphonic {
                    ambiguous.insert(text.to_owned(), ());
                }
                guessed += 1;
                guess
            }
        };
        let corpus_count = counts.get(text).copied();
        // 网络用语自带词频（没填的按兜底值）；其余领域词按语料次数或文档频次折算
        if let Some(given) = row.frequency {
            let entry = LexiconEntry {
                text: text.to_owned(),
                syllables,
                frequency: given,
            };
            let domain = row.domain.clone().unwrap_or_default();
            report.record_kept(&domain);
            domains
                .entry(domain)
                .or_default()
                .insert((entry.text.clone(), entry.syllables.clone()), entry);
            continue;
        }
        let frequency = corpus_count.map_or_else(
            || 1 + ((row.df as f64 + 1.0).log2().round() as u32).min(DOMAIN_FLOOR),
            |c| c.max(1).min(u64::from(u32::MAX)) as u32,
        );
        let entry = LexiconEntry {
            text: text.to_owned(),
            syllables,
            frequency,
        };
        let Some(domain) = row.domain.clone() else {
            // 语料挖出来的额外词（`mine` / `phrases` / 人工挑的领域词）：一律留在基础词库
            insert(entry);
            continue;
        };
        // 第 1 步的规则：判死的丢掉，判走的换一本包
        let verdict =
            domain_filter::judge(&domain, &entry.text, row.df, options.places_min_df, &keep);
        let from_base = corpus_count.unwrap_or(0) >= domain_keep_min;
        match verdict {
            Some((Sink::Drop, reason)) => {
                report.record_drop(&domain, &entry.text, row.df, reason);
                continue;
            }
            Some((Sink::Pack(to), _)) => {
                report.record_move(&domain, to, from_base);
                domains
                    .entry(to.to_owned())
                    .or_default()
                    .insert((entry.text.clone(), entry.syllables.clone()), entry);
                continue;
            }
            None => {}
        }
        report.record_kept(&domain);
        // 语料里常见的领域词其实是通用词（医疗器械、侵权行为），留在基础词库；其余进各自的领域词库
        if from_base {
            insert(entry);
        } else {
            domains
                .entry(domain)
                .or_default()
                .insert((entry.text.clone(), entry.syllables.clone()), entry);
        }
    }

    std::fs::create_dir_all(out_dir)?;
    report.write(&out_dir.join("domain-report.tsv"))?;
    let output = out_dir.join("dict.tsv");
    let mut file = BufWriter::new(std::fs::File::create(&output)?);
    writeln!(
        file,
        "# 青简基础词库，由 qingjian-dict-convert lexicon 生成。词\t音节\t词频\n\
# 来源：通用规范汉字表（8105 字）；现代汉语常用词表（liuxilu 校对版）；THUOCL 领域词（MIT，清华大学自然语言处理实验室）；\n\
# 读音：Unihan（Unicode License）+ LLM 标注多音字词；词频：青简自己的语料统计（中文维基 CC BY-SA 4.0、LCCC MIT）。"
    )?;
    for entry in entries.values() {
        writeln!(
            file,
            "{}\t{}\t{}",
            entry.text,
            entry.syllables.join(" "),
            entry.frequency
        )?;
    }
    file.flush()?;
    write_domains(&domains, out_dir)?;
    if let Some(path) = emit_ambiguous {
        let mut file = BufWriter::new(std::fs::File::create(path)?);
        for word in ambiguous.keys() {
            writeln!(file, "{word}")?;
        }
        file.flush()?;
        tracing::info!(path = %path.display(), words = ambiguous.len(), "待标注的多音字词已写出");
    }
    tracing::info!(
        path = %output.display(),
        entries = entries.len(),
        domain_libraries = domains.len(),
        domain_entries = domains.values().map(BTreeMap::len).sum::<usize>(),
        given,
        annotated,
        guessed,
        disputed_common_readings = disputed,
        still_ambiguous = ambiguous.len(),
        rejected_annotations,
        dropped,
        "词库写出完成"
    );
    Ok(())
}

/// 每个领域写一本 `dicts/<领域>.tsv`（与主词库同格式）和一本带元数据的 `dicts/<领域>.qj`。
fn write_domains(
    domains: &BTreeMap<String, BTreeMap<(String, Vec<String>), LexiconEntry>>,
    out_dir: &Path,
) -> Result<(), ConvertError> {
    let dir = out_dir.join("dicts");
    std::fs::create_dir_all(&dir)?;
    for (stem, entries) in domains {
        let name = DOMAIN_NAMES
            .iter()
            .find(|(key, _)| key == stem)
            .map_or(stem.as_str(), |(_, name)| name);
        let internet = stem.starts_with("internet_slang");
        let mut tsv = if internet {
            format!(
                "# 青简网络用语包：{name}，由 qingjian-dict-convert lexicon 从 assets/lexicon/04_internet_slang/ 打出（缺省关）。词\t音节\t词频\n# 来源：维基百科与维基词典（CC BY-SA 4.0）、CC-CEDICT（CC BY-SA 4.0）；读音 Unihan + 标注。\n"
            )
        } else {
            format!(
                "# 青简领域词库：{name}，由 qingjian-dict-convert lexicon 从 THUOCL 拆出，基础词库里没有的部分。词\t音节\t词频\n"
            )
        };
        for entry in entries.values() {
            tsv.push_str(&entry.text);
            tsv.push('\t');
            tsv.push_str(&entry.syllables.join(" "));
            tsv.push('\t');
            tsv.push_str(&entry.frequency.to_string());
            tsv.push('\n');
        }
        let tsv_path = dir.join(format!("{stem}.tsv"));
        std::fs::write(&tsv_path, &tsv)?;
        let dictionary = Dictionary::parse(&tsv)?;
        let metadata = Metadata {
            name: if internet {
                format!("青简网络用语包：{name}")
            } else {
                format!("青简领域词库：{name}")
            },
            license: if internet {
                INTERNET_LICENSE.to_owned()
            } else {
                DOMAIN_LICENSE.to_owned()
            },
            attribution: if internet {
                INTERNET_ATTRIBUTION.to_owned()
            } else {
                DOMAIN_ATTRIBUTION.to_owned()
            },
            source: if internet {
                INTERNET_SOURCE.to_owned()
            } else {
                DOMAIN_SOURCE.to_owned()
            },
            generator: format!("qingjian-dict-convert {}", env!("CARGO_PKG_VERSION")),
            ..Metadata::default()
        };
        let qj_path = dir.join(format!("{stem}.qj"));
        dictionary.write_qj(&qj_path, &metadata)?;
        tracing::info!(domain = %name, entries = entries.len(), path = %qj_path.display(), "领域词库已写出");
    }
    Ok(())
}
