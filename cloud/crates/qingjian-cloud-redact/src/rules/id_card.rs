//! 身份证：18 位要过出生日期与校验位，15 位（旧证）要过出生日期。

use std::sync::LazyLock;

use regex::Regex;

use super::replace::{replace_where, touches_digit};

const PLACEHOLDER: &str = "〔证件号〕";

static ID_CARD: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\d{17}[\dXx]|\d{15}").expect("身份证正则"));

/// 校验位的加权系数与对照表。
const WEIGHTS: [u32; 17] = [7, 9, 10, 5, 8, 4, 2, 1, 6, 3, 7, 9, 10, 5, 8, 4, 2];
const CHECK_CODES: &[u8; 11] = b"10X98765432";

pub fn apply(text: &str) -> (String, u32) {
    replace_where(text, &ID_CARD, PLACEHOLDER, |found, before, after| {
        !touches_digit(before, after) && is_id_card(found)
    })
}

fn is_id_card(text: &str) -> bool {
    let bytes = text.as_bytes();
    if bytes[0] == b'0' {
        return false;
    }
    match bytes.len() {
        18 => valid_date(&text[6..10], &text[10..12], &text[12..14]) && check_ok(bytes),
        15 => valid_date(&format!("19{}", &text[6..8]), &text[8..10], &text[10..12]),
        _ => false,
    }
}

fn valid_date(year: &str, month: &str, day: &str) -> bool {
    let (Ok(year), Ok(month), Ok(day)) = (
        year.parse::<u32>(),
        month.parse::<u32>(),
        day.parse::<u32>(),
    ) else {
        return false;
    };
    (1900..=2100).contains(&year) && (1..=12).contains(&month) && (1..=31).contains(&day)
}

fn check_ok(bytes: &[u8]) -> bool {
    let sum: u32 = bytes[..17]
        .iter()
        .zip(WEIGHTS)
        .map(|(digit, weight)| u32::from(digit - b'0') * weight)
        .sum();
    CHECK_CODES[(sum % 11) as usize] == bytes[17].to_ascii_uppercase()
}
