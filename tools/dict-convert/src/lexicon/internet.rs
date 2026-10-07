//! 网络用语包（`assets/lexicon/04_internet_slang/*.tsv`）：一行 `词\t拼音\t词频\t年份\t来源`。
//!
//! **一本包一个 TSV**，文件名主干就是包名：`internet_slang.tsv` 是干净那本，`internet_slang_coarse.tsv`
//! 是粗口那本 —— 两本分开靠分文件，不靠某一列的值，免得写错一个字就把脏话放进干净包里。
//!
//! 年份只解析、不参与打包：空值与 `unknown` 都当「不按年份卸载」（`None`），
//! 留着给以后的批量卸载用（现在还没有卸载机制，先如实读进来）。
//!
//! 拼音一律小写：源文件里写了大写（CC-CEDICT 的专名写法，`Tian chao`）直接报错，不悄悄小写化放过去 ——
//! 大写通常意味着这行是照抄来的专名拼音，得回去看一眼再决定收不收。

use std::path::Path;

use qingjian_dictionary::canonical_syllable;

use crate::error::ConvertError;

/// 没填词频时的兜底：弱到抢不了同音常用词的首选，但用户勾上这本包就能选到。
/// 语料到位后按「对着同音竞争词人工定」重写这一列（见 README）。
pub const DEFAULT_FREQUENCY: u32 = 100;

/// 网络用语源的一行。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    /// 词。
    pub text: String,

    /// 读音（空格分隔的音节）；给了就不再按字猜。
    pub syllables: Option<Vec<String>>,

    /// 词频；没填为 `None`（按 [`DEFAULT_FREQUENCY`] 兜底）。
    pub frequency: Option<u32>,

    /// 流行年份；空值与 `unknown` 都是 `None`。
    pub year: Option<u16>,
}

impl Row {
    /// 打包用的词频。
    pub fn frequency(&self) -> u32 {
        self.frequency.unwrap_or(DEFAULT_FREQUENCY).max(1)
    }
}

/// 读一本网络用语包（`path` 是那个 TSV）；交回 (包名, 行)。
pub fn load(path: &Path) -> Result<(String, Vec<Row>), ConvertError> {
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or_default()
        .to_owned();
    let source = std::fs::read_to_string(path)?;
    let mut rows = Vec::new();
    for (index, line) in source.lines().enumerate() {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut fields = line.split('\t');
        let text = fields.next().unwrap_or_default().trim().to_owned();
        if text.is_empty() {
            continue;
        }
        let raw = fields.next().unwrap_or_default();
        if raw.chars().any(|c| c.is_ascii_uppercase()) {
            return Err(ConvertError::Format {
                path: path.to_owned(),
                line: index + 1,
                reason: format!(
                    "网络用语「{text}」的拼音写成大写了（{raw}）：拼音一律小写；\
                     大写的多是照抄来的专名写法，回去确认这条收不收，不要直接放过"
                ),
            });
        }
        let syllables = Some(raw)
            .filter(|p| !p.trim().is_empty())
            .map(|p| {
                p.split_whitespace()
                    .map(|s| canonical_syllable(s).to_owned())
                    .collect::<Vec<_>>()
            })
            .filter(|s: &Vec<String>| !s.is_empty());
        let frequency = fields.next().and_then(|f| f.trim().parse().ok());
        let year = fields.next().and_then(parse_year);
        rows.push(Row {
            text,
            syllables,
            frequency,
            year,
        });
    }
    Ok((stem, rows))
}

/// 读整个目录（`04_internet_slang/`）：一个 TSV 一本包，按文件名排序（稳定）。
pub fn load_dir(dir: &Path) -> Result<Vec<(String, Vec<Row>)>, ConvertError> {
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut files: Vec<_> = std::fs::read_dir(dir)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension().and_then(|e| e.to_str()) == Some("tsv")
                && path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|name| !name.starts_with('.') && !name.starts_with('~'))
        })
        .collect();
    files.sort();
    let mut packs = Vec::new();
    for file in files {
        let (stem, rows) = load(&file)?;
        tracing::info!(pack = %stem, rows = rows.len(), file = %file.display(), "网络用语已读取");
        packs.push((stem, rows));
    }
    Ok(packs)
}

/// 年份那一列：空、`unknown`（不分大小写）都当没标。
fn parse_year(field: &str) -> Option<u16> {
    let field = field.trim();
    if field.is_empty() || field.eq_ignore_ascii_case("unknown") {
        return None;
    }
    field.parse().ok().filter(|year| *year > 1900)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(dir: &Path, name: &str, body: &str) -> std::path::PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, body).unwrap();
        path
    }

    fn temp_dir(tag: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("qingjian-internet-{tag}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn year_column_parses_and_unknown_is_none() {
        let dir = temp_dir("year");
        write(
            &dir,
            "internet_slang.tsv",
            "# 词\t拼音\t词频\t年份\t来源\n\
             内卷\tnei juan\t12000\t2021\tcedict\n\
             真香\tzhen xiang\t\tunknown\twikipedia\n\
             破防\tpo fang\t\t\twikipedia\n",
        );
        let (_, rows) = load(&dir.join("internet_slang.tsv")).unwrap();
        assert_eq!(rows[0].year, Some(2021), "填了年份就按年份");
        assert_eq!(rows[1].year, None, "unknown 当没标");
        assert_eq!(rows[2].year, None, "空值当没标");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn frequency_is_optional_with_a_floor() {
        let dir = temp_dir("freq");
        write(
            &dir,
            "internet_slang.tsv",
            "内卷\tnei juan\t12000\t2021\tcedict\n\
             丈育\t\t\t\twiktionary\n",
        );
        let (_, rows) = load(&dir.join("internet_slang.tsv")).unwrap();
        assert_eq!(rows[0].frequency, Some(12000));
        assert_eq!(rows[0].frequency(), 12000);
        assert_eq!(rows[1].frequency, None, "没填词频就是 None");
        assert_eq!(rows[1].frequency(), DEFAULT_FREQUENCY, "兜底给弱的底值");
        assert_eq!(rows[1].syllables, None, "没填拼音就按字推");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn uppercase_pinyin_is_rejected_not_silently_lowercased() {
        let dir = temp_dir("upper");
        write(
            &dir,
            "internet_slang.tsv",
            "天朝\tTian chao\t100\tunknown\twiktionary\n",
        );
        let error = load(&dir.join("internet_slang.tsv")).unwrap_err();
        let text = format!("{error}");
        assert!(text.contains("天朝"), "{text}");
        assert!(text.contains("大写"), "{text}");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn each_file_is_its_own_pack_so_coarse_stays_separate() {
        let dir = temp_dir("split");
        write(
            &dir,
            "internet_slang.tsv",
            "内卷\tnei juan\t12000\t2021\tcedict\n",
        );
        write(
            &dir,
            "internet_slang_coarse.tsv",
            "卧槽\two cao\t\t\twiktionary\n",
        );
        write(&dir, "README.md", "不是 TSV，不该被读进来\n");
        let packs = load_dir(&dir).unwrap();
        let names: Vec<&str> = packs.iter().map(|(name, _)| name.as_str()).collect();
        assert_eq!(names, ["internet_slang", "internet_slang_coarse"]);
        assert_eq!(packs[0].1[0].text, "内卷");
        assert_eq!(packs[1].1[0].text, "卧槽", "粗口只在粗口那本里");
        assert!(!packs[0].1.iter().any(|row| row.text == "卧槽"));
        std::fs::remove_dir_all(dir).unwrap();
    }
}
