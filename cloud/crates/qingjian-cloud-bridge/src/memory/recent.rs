//! 最近上屏的 [`RECENT_CHARS`] 个字：提示拿它加当前首选去匹配卡片。只在内存里，换对象时清空。

use std::collections::VecDeque;

use super::RECENT_CHARS;

#[derive(Debug, Clone, Default)]
pub struct RecentText {
    chars: VecDeque<char>,
}

impl RecentText {
    pub fn push_str(&mut self, text: &str) {
        for c in text.chars() {
            if self.chars.len() == RECENT_CHARS {
                self.chars.pop_front();
            }
            self.chars.push_back(c);
        }
    }

    pub fn text(&self) -> String {
        self.chars.iter().collect()
    }

    pub fn clear(&mut self) {
        self.chars.clear();
    }
}
