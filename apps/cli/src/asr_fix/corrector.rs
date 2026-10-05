//! 按读音把识别结果里的同音、近音片段换回术语：滑窗比对每个字的读音，同音不扣分、模糊音扣 1，
//! 在预算内的都是候选，按读音差小、术语长优先挑互不重叠的。

use super::fix::Fix;
use super::options::Options;
use super::phonetic::Phonetic;
use super::term::Term;
use super::text::is_han;

/// 术语词频高过这个就算常用词（词库多字词的前 15% 左右；「统一」1 万多，「基线」两百多）。
const COMMON_TERM_FREQUENCY: u32 = 2000;

pub struct Corrector {
    terms: Vec<Term>,
}

/// 一句的纠正结果。
pub struct Correction {
    pub text: String,

    /// 换了的。
    pub applied: Vec<Fix>,

    /// 读音对得上、但原文是词库词或术语是常用词而没换的（宽松模式下为空）。
    pub held: Vec<Fix>,
}

impl Corrector {
    /// 建术语表（术语与它学自的会议，空为全局）；返回纠正器与读不出音（含非汉字或生僻字）而跳过的术语。
    pub fn new(phonetic: &Phonetic, terms: Vec<(String, Vec<String>)>) -> (Self, Vec<String>) {
        let mut kept = Vec::new();
        let mut skipped = Vec::new();
        for (text, scopes) in terms {
            let chars: Vec<char> = text.chars().collect();
            let readings = (chars.len() >= 2 && chars.iter().all(|c| is_han(*c)))
                .then(|| phonetic.term_readings(&text))
                .flatten();
            match readings {
                Some(readings) => kept.push(Term {
                    text,
                    chars,
                    readings,
                    scopes,
                }),
                None => skipped.push(text),
            }
        }
        (Self { terms: kept }, skipped)
    }

    pub fn terms(&self) -> impl Iterator<Item = &str> {
        self.terms.iter().map(|term| term.text.as_str())
    }

    /// 纠一句；`group` 是这句所属的会议（留一场会议评测用）。
    pub fn correct(
        &self,
        phonetic: &Phonetic,
        text: &str,
        group: Option<&str>,
        options: Options,
    ) -> Correction {
        let chars: Vec<char> = text.chars().collect();
        let mut found = Vec::new();
        for term in &self.terms {
            if options.holdout
                && let Some(group) = group
                && !term.usable_in(group)
            {
                continue;
            }
            let n = term.chars.len();
            for start in 0..chars.len().saturating_sub(n - 1) {
                let window = &chars[start..start + n];
                if window == term.chars.as_slice() || !window.iter().all(|c| is_han(*c)) {
                    continue;
                }
                if Self::cuts_a_word(phonetic, &chars, start, start + n) {
                    continue;
                }
                let Some(cost) = Self::cost(phonetic, term, window) else {
                    continue;
                };
                if cost > term.budget() {
                    continue;
                }
                let original: String = window.iter().collect();
                let is_word = phonetic.word_frequency(&original).is_some();
                let common_term = phonetic
                    .word_frequency(&term.text)
                    .is_some_and(|frequency| frequency > COMMON_TERM_FREQUENCY);
                found.push(Fix {
                    start,
                    original,
                    term: term.text.clone(),
                    cost,
                    is_word,
                    common_term,
                });
            }
        }
        // 读音差小的先挑，同分时长术语优先（「青简输入法」盖过「青简」）
        found.sort_by(|a, b| {
            (a.cost, std::cmp::Reverse(a.len()), a.start).cmp(&(
                b.cost,
                std::cmp::Reverse(b.len()),
                b.start,
            ))
        });
        let mut applied: Vec<Fix> = Vec::new();
        let mut held: Vec<Fix> = Vec::new();
        for fix in found {
            let overlaps = |other: &Fix| fix.start < other.end() && other.start < fix.end();
            if applied.iter().any(overlaps) || held.iter().any(overlaps) {
                continue;
            }
            if fix.doubtful() && !options.loose {
                held.push(fix);
            } else {
                applied.push(fix);
            }
        }
        applied.sort_by_key(|fix| fix.start);
        held.sort_by_key(|fix| fix.start);
        let mut output = String::new();
        let mut cursor = 0;
        for fix in &applied {
            output.extend(&chars[cursor..fix.start]);
            output.push_str(&fix.term);
            cursor = fix.end();
        }
        output.extend(&chars[cursor..]);
        Correction {
            text: output,
            applied,
            held,
        }
    }

    /// 窗口边上的字与窗口外相邻的字连成词库词（「发【信】息」「上【海的】」「重【返华】盛顿」）：
    /// 那几个字本来属于别的词，换掉就把词拆了。
    fn cuts_a_word(phonetic: &Phonetic, chars: &[char], start: usize, end: usize) -> bool {
        const MAX_WORD: usize = 4;
        let crosses = |boundary: usize| {
            (boundary.saturating_sub(MAX_WORD - 1)..boundary).any(|from| {
                (boundary + 1..=(from + MAX_WORD).min(chars.len())).any(|to| {
                    let span = &chars[from..to];
                    span.iter().all(|c| is_han(*c))
                        && phonetic
                            .word_frequency(&span.iter().collect::<String>())
                            .is_some()
                })
            })
        };
        (start > 0 && crosses(start)) || (end < chars.len() && crosses(end))
    }

    /// 窗口与术语的读音差；有字读音对不上（既不同音也不是模糊音）返回 `None`。
    fn cost(phonetic: &Phonetic, term: &Term, window: &[char]) -> Option<u32> {
        let mut total = 0;
        for ((c, expected), wanted) in window.iter().zip(&term.readings).zip(&term.chars) {
            if c != wanted {
                total += phonetic.char_cost(*c, expected)?;
            }
        }
        Some(total)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use qingjian_dictionary::Dictionary;

    const SAMPLE: &str = "青\tqing\t900\n清\tqing\t800\n简\tjian\t900\n剪\tjian\t500\n\
        通\ttong\t900\n变\tbian\t900\n便\tbian\t800\n便\tpian\t300\n通便\ttong bian\t200\n\
        声\tsheng\t900\n生\tsheng\t900\n纹\twen\t500\n文\twen\t900\n门\tmen\t900\n门生\tmen sheng\t300\n";

    fn correct(terms: &[&str], text: &str, loose: bool) -> Correction {
        let dictionary = Dictionary::parse(SAMPLE).unwrap();
        let phonetic = Phonetic::new([&dictionary]);
        let terms = terms.iter().map(|t| (t.to_string(), Vec::new())).collect();
        let (corrector, _) = Corrector::new(&phonetic, terms);
        let options = Options {
            loose,
            ..Options::default()
        };
        corrector.correct(&phonetic, text, None, options)
    }

    #[test]
    fn replaces_homophones_with_the_term() {
        let result = correct(&["青简"], "我在用清剪打字", false);
        assert_eq!(result.text, "我在用青简打字");
        assert_eq!(result.applied.len(), 1);
    }

    #[test]
    fn holds_dictionary_words_unless_loose() {
        let held = correct(&["通变"], "模型叫通便", false);
        assert_eq!(held.text, "模型叫通便");
        assert_eq!(held.held.len(), 1);
        assert_eq!(correct(&["通变"], "模型叫通便", true).text, "模型叫通变");
    }

    #[test]
    fn leaves_windows_that_cut_a_neighbouring_word() {
        // 「门生」是词：「声纹」不能拿走「生」
        assert_eq!(correct(&["声纹"], "门生文", false).text, "门生文");
    }

    #[test]
    fn two_char_terms_need_exact_homophones() {
        // 门 men ↔ 纹 wen 不是模糊音，生 sheng 同音也不够
        assert_eq!(correct(&["声纹"], "生门", false).text, "生门");
        assert_eq!(correct(&["声纹"], "生文", false).text, "声纹");
    }
}
