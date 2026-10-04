//! 邮箱：本地部分加带点的域名，域名末段至少两个字母。

use std::sync::LazyLock;

use regex::Regex;

use super::replace::replace_where;

const PLACEHOLDER: &str = "〔邮箱〕";

static EMAIL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"[A-Za-z0-9._%+\-]+@[A-Za-z0-9\-]+(?:\.[A-Za-z0-9\-]+)*\.[A-Za-z]{2,}")
        .expect("邮箱正则")
});

pub fn apply(text: &str) -> (String, u32) {
    replace_where(text, &EMAIL, PLACEHOLDER, |_| true)
}
