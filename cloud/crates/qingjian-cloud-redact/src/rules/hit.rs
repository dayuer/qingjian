//! 一个候选命中的位置与上下文，规则据此决定取不取。

pub struct Hit<'a> {
    /// 候选本身。
    pub found: &'a str,

    /// 候选之前的全部文字。
    pub prefix: &'a str,

    /// 候选之后的全部文字。
    pub suffix: &'a str,
}

impl Hit<'_> {
    pub fn before(&self) -> Option<char> {
        self.prefix.chars().next_back()
    }

    pub fn after(&self) -> Option<char> {
        self.suffix.chars().next()
    }

    /// 前后紧挨数字：候选是更长数字串的一部分。
    pub fn touches_digit(&self) -> bool {
        self.before().is_some_and(|c| c.is_ascii_digit())
            || self.after().is_some_and(|c| c.is_ascii_digit())
    }

    /// 隔着一个空格或连字符还连着更多数字（至少两位）：候选只是分组数字串的一段。
    pub fn joins_more_digits(&self) -> bool {
        let digits_after = self
            .suffix
            .strip_prefix([' ', '-'])
            .is_some_and(|rest| rest.chars().take(2).filter(char::is_ascii_digit).count() == 2);
        let digits_before = self.prefix.strip_suffix([' ', '-']).is_some_and(|rest| {
            rest.chars()
                .rev()
                .take(2)
                .filter(char::is_ascii_digit)
                .count()
                == 2
        });
        digits_after || digits_before
    }
}
