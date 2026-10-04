//! 规则共用的替换循环：正则找候选，规则自己判定取舍，被拒的候选只前进一个字符，不吞掉后面可能的真匹配。

use regex::Regex;

use super::hit::Hit;

/// 把 `re` 的每个候选交给 `accept`，通过的换成 `placeholder`，返回新文字与替换数。
pub fn replace_where(
    text: &str,
    re: &Regex,
    placeholder: &str,
    accept: impl Fn(&Hit) -> bool,
) -> (String, u32) {
    replace_group(text, re, 0, placeholder, accept)
}

/// 同 [`replace_where`]，但只换正则里第 `group` 组（关键词留着，只换后面的号码）；候选的上下文也按这一组算。
pub fn replace_group(
    text: &str,
    re: &Regex,
    group: usize,
    placeholder: &str,
    accept: impl Fn(&Hit) -> bool,
) -> (String, u32) {
    let mut output = String::with_capacity(text.len());
    let mut copied = 0;
    let mut position = 0;
    let mut count = 0;
    while let Some(captures) = re.captures_at(text, position) {
        let whole = captures.get(0).expect("整体匹配");
        let Some(span) = captures.get(group) else {
            break;
        };
        let hit = Hit {
            found: span.as_str(),
            prefix: &text[..span.start()],
            suffix: &text[span.end()..],
        };
        if accept(&hit) {
            output.push_str(&text[copied..span.start()]);
            output.push_str(placeholder);
            copied = span.end();
            position = span.end();
            count += 1;
        } else {
            let step = text[whole.start()..]
                .chars()
                .next()
                .map_or(1, char::len_utf8);
            position = whole.start() + step;
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
