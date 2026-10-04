//! 链接：`http(s)://` 或 `www.` 开头，到空白、中文或结尾标点为止。

use std::sync::LazyLock;

use regex::Regex;

use super::replace::replace_where;

const PLACEHOLDER: &str = "〔链接〕";

static URL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(?:https?://|www\.)[A-Za-z0-9\-._~:/?#@!$&*+;=%]*[A-Za-z0-9\-_~/#@$&*+=%]")
        .expect("链接正则")
});

pub fn apply(text: &str) -> (String, u32) {
    replace_where(text, &URL, PLACEHOLDER, |_, _, _| true)
}
