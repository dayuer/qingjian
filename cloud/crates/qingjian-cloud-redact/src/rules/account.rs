//! 微信号与 QQ 号：只有跟在关键词（微信 / 微信号 / wx / WeChat，QQ / QQ号 / q号，大小写不敏感）后面才换。
//! 微信号是字母开头的 6–20 位字母数字下划线连字符，QQ 号是 5–11 位数字；
//! 关键词后面跟普通词（微信支付、微信群）或数字位数不对的都不动。

use std::sync::LazyLock;

use regex::Regex;

use super::keyword::SEPARATOR;
use super::replace::replace_group;

const PLACEHOLDER: &str = "〔账号〕";

/// ASCII 关键词前面要是词边界（`(?-u:\b)` 让中文算非词字符，「我的QQ」里的 QQ 才有边界）。
static WECHAT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(
        r"(?i)(?:微信号|微信|(?-u:\b)wechat|(?-u:\b)wx){SEPARATOR}([A-Za-z][A-Za-z0-9_\-]{{5,19}})"
    ))
    .expect("微信号正则")
});

static QQ: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(
        r"(?i)(?:(?-u:\b)qq号?|(?-u:\b)q号){SEPARATOR}(\d{{5,11}})"
    ))
    .expect("QQ 号正则")
});

pub fn apply(text: &str) -> (String, u32) {
    let (text, wechat) = replace_group(text, &WECHAT, 1, PLACEHOLDER, |hit| {
        !hit.after()
            .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    });
    let (text, qq) = replace_group(&text, &QQ, 1, PLACEHOLDER, |hit| {
        !hit.after().is_some_and(|c| c.is_ascii_digit())
    });
    (text, wechat + qq)
}
