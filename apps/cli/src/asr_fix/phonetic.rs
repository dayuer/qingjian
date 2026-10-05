//! 汉字读音表：单字的全部读音（多音字都留着，识别结果里的字不知道读哪个）与多字词的读音。

use std::collections::HashMap;

use qingjian_core::FuzzyRules;
use qingjian_dictionary::{Dictionary, canonical_syllable};

pub struct Phonetic {
    /// 单字 → 全部读音。
    chars: HashMap<char, Vec<String>>,

    /// 多字词 → 词频最高的那个读音（音节列表）与它的词频。
    words: HashMap<String, (Vec<String>, u32)>,

    /// 近音判定：模糊音全开。
    rules: FuzzyRules,
}

impl Phonetic {
    pub fn new<'a>(dictionaries: impl IntoIterator<Item = &'a Dictionary>) -> Self {
        let mut chars: HashMap<char, Vec<String>> = HashMap::new();
        let mut words: HashMap<String, (Vec<String>, u32)> = HashMap::new();
        for dictionary in dictionaries {
            for entry in dictionary.entries() {
                let mut text = entry.text.chars();
                if let (Some(c), None) = (text.next(), text.next()) {
                    let readings = chars.entry(c).or_default();
                    if !readings.iter().any(|r| r == entry.pinyin) {
                        readings.push(entry.pinyin.to_owned());
                    }
                    continue;
                }
                match words.get_mut(entry.text) {
                    Some(best) if best.1 >= entry.frequency => {}
                    _ => {
                        let syllables = entry.syllables().map(str::to_owned).collect();
                        words.insert(entry.text.to_owned(), (syllables, entry.frequency));
                    }
                }
            }
        }
        Self {
            chars,
            words,
            rules: FuzzyRules::ALL,
        }
    }

    pub fn char_readings(&self, c: char) -> &[String] {
        self.chars.get(&c).map_or(&[], Vec::as_slice)
    }

    /// 术语每个字可接受的读音：词库里有这个词就用词的读音（多音字读对），否则逐字取全部读音；
    /// 有字查不到读音返回 `None`。
    pub fn term_readings(&self, term: &str) -> Option<Vec<Vec<String>>> {
        if let Some((syllables, _)) = self.words.get(term)
            && syllables.len() == term.chars().count()
        {
            return Some(syllables.iter().map(|s| vec![s.clone()]).collect());
        }
        term.chars()
            .map(|c| {
                let readings = self.char_readings(c);
                (!readings.is_empty()).then(|| readings.to_vec())
            })
            .collect()
    }

    /// 听到的字 `heard` 当成读 `expected` 之一的字要付的代价：同音 0、近音（模糊音）1、对不上 `None`。
    pub fn char_cost(&self, heard: char, expected: &[String]) -> Option<u32> {
        let mut best = None;
        for e in expected {
            for h in self.char_readings(heard) {
                let cost = if canonical_syllable(e) == canonical_syllable(h) {
                    0
                } else if self.rules.is_variant(e, h) {
                    1
                } else {
                    continue;
                };
                best = Some(best.map_or(cost, |b: u32| b.min(cost)));
            }
        }
        best
    }

    /// 多字词在词库里的词频；不是词库词返回 `None`。
    pub fn word_frequency(&self, text: &str) -> Option<u32> {
        self.words.get(text).map(|(_, frequency)| *frequency)
    }
}
