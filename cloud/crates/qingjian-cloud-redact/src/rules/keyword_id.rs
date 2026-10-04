//! 有「身份证」「证件号」等关键词紧跟 15 或 18 位数字时，校验位不对也换（用户写在这个词后面的就是证件号）。
//! 没有关键词的号码仍走 [`super::id_card`] 的校验，避免误伤订单号。

use std::sync::LazyLock;

use regex::Regex;

use super::keyword::SEPARATOR;
use super::replace::replace_group;

const PLACEHOLDER: &str = "〔证件号〕";

static KEYWORD_ID: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(
        r"(?:身份证号码|身份证号|身份证|证件号码|证件号){SEPARATOR}(\d{{17}}[\dXx]|\d{{15}})"
    ))
    .expect("证件号关键词正则")
});

pub fn apply(text: &str) -> (String, u32) {
    replace_group(text, &KEYWORD_ID, 1, PLACEHOLDER, |hit| {
        !hit.after().is_some_and(|c| c.is_ascii_digit())
    })
}
