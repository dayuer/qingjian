//! 凭据：密码 / 口令 / pin / pwd / password 后面的 4–32 位 ASCII 可见字符串，
//! 和 验证码 / 动态码 / 校验码 后面的 4–8 位字母数字，整串换成〔凭据〕，关键词与连接词留着。
//!
//! 为了不把中文句子吞掉：串必须是 ASCII，遇到中文就停，停下后不足 4 位不换（「密码是不是忘了」「验证码是什么」都不动）；
//! 英文关键词前后要有词边界（`spinning 1234`、`pinned message` 不命中）。
//! 关键词与串之间有冒号或「是 / 为 / 改成 / 改为」时，任何串都换；只隔空白时，串里要有数字或符号才换，
//! 免得 `password reset link`、`密码 abcdefgh` 这类普通说法被换掉。

use std::sync::LazyLock;

use regex::Regex;

use super::replace::replace_group;

const PLACEHOLDER: &str = "〔凭据〕";

/// 关键词之后的连接：冒号、空白、是 / 为 / 改成 / 改为，可连着出现。
const LINK_WORDS: &str = r"(?:是|为|改成|改为)";

/// 凭据关键词（中文不要边界；英文大小写不敏感且前后要词边界，`(?-u:\b)` 让中文算非词字符）。
const KEYWORD: &str = r"(?:密码|口令|(?i:(?-u:\b)(?:pin|pwd|password)(?-u:\b)))";

/// 4–32 位 ASCII 可见字符。
const SECRET: &str = r"([\x21-\x7e]{4,32})";

/// 与关键词只隔空白时的串：不能以 `:` 起头（那是冒号连接，交给上一条规则）。
const PLAIN_SECRET: &str = r"([\x21-\x39\x3b-\x7e][\x21-\x7e]{3,31})";

static EXPLICIT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(
        r"{KEYWORD}(?:\s*[：:][：:\s]*(?:{LINK_WORDS}[：:\s]*)*|\s*(?:{LINK_WORDS}[：:\s]*)+){SECRET}"
    ))
    .expect("凭据正则")
});

static PLAIN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(
        r"(?:(?:密码|口令)\s*|(?i:(?-u:\b)(?:pin|pwd|password)(?-u:\b))\s+){PLAIN_SECRET}"
    ))
    .expect("凭据正则")
});

static CODE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(
        r"(?:验证码|动态码|校验码)[：:\s]*(?:{LINK_WORDS}[：:\s]*)*([A-Za-z0-9]{{4,8}})"
    ))
    .expect("验证码正则")
});

pub fn apply(text: &str) -> (String, u32) {
    let (text, explicit) = replace_group(text, &EXPLICIT, 1, PLACEHOLDER, |hit| {
        !hit.after().is_some_and(is_visible_ascii)
    });
    let (text, plain) = replace_group(&text, &PLAIN, 1, PLACEHOLDER, |hit| {
        !hit.after().is_some_and(is_visible_ascii)
            && hit.found.chars().any(|c| !c.is_ascii_alphabetic())
    });
    let (text, code) = replace_group(&text, &CODE, 1, PLACEHOLDER, |hit| {
        !hit.after().is_some_and(|c| c.is_ascii_alphanumeric())
    });
    (text, explicit + plain + code)
}

fn is_visible_ascii(c: char) -> bool {
    ('\x21'..='\x7e').contains(&c)
}
