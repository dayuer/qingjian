//! 干净回放口径（`--replay --clean`）：剔除日志里能机器判定为「当时选错了」的上屏再算命中率。
//!
//! 规则 2026-10-07 与审计会话定死，以后不改；要改就另开一个口径并重报基线（记录在 `docs/notes/replay-clean.md`）：
//! - R1 撤销：`retract` 的 `of` 指向此前最近一条同 `id` 的上屏（日志的 `id` 每个会话从头计），剔那一条。
//! - R2 上屏后删掉重打：`retype` 的 `of` 指向的上屏，其作用域等于 `before` 且 `before != after` 时剔它。
//!   组句内的重打 `of` 指的是这次上屏本身、作用域等于 `after`，不剔（见 core 的 `InputLogEntry::Retype`）。
//! - R3 汉字夹短字母：上屏文字既有汉字又有字母、每段连续字母不超过两个（没打完的拼音尾巴混进了上屏：先IE、修改U），
//!   但整段能由词库词拼成的不剔（C盘、U盘、T恤，中英混杂词已并进基础词库）。
//!
//! 只是统计时剔，回放本身照常逐条走完，上文与内存学习和原始口径完全一样。

mod rule;
mod summary;

use std::collections::HashMap;

use qingjian_core::InputLogEntry;

pub use rule::Rule;
pub use summary::Summary;

/// 词库里最长按几个字拼 R3 的例外。
const MAX_WORD_CHARS: usize = 12;

/// 扫一遍日志（`(行号, 条目)`，行号从 1 起），返回要剔的行号与理由。`is_word` 判断一段文字是不是词库词。
pub fn scan<'a>(
    entries: impl IntoIterator<Item = (usize, &'a InputLogEntry)>,
    is_word: impl Fn(&str) -> bool,
) -> HashMap<usize, Rule> {
    // id → (行号, 作用域)，同 id 只留最近一条
    let mut latest: HashMap<u64, (usize, &str)> = HashMap::new();
    let mut excluded = HashMap::new();
    for (line, entry) in entries {
        match entry {
            InputLogEntry::Commit(commit) => {
                let scope = if commit.scope.is_empty() {
                    commit.keys.as_str()
                } else {
                    commit.scope.as_str()
                };
                latest.insert(commit.id, (line, scope));
                if latin_tail(&commit.text, &is_word) {
                    excluded.entry(line).or_insert(Rule::LatinTail);
                }
            }
            InputLogEntry::Retract { of, .. } => {
                if let Some(&(target, _)) = latest.get(of) {
                    excluded.entry(target).or_insert(Rule::Retracted);
                }
            }
            InputLogEntry::Retype { before, after, of } => {
                if let Some(&(target, scope)) = latest.get(of)
                    && scope == before
                    && before != after
                {
                    excluded.entry(target).or_insert(Rule::Retyped);
                }
            }
            _ => {}
        }
    }
    excluded
}

/// R3：既有汉字又有字母、每段连续字母不超过两个，而且不能由词库词拼成。
fn latin_tail(text: &str, is_word: impl Fn(&str) -> bool) -> bool {
    let has_han = text.chars().any(|c| ('\u{3400}'..='\u{9fff}').contains(&c));
    let mut runs = Vec::new();
    let mut current = 0usize;
    for c in text.chars() {
        if c.is_ascii_alphabetic() {
            current += 1;
        } else if current > 0 {
            runs.push(current);
            current = 0;
        }
    }
    if current > 0 {
        runs.push(current);
    }
    has_han && !runs.is_empty() && runs.iter().all(|&n| n <= 2) && !composed_of_words(text, is_word)
}

/// 整段能不能切成若干个词库词。
fn composed_of_words(text: &str, is_word: impl Fn(&str) -> bool) -> bool {
    let chars: Vec<char> = text.chars().collect();
    let mut reachable = vec![false; chars.len() + 1];
    reachable[0] = true;
    for start in 0..chars.len() {
        if !reachable[start] {
            continue;
        }
        for end in start + 1..=chars.len().min(start + MAX_WORD_CHARS) {
            let piece: String = chars[start..end].iter().collect();
            if is_word(&piece) {
                reachable[end] = true;
            }
        }
    }
    reachable[chars.len()]
}

#[cfg(test)]
mod tests;
