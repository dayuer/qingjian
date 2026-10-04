//! 银行卡：13–19 位数字（可按 4 位一组用空格或连字符分开）且 Luhn 通过。
//! 带国际区号的手机号、毫秒时间戳（13 位、1 开头）不算卡号。

use std::sync::LazyLock;

use regex::Regex;

use super::phone::is_phone_with_prefix;
use super::replace::{luhn_ok, replace_where, touches_digit};

const PLACEHOLDER: &str = "〔卡号〕";

static BANK_CARD: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\d{4}(?:[ -]\d{4}){2,3}(?:[ -]\d{1,3})?|\d{13,19}").expect("银行卡正则")
});

pub fn apply(text: &str) -> (String, u32) {
    replace_where(text, &BANK_CARD, PLACEHOLDER, |found, before, after| {
        if touches_digit(before, after) || before == Some('+') {
            return false;
        }
        let digits: String = found.chars().filter(char::is_ascii_digit).collect();
        (13..=19).contains(&digits.len())
            && luhn_ok(&digits)
            && !is_phone_with_prefix(&digits)
            && !(digits.len() == 13 && digits.starts_with('1'))
    })
}
