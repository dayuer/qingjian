//! `gloss-gen pinyin` 写出的 JSONL：每行 `{"word":"重庆","pinyin":["chong","qing"]}`。
//!
//! 可选的 `"keep_both": true` 表示**两个读音都是规范读音、又都常用**（谁 shui/shei、重装 chong/zhong、
//! 必得 bide/bidei，以及单字条目）：这种不降权，两个读音同权保留。只有旧读音本身是错的
//! （且 ju、一宿 su、乐感 le）才按除数降权 —— 默认 [`super::DISPUTED_READING_DIVISOR`]，
//! 个别情况可以在行里写 `"divisor": 2` 单独放轻（露 lou/lu 这类口语词）。

use std::collections::HashMap;
use std::path::Path;

use qingjian_dictionary::canonical_syllable;
use serde::Deserialize;

use crate::error::ConvertError;

#[derive(Debug, Deserialize)]
struct Row {
    word: String,

    pinyin: Vec<String>,

    /// 两个读音都是规范读音且都常用：不降权，同权保留。
    #[serde(default)]
    keep_both: bool,

    /// 旧读音的降权倍数；只在不是 `keep_both` 时用，没写用默认除数。
    #[serde(default)]
    divisor: Option<u32>,
}

/// 一条标注：主读音，以及旧读音怎么处理。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Annotation {
    /// 主读音（规范读音），按字对齐。
    pub syllables: Vec<String>,

    /// 见文件头：两个读音同权。
    pub keep_both: bool,

    /// 旧读音的降权倍数；`None` 用默认除数。
    pub divisor: Option<u32>,
}

impl Annotation {
    /// 旧读音的降权倍数：`keep_both` 时是 1（同权），否则用标注里的值、没写用 `default`。
    pub fn divisor(&self, default: u32) -> u32 {
        if self.keep_both {
            1
        } else {
            self.divisor.unwrap_or(default).max(1)
        }
    }
}

/// 词 → 标注。同一个词以最后一条为准，坏行跳过。
pub fn load(path: &Path) -> Result<HashMap<String, Annotation>, ConvertError> {
    let source = std::fs::read_to_string(path)?;
    let mut out = HashMap::new();
    for line in source.lines() {
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<Row>(line) {
            Ok(mut row) => {
                for syllable in &mut row.pinyin {
                    *syllable = canonical_syllable(syllable).to_owned();
                }
                if row.keep_both && row.divisor.is_some() {
                    tracing::warn!(word = %row.word, "标了 keep_both 又写了 divisor，divisor 不生效");
                }
                out.insert(
                    row.word,
                    Annotation {
                        syllables: row.pinyin,
                        keep_both: row.keep_both,
                        divisor: row.divisor,
                    },
                );
            }
            Err(error) => tracing::warn!(%error, "拼音标注坏行，跳过"),
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load_body(tag: &str, body: &str) -> HashMap<String, Annotation> {
        let path = std::env::temp_dir().join(format!(
            "qingjian-annotations-{tag}-{}.jsonl",
            std::process::id()
        ));
        std::fs::write(&path, body).unwrap();
        let annotations = load(&path).unwrap();
        std::fs::remove_file(path).unwrap();
        annotations
    }

    #[test]
    fn normalizes_u_umlaut_spellings() {
        let annotations = load_body(
            "normalize",
            "{\"word\":\"策略\",\"pinyin\":[\"ce\",\"lue\"]}\n{\"word\":\"虐待\",\"pinyin\":[\"nue\",\"dai\"]}\n",
        );
        assert_eq!(annotations["策略"].syllables, ["ce", "lve"]);
        assert_eq!(annotations["虐待"].syllables, ["nve", "dai"]);
    }

    /// 默认降权（不给标记）与「同权」（keep_both）与「放轻」（divisor）三种。
    #[test]
    fn keep_both_and_divisor_control_the_downweight() {
        let annotations = load_body(
            "keep-both",
            "{\"word\":\"谁\",\"pinyin\":[\"shei\"]}\n\
             {\"word\":\"重装\",\"pinyin\":[\"chong\",\"zhuang\"],\"keep_both\":true}\n\
             {\"word\":\"露面\",\"pinyin\":[\"lu\",\"mian\"],\"divisor\":2}\n",
        );
        assert_eq!(annotations["谁"].divisor(8), 8, "没标记按默认除数");
        assert_eq!(annotations["重装"].divisor(8), 1, "keep_both 不降权");
        assert_eq!(annotations["露面"].divisor(8), 2, "标了倍数就按倍数");
        // keep_both 与 divisor 同时写时，keep_both 说了算
        let both = load_body(
            "both",
            "{\"word\":\"谁\",\"pinyin\":[\"shei\"],\"keep_both\":true,\"divisor\":2}\n",
        );
        assert_eq!(both["谁"].divisor(8), 1);
    }
}
