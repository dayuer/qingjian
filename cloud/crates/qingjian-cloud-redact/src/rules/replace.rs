//! 规则共用的替换循环：正则找候选，规则自己判定取舍，被拒的候选只前进一个字符，不吞掉后面可能的真匹配。

use regex::Regex;

/// 候选前后紧挨的字符，都按「是不是 ASCII 数字」判断「是更长数字串的一部分」。
pub fn touches_digit(before: Option<char>, after: Option<char>) -> bool {
    before.is_some_and(|c| c.is_ascii_digit()) || after.is_some_and(|c| c.is_ascii_digit())
}

/// 把 `re` 的每个候选交给 `accept(候选, 前一个字符, 后一个字符)`，通过的换成 `placeholder`，返回新文字与替换数。
pub fn replace_where(
    text: &str,
    re: &Regex,
    placeholder: &str,
    accept: impl Fn(&str, Option<char>, Option<char>) -> bool,
) -> (String, u32) {
    let mut output = String::with_capacity(text.len());
    let mut copied = 0;
    let mut position = 0;
    let mut count = 0;
    while let Some(found) = re.find_at(text, position) {
        let before = text[..found.start()].chars().next_back();
        let after = text[found.end()..].chars().next();
        if accept(found.as_str(), before, after) {
            output.push_str(&text[copied..found.start()]);
            output.push_str(placeholder);
            copied = found.end();
            position = found.end();
            count += 1;
        } else {
            let step = text[found.start()..]
                .chars()
                .next()
                .map_or(1, char::len_utf8);
            position = found.start() + step;
        }
        if position >= text.len() {
            break;
        }
    }
    output.push_str(&text[copied..]);
    (output, count)
}

/// Luhn 校验（只含数字）。
pub fn luhn_ok(digits: &str) -> bool {
    let mut sum = 0;
    for (index, c) in digits.chars().rev().enumerate() {
        let Some(mut digit) = c.to_digit(10) else {
            return false;
        };
        if index % 2 == 1 {
            digit *= 2;
            if digit > 9 {
                digit -= 9;
            }
        }
        sum += digit;
    }
    sum % 10 == 0
}
