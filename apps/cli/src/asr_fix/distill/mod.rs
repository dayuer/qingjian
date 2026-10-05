//! 把「错的片段 → 对的片段」纠错对提炼成术语：去掉两边相同的前后缀得到改动处，
//! 改动处必须字数不变、全是汉字、读音对得上（同音或模糊音）；再在对的片段里把改动处扩到一个词库词
//! （「这个基限 → 这个基线」改动处只有「线」，扩成「基线」）；只改一个字又扩不到词的不要，
//! 硬凑两个字多半凑出「动地」这种不是词的东西。人名在畅译词库里是单独的纯术语条目，不靠这条路。

mod distillation;
mod skip;

pub use distillation::Distillation;
pub use skip::Skip;

use super::phonetic::Phonetic;
use super::text::is_han;

/// 纯术语最长几个字；再长多半是带上下文的句子片段。
const MAX_TERM_CHARS: usize = 6;

/// 同音互换只算润色的字组：大模型改「的 / 得 / 地」「他 / 她 / 它」是语法，不是识别错。
const GRAMMAR_SETS: [&str; 2] = ["的得地", "他她它"];

/// 扩词时最多扩到几个字。
const MAX_WORD_CHARS: usize = 4;

pub fn distill(phonetic: &Phonetic, wrong: Option<&str>, right: &str) -> Result<String, Skip> {
    let r: Vec<char> = right.chars().collect();
    let Some(wrong) = wrong else {
        if (2..=MAX_TERM_CHARS).contains(&r.len()) {
            return Ok(right.to_owned());
        }
        return Err(Skip::BadLength);
    };
    let w: Vec<char> = wrong.chars().collect();
    let prefix = w.iter().zip(&r).take_while(|(a, b)| a == b).count();
    let room = w.len().min(r.len()) - prefix;
    let suffix = w
        .iter()
        .rev()
        .zip(r.iter().rev())
        .take(room)
        .take_while(|(a, b)| a == b)
        .count();
    let core_w = &w[prefix..w.len() - suffix];
    let core_r = &r[prefix..r.len() - suffix];
    if core_r.is_empty() && core_w.is_empty() {
        return Err(Skip::Unchanged);
    }
    if !core_r.iter().chain(core_w).all(|c| is_han(*c)) {
        return Err(Skip::NotHan);
    }
    if core_w.len() != core_r.len() {
        return Err(Skip::LengthChanged);
    }
    let grammar = core_w.iter().zip(core_r).all(|(heard, meant)| {
        GRAMMAR_SETS
            .iter()
            .any(|set| set.contains(*heard) && set.contains(*meant))
    });
    if grammar {
        return Err(Skip::Grammar);
    }
    for (heard, meant) in core_w.iter().zip(core_r) {
        if phonetic
            .char_cost(*heard, phonetic.char_readings(*meant))
            .is_none()
        {
            return Err(Skip::NotHomophone);
        }
    }
    let end = prefix + core_r.len();
    // 包住改动处的最短词库词，同样长取词频高的
    let mut best: Option<(usize, u32, usize, usize)> = None;
    for start in prefix.saturating_sub(MAX_WORD_CHARS - 1)..=prefix {
        for stop in end..=(start + MAX_WORD_CHARS).min(r.len()) {
            let span = &r[start..stop];
            if span.len() < 2 || !span.iter().all(|c| is_han(*c)) {
                continue;
            }
            let text: String = span.iter().collect();
            if let Some(frequency) = phonetic.word_frequency(&text) {
                let better = best.is_none_or(|(len, freq, _, _)| {
                    span.len() < len || (span.len() == len && frequency > freq)
                });
                if better {
                    best = Some((span.len(), frequency, start, stop));
                }
            }
        }
    }
    let (start, stop) = match best {
        Some((_, _, start, stop)) => (start, stop),
        None if core_r.len() >= 2 => (prefix, end),
        None => return Err(Skip::NoWord),
    };
    Ok(r[start..stop].iter().collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use qingjian_dictionary::Dictionary;

    const SAMPLE: &str = "这\tzhe\t900\n个\tge\t900\n基\tji\t900\n线\txian\t900\n限\txian\t800\n\
        基线\tji xian\t300\n算\tsuan\t900\n薪\txin\t500\n新\txin\t900\n的\tde\t900\n地\tdi\t900\n地\tde\t500\n\
        自\tzi\t900\n动\tdong\t900\n他\tta\t900\n们\tmen\t900\n是\tshi\t900\n";

    fn run(wrong: &str, right: &str) -> Result<String, Skip> {
        let dictionary = Dictionary::parse(SAMPLE).unwrap();
        let phonetic = Phonetic::new([&dictionary]);
        distill(&phonetic, Some(wrong), right)
    }

    #[test]
    fn widens_a_single_char_change_to_the_dictionary_word() {
        assert_eq!(run("这个基限", "这个基线").as_deref(), Ok("基线"));
    }

    #[test]
    fn drops_single_char_changes_no_word_covers() {
        assert_eq!(run("方说算新", "方说算薪"), Err(Skip::NoWord));
        assert_eq!(run("自动的", "自动地"), Err(Skip::Grammar));
    }

    #[test]
    fn rejects_changes_that_are_not_homophones() {
        assert_eq!(run("他们", "他们是"), Err(Skip::LengthChanged));
        assert_eq!(run("格式的exce", "格式的Exce"), Err(Skip::NotHan));
    }
}
