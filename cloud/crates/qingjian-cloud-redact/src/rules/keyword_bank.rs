//! 有「卡号」「银行卡」「账号」「账户」等关键词紧跟 13–19 位数字（可 4 位一组分开）时，Luhn 不过也换。
//! 没有关键词的号码仍走 [`super::bank_card`] 的 Luhn 校验。

use std::sync::LazyLock;

use regex::Regex;

use super::keyword::SEPARATOR;
use super::replace::replace_group;

const PLACEHOLDER: &str = "〔卡号〕";

static KEYWORD_BANK: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(
        r"(?:银行卡号|银行卡|卡号|账号|账户){SEPARATOR}(\d{{4}}(?:[ -]\d{{4}}){{2,3}}(?:[ -]\d{{1,3}})?|\d{{13,19}})"
    ))
    .expect("卡号关键词正则")
});

pub fn apply(text: &str) -> (String, u32) {
    replace_group(text, &KEYWORD_BANK, 1, PLACEHOLDER, |hit| {
        let digits = hit.found.chars().filter(char::is_ascii_digit).count();
        (13..=19).contains(&digits) && !hit.after().is_some_and(|c| c.is_ascii_digit())
    })
}
