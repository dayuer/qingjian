//! 大陆手机号：`1[3-9]` 开头共 11 位，可带 `+86` / `0086` / `86` 前缀，中间可用空格或连字符分成 3-4-4。
//! 前后紧挨数字的不算（那是更长数字串的一部分）。

use std::sync::LazyLock;

use regex::Regex;

use super::replace::{replace_where, touches_digit};

const PLACEHOLDER: &str = "〔手机号〕";

static PHONE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?:(?:\+86|0086)[ -]?|86)?1[3-9]\d[ -]?\d{4}[ -]?\d{4}").expect("手机号正则")
});

pub fn apply(text: &str) -> (String, u32) {
    replace_where(text, &PHONE, PLACEHOLDER, |_, before, after| {
        !touches_digit(before, after)
    })
}

/// 去掉国家区号前缀与分隔后是不是一个手机号（银行卡规则用它避开「+86 手机号」被当成 13 位卡号）。
pub fn is_phone_with_prefix(digits: &str) -> bool {
    let rest = digits
        .strip_prefix("0086")
        .or_else(|| digits.strip_prefix("86"))
        .unwrap_or(digits);
    rest.len() == 11
        && rest.starts_with('1')
        && matches!(rest.as_bytes()[1], b'3'..=b'9')
        && rest.len() != digits.len()
}
