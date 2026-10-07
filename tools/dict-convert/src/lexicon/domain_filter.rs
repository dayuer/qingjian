//! 领域包的第一层清洗（词库分层第 1 步的规则，见 `docs/plan/dictionary-layering.md`）：
//! 长度门槛、寄主 + 学名与机构法规名、地名的县界与著名度、诗词名句整本移出词库。
//!
//! 这一层只做「判去哪、判死」，不动读音与词频；规则是纯函数，同一份输入永远出同一份输出。
//! 白名单（固定书名这类必须留的）在 `assets/lexicon/00_meta/domain-keep.tsv`，只增，改它算改数据不算改规则。

use std::collections::HashSet;
use std::path::Path;

use crate::error::ConvertError;

/// 领域包默认丢掉超过这个字数的词（成语与白名单除外）。
pub const MAX_CHARS: usize = 6;

/// 地名包里「著名地点」的文档频次门槛（`--places-min-df` 的缺省值）。
pub const DEFAULT_PLACES_MIN_DF: u64 = 500;

/// 寄主（单字）+ 学名那种拼接词的宿主字。
const HOSTS: &str = "人猪牛羊犬猫鸡鸭鹅鼠马鱼虾蟹蛇蛙鸽蚕蜂兔猴鹿狐貂貉豕禽畜螺蚊蝇蜱蚤虱";

/// 学名（寄生虫、病原体）那半截常见的收尾。
const TAXON_TAILS: &[&str] = &[
    "虫",
    "菌",
    "毒",
    "病",
    "蚴",
    "蜱",
    "螨",
    "虱",
    "蚤",
    "蝇",
    "蚊",
    "螺",
    "绦",
    "杆菌",
    "球菌",
    "病毒",
    "螺旋体",
    "立克次体",
];

/// 行政区划的尾巴（县级及以上：省 / 市 / 县 / 区 / 镇 / 乡 / 各级自治区）。
const ADMIN_TAILS: &[&str] = &[
    "省",
    "市",
    "县",
    "区",
    "镇",
    "乡",
    "自治区",
    "自治州",
    "自治县",
    "特别行政区",
    "盟",
    "旗",
];

/// 白名单。
pub struct Whitelist {
    words: HashSet<String>,
}

impl Whitelist {
    /// 读白名单；文件不存在当空的（新克隆的仓库里还没建）。
    pub fn load(path: &Path) -> Result<Self, ConvertError> {
        let mut words = HashSet::new();
        if path.exists() {
            for line in std::fs::read_to_string(path)?.lines() {
                let word = line.trim();
                if word.is_empty() || word.starts_with('#') {
                    continue;
                }
                words.insert(word.to_owned());
            }
        }
        Ok(Self { words })
    }

    pub fn contains(&self, word: &str) -> bool {
        self.words.contains(word)
    }
}

/// 这条词改派到哪。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sink {
    /// 丢掉（长机构名、寄主 + 学名这类）。
    Drop,

    /// 换一本包（诗词名句、小地名）。
    Pack(&'static str),
}

/// 为什么（报告里按它归类，便于逐条复核）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reason {
    /// R1 超过 6 个字。
    TooLong,

    /// R2 寄主 + 学名。
    HostAndTaxon,

    /// R3 小地名（社区、巷弄、村组、以及不够著名的地点）。
    MinorPlace,

    /// R4 诗词名句：整本移出词库，进名句包。
    Poetry,
}

impl Reason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::TooLong => "R1 长度",
            Self::HostAndTaxon => "R2 寄主学名",
            Self::MinorPlace => "R3 小地名",
            Self::Poetry => "R4 名句",
        }
    }
}

/// 判一条领域词。交回 `None` 表示照旧（按语料次数决定进基础词库还是本包）。
pub fn judge(
    stem: &str,
    word: &str,
    df: u64,
    places_min_df: u64,
    keep: &Whitelist,
) -> Option<(Sink, Reason)> {
    // R4：诗词名句整本从词库拿出去（含语料里 ≥ 50 次、本来会留在基础词库的那部分）
    if stem == "poetry_lines" {
        return Some((Sink::Pack("poetry_lines"), Reason::Poetry));
    }
    if keep.contains(word) {
        return None;
    }
    // R1：长度门槛。成语整本不看长度 —— 成语再长也是成语
    if stem != "idioms" && word.chars().count() > MAX_CHARS {
        return Some((Sink::Drop, Reason::TooLong));
    }
    // R2：动物、医学里的寄主 + 学名（`犬复孔绦虫`、`牛带绦虫`）。
    // 长机构名（`疾病预防控制中心`）不用另设规则：R1 的长度门槛已经把它们全删了；
    // 而按「局 / 院 / 部」这类后缀删会误伤 医院、卫生局、颈部 这种常用词。
    if matches!(stem, "animals" | "medicine") && is_host_and_taxon(word) {
        return Some((Sink::Drop, Reason::HostAndTaxon));
    }
    // R3：地名只留县级及以上与著名地点，其余进 places-extended（缺省关）
    if stem == "places" && !is_major_place(word, df, places_min_df) {
        return Some((Sink::Pack("places-extended"), Reason::MinorPlace));
    }
    None
}

/// 寄主（单字）+ 学名：`犬复孔绦虫`、`牛带绦虫`、`猫抓病`、`猪链球菌病` 这类。
fn is_host_and_taxon(word: &str) -> bool {
    let mut chars = word.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !HOSTS.contains(first) {
        return false;
    }
    // 宿主那半截是单字，后面至少两字才算「拼接」
    word.chars().count() >= 3 && has_tail(word, TAXON_TAILS)
}

/// 地名留不留：县级及以上的短名，或文档频次够高的著名地点。
fn is_major_place(word: &str, df: u64, places_min_df: u64) -> bool {
    let count = word.chars().count();
    if count <= MAX_CHARS && has_tail(word, ADMIN_TAILS) {
        return true;
    }
    df >= places_min_df
}

/// 词尾是不是这些尾巴之一。
fn has_tail(word: &str, tails: &[&str]) -> bool {
    tails.iter().any(|tail| word.ends_with(tail))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn whitelist(words: &[&str]) -> Whitelist {
        Whitelist {
            words: words.iter().map(|w| (*w).to_owned()).collect(),
        }
    }

    #[test]
    fn long_words_go_first_but_idioms_stay() {
        let keep = whitelist(&[]);
        assert_eq!(
            judge("law", "中华人民共和国公司法", 900, 500, &keep),
            Some((Sink::Drop, Reason::TooLong))
        );
        // 成语整本不看长度
        assert_eq!(judge("idioms", "一个巴掌拍不响", 900, 500, &keep), None);
        // 白名单能保下一条长专名
        assert_eq!(
            judge(
                "law",
                "中华人民共和国公司法",
                900,
                500,
                &whitelist(&["中华人民共和国公司法"])
            ),
            None
        );
    }

    #[test]
    fn host_and_taxon_and_institutions_go() {
        let keep = whitelist(&[]);
        for word in ["犬复孔绦虫", "牛带绦虫", "猪链球菌病", "猫抓病"] {
            assert_eq!(
                judge("animals", word, 100, 500, &keep),
                Some((Sink::Drop, Reason::HostAndTaxon)),
                "{word}"
            );
        }
        // 长机构名靠 R1 的长度门槛删，不另设后缀规则（否则 医院 / 卫生局 会被误伤）
        assert_eq!(
            judge("medicine", "疾病预防控制中心", 100, 500, &keep),
            Some((Sink::Drop, Reason::TooLong))
        );
        assert_eq!(judge("medicine", "人民医院", 500000, 500, &keep), None);
        assert_eq!(judge("medicine", "卫生局", 27235, 500, &keep), None);
        // 寄主 + 学名只在动物、医学两本里套
        assert_eq!(judge("law", "牛带绦虫", 100, 500, &keep), None);
    }

    #[test]
    fn places_split_by_level_and_fame() {
        let keep = whitelist(&[]);
        // 县级及以上短名留下
        assert_eq!(judge("places", "张家港市", 100, 500, &keep), None);
        // 著名地点按文档频次留下
        assert_eq!(judge("places", "曹家巷", 900, 500, &keep), None);
        // 小地名进扩展包
        assert_eq!(
            judge("places", "曹家巷", 12, 500, &keep),
            Some((Sink::Pack("places-extended"), Reason::MinorPlace))
        );
        // 超过 6 字的地名直接删（比 6 字长的行政区划全名不该整块打）
        assert_eq!(
            judge(
                "places",
                "江西武夷山国家级自然保护区管理局",
                700,
                500,
                &keep
            ),
            Some((Sink::Drop, Reason::TooLong))
        );
    }

    #[test]
    fn poetry_lines_pack_always_moves_out() {
        let keep = whitelist(&[]);
        assert_eq!(
            judge("poetry_lines", "更上一层楼", 9999, 500, &keep),
            Some((Sink::Pack("poetry_lines"), Reason::Poetry))
        );
    }
}
