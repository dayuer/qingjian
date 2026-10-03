//! 大模型给的拼音的规整与校验：音节必须合法、个数等于汉字数；用户实际敲过的键要能由这些音节拼出（全拼或简拼混用）。

use qingjian_core::parser::is_syllable;

/// 规整成输入法词库的写法：小写、ü 写 v、去声调数字，音节空格分隔；有不合法的音节返回 `None`。
pub fn normalize(pinyin: &str) -> Option<Vec<String>> {
    let syllables: Vec<String> = pinyin
        .replace(['\'', '-', ','], " ")
        .split_whitespace()
        .map(|s| {
            s.to_lowercase()
                .replace("u:", "v")
                .chars()
                .map(strip_tone)
                .filter(|c| c.is_ascii_lowercase())
                .collect::<String>()
        })
        .map(|s| s.replace("lue", "lve").replace("nue", "nve"))
        .collect();
    if syllables.is_empty() || !syllables.iter().all(|s| is_syllable(s)) {
        return None;
    }
    Some(syllables)
}

/// 带声调的元音换成不带的，ü 一族换成 v。
fn strip_tone(c: char) -> char {
    match c {
        'ā' | 'á' | 'ǎ' | 'à' => 'a',
        'ē' | 'é' | 'ě' | 'è' => 'e',
        'ī' | 'í' | 'ǐ' | 'ì' => 'i',
        'ō' | 'ó' | 'ǒ' | 'ò' => 'o',
        'ū' | 'ú' | 'ǔ' | 'ù' => 'u',
        'ü' | 'ǖ' | 'ǘ' | 'ǚ' | 'ǜ' => 'v',
        other => other,
    }
}

/// 全是汉字（基本区与扩展 A）。
pub fn is_han(text: &str) -> bool {
    !text.is_empty()
        && text
            .chars()
            .all(|c| matches!(c, '\u{4e00}'..='\u{9fff}' | '\u{3400}'..='\u{4dbf}'))
}

/// 拼音能不能对上这个词：合法、个数与字数一致。
pub fn fits_text(syllables: &[String], text: &str) -> bool {
    is_han(text) && syllables.len() == text.chars().count()
}

/// 敲的键（去掉 `'`）能不能由这些音节依次拼出：每个音节贡献完整拼写或一段非空前缀（简拼）。
pub fn matches_keys(syllables: &[String], keys: &str) -> bool {
    let keys: Vec<char> = keys.chars().filter(|c| *c != '\'').collect();
    // reachable[i]：前 i 个键已被前若干音节拼出
    let mut reachable = vec![false; keys.len() + 1];
    reachable[0] = true;
    for syllable in syllables {
        let syllable: Vec<char> = syllable.chars().collect();
        let mut next = vec![false; keys.len() + 1];
        for start in 0..=keys.len() {
            if !reachable[start] {
                continue;
            }
            for len in 1..=syllable.len() {
                if start + len > keys.len() || keys[start + len - 1] != syllable[len - 1] {
                    break;
                }
                next[start + len] = true;
            }
        }
        reachable = next;
    }
    reachable[keys.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(list: &[&str]) -> Vec<String> {
        list.iter().map(|x| (*x).to_owned()).collect()
    }

    #[test]
    fn normalizes_and_validates() {
        assert_eq!(normalize("Qīng jiǎn").unwrap(), ["qing", "jian"]);
        assert_eq!(normalize("lü4 se4").unwrap(), ["lv", "se"]);
        assert_eq!(normalize("nue").unwrap(), ["nve"]);
        assert!(normalize("qing jjj").is_none());
        assert!(normalize("").is_none());
    }

    #[test]
    fn keys_match_full_and_abbreviated() {
        assert!(matches_keys(&s(&["qing", "jian"]), "qingjian"));
        assert!(matches_keys(&s(&["qing", "jian"]), "qing'jian"));
        assert!(matches_keys(&s(&["qing", "jian"]), "qj"));
        assert!(matches_keys(&s(&["qing", "jian"]), "qingj"));
        assert!(!matches_keys(&s(&["qing", "jian"]), "qingjia n"));
        assert!(!matches_keys(&s(&["qing", "jian"]), "jianqing"));
        assert!(!matches_keys(&s(&["qing", "jian"]), "qingjianx"));
    }

    #[test]
    fn text_must_be_han_with_matching_length() {
        assert!(fits_text(&s(&["qing", "jian"]), "青简"));
        assert!(!fits_text(&s(&["qing"]), "青简"));
        assert!(!fits_text(&s(&["a", "b"]), "AB"));
    }
}
