//! 座机：`01x` / `02x` 区号加 8 位，或 `03xx`–`09xx` 区号加 7、8 位，区号后可有连字符。
//! 区号限定后，`00` 开头的订单号之类不会被当成座机。

use std::sync::LazyLock;

use regex::Regex;

use super::replace::{replace_where, touches_digit};

const PLACEHOLDER: &str = "〔电话〕";

static LANDLINE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"0[12]\d-?\d{8}|0[3-9]\d{2}-?\d{7,8}").expect("座机正则"));

pub fn apply(text: &str) -> (String, u32) {
    replace_where(text, &LANDLINE, PLACEHOLDER, |_, before, after| {
        !touches_digit(before, after)
    })
}
