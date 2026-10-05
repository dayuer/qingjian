//! 名字的拼音首字母：通讯录按字母分组与右侧索引要用它（设计稿 02 的 2b）。
//!
//! 设计说「首字母由输入法的拼音引擎算，**App 不自己算**」，所以算在这儿——名字在
//! `contacts.json` 里，而词库只有键盘这一侧有（扩展包里），App 那侧拿不到。
//!
//! 算法：取名字的**第一个字**，在 [`SYLLABLES`] 里找它读得出的**第一个**音节，取那个音节的声母；
//! 音节本身没有声母（「阿」a、「安」an）时取音节首字母。多音字取音节表里排在前面的那个——
//! 只用来排序分组，不追求读对。
//!
//! 首字不是汉字时（外文名、数字、表情）按首字母大写走；实在算不出给 `None`，App 那边归到「#」。

use qingjian_core::parser::{INITIALS, SYLLABLES};
use qingjian_dictionary::{Dictionary, SyllablePattern};

/// 名字的首字母（大写）。名字为空或首字算不出读音时为 `None`。
pub fn initial_of(dictionary: &Dictionary, name: &str) -> Option<char> {
    let first = name.trim().chars().next()?;
    if first.is_ascii_alphabetic() {
        return Some(first.to_ascii_uppercase());
    }
    let syllable = SYLLABLES.iter().find(|s| reads(dictionary, first, s))?;
    // 声母取最长匹配：「zhang」要取 zh 不是 z
    let suffix = INITIALS
        .iter()
        .filter(|initial| syllable.starts_with(**initial))
        .max_by_key(|initial| initial.len());
    suffix
        .and_then(|initial| initial.chars().next())
        .or_else(|| syllable.chars().next())
        .map(|letter| letter.to_ascii_uppercase())
}

/// 词库里这个字有没有这个读音。
fn reads(dictionary: &Dictionary, ch: char, syllable: &str) -> bool {
    let pattern = [SyllablePattern {
        text: syllable,
        complete: true,
    }];
    let text = ch.to_string();
    dictionary
        .lookup_exact(&pattern)
        .iter()
        .any(|hit| hit.text == text)
}
