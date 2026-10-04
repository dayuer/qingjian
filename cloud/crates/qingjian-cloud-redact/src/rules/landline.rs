//! 座机：`01x` / `02x` 区号加 8 位，或 `03xx`–`09xx` 区号加 7、8 位。
//! 区号可以带全角或半角括号，区号与号码之间、8 位号码中间可以有一个空格或连字符。
//! 区号限定后，`00` 开头的订单号之类不会被当成座机；隔着分隔符还连着更多数字的（分组的长数字串）不算。

use std::sync::LazyLock;

use regex::Regex;

use super::replace::replace_where;

const PLACEHOLDER: &str = "〔电话〕";

static LANDLINE: LazyLock<Regex> = LazyLock::new(|| {
    let area3 = r"(?:[(（]0[12]\d[)）]|0[12]\d)";
    let area4 = r"(?:[(（]0[3-9]\d{2}[)）]|0[3-9]\d{2})";
    Regex::new(&format!(
        r"{area3}[- ]?(?:\d{{8}}|\d{{4}}[- ]\d{{4}})|{area4}[- ]?(?:\d{{7,8}}|\d{{4}}[- ]\d{{4}}|\d{{3}}[- ]\d{{4}})"
    ))
    .expect("座机正则")
});

pub fn apply(text: &str) -> (String, u32) {
    replace_where(text, &LANDLINE, PLACEHOLDER, |hit| {
        !hit.touches_digit() && !hit.joins_more_digits()
    })
}
