use crate::engine::{MarkedKind, MarkedSegment};
use crate::parser::Segmentation;

use super::edit::Edit;

/// 一次拼写纠正：用户敲的串、纠正后的串、那一处编辑（偶尔再加一处换键），以及纠正后串的完整音节切分。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Correction {
    /// 用户敲的（作用域里的拼音，不含 `'`）。
    pub original: String,

    /// 纠正后的拼音。
    pub corrected: String,

    /// 从原串到纠正后串的那一处编辑。
    pub edit: Edit,

    /// 第二处编辑：只会是换成相邻键（手机上一个词里敲错两次的多半是两次误触，`dinbushabg` → `dianbushang`），
    /// 只在一处编辑凑不出完整拼音时才试。换键不改长度，所以 [`Edit::to_original`] 仍只看第一处。
    pub second: Option<Edit>,

    /// 纠正后串的切分：每个音节都完整，或（相邻换位时）只有末尾一个还没敲完。
    pub segmentation: Segmentation,
}

impl Correction {
    /// 这处编辑落在纠正后哪个音节里：返回 (敲的那段字母, 纠正后的音节)，给个人敲错表记（敲的那段多半不是合法音节）；
    /// 多敲的字母算在它前面那个音节上（与 [`Edit::to_original`] 一致）。编辑处不在任何音节里时返回 `None`。
    /// `consumed` 是上屏消耗掉的纠正后字母数：没吃到编辑处的上屏不算接受了纠正。
    pub fn typo_pair(&self, consumed: usize) -> Option<(String, String)> {
        // 两处编辑的纠正说不清用户要的是哪个音节对，不记
        if self.second.is_some() {
            return None;
        }
        let at = match self.edit {
            Edit::Substitute { index, .. }
            | Edit::Insert { index }
            | Edit::Transpose { index, .. } => index,
            Edit::Delete { index, .. } => index.saturating_sub(1),
        };
        if at >= consumed {
            return None;
        }
        let mut start = 0;
        for syllable in &self.segmentation.syllables {
            let end = start + syllable.text.len();
            if at < end {
                // 编辑落在还没敲完的末尾音节里：还不知道用户要的是哪个音节，不记
                if !syllable.complete {
                    return None;
                }
                let typed_start = self.edit.to_original(start);
                let typed_end = self.edit.to_original(end);
                let typed = self.original.get(typed_start..typed_end)?;
                return (!typed.is_empty() && typed != syllable.text)
                    .then(|| (typed.to_owned(), syllable.text.clone()));
            }
            start = end;
        }
        None
    }

    /// preedit 的分段：纠正后的切分拼音（`'` 连接），被改掉的原字母以 [`MarkedKind::Corrected`] 插在它原来的位置。
    /// `nihooma` → `ni'h` + ~~o~~ + `ao'ma`；有第二处编辑时两处都划。
    pub fn marked_segments(&self) -> Vec<MarkedSegment> {
        let display = self.segmentation.joined("'");
        let mut struck: Vec<(usize, String)> = [Some(&self.edit), self.second.as_ref()]
            .into_iter()
            .flatten()
            .filter_map(Edit::struck)
            .collect();
        struck.sort_by_key(|(at, _)| *at);
        if struck.is_empty() {
            return vec![MarkedSegment::new(display, MarkedKind::Typed)];
        }
        // 纠正后串的字母下标 → 显示串（含 `'`）的下标
        let split_at = |at: usize| {
            let mut letters = 0;
            for (i, c) in display.char_indices() {
                if c == '\'' {
                    continue;
                }
                if letters == at {
                    return i;
                }
                letters += 1;
            }
            display.len()
        };
        let mut segments = Vec::with_capacity(5);
        let mut from = 0;
        for (at, text) in struck {
            let split = split_at(at);
            if split > from {
                segments.push(MarkedSegment::new(&display[from..split], MarkedKind::Typed));
            }
            segments.push(MarkedSegment::new(text, MarkedKind::Corrected));
            from = split;
        }
        if from < display.len() {
            segments.push(MarkedSegment::new(&display[from..], MarkedKind::Typed));
        }
        segments
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Syllable;

    fn segmentation(syllables: &[&str]) -> Segmentation {
        Segmentation {
            syllables: syllables.iter().map(|s| Syllable::complete(s)).collect(),
        }
    }

    fn texts(segments: &[MarkedSegment]) -> Vec<(String, MarkedKind)> {
        segments.iter().map(|s| (s.text.clone(), s.kind)).collect()
    }

    #[test]
    fn strikes_the_replaced_letter_in_place() {
        let correction = Correction {
            original: "nihooma".into(),
            corrected: "nihaoma".into(),
            edit: Edit::Substitute {
                index: 3,
                from: 'o',
            },
            second: None,
            segmentation: segmentation(&["ni", "hao", "ma"]),
        };
        assert_eq!(
            texts(&correction.marked_segments()),
            [
                ("ni'h".to_owned(), MarkedKind::Typed),
                ("o".to_owned(), MarkedKind::Corrected),
                ("ao'ma".to_owned(), MarkedKind::Typed),
            ]
        );
    }

    #[test]
    fn typo_pair_is_the_syllable_around_the_edit() {
        let correction = Correction {
            original: "nihooma".into(),
            corrected: "nihaoma".into(),
            edit: Edit::Substitute {
                index: 3,
                from: 'o',
            },
            second: None,
            segmentation: segmentation(&["ni", "hao", "ma"]),
        };
        assert_eq!(
            correction.typo_pair(7),
            Some(("hoo".to_owned(), "hao".to_owned()))
        );
        // 只吃到 ni 就上屏：没接受纠正
        assert_eq!(correction.typo_pair(2), None);
        let correction = Correction {
            original: "meiganxi".into(),
            corrected: "meiguanxi".into(),
            edit: Edit::Insert { index: 4 },
            second: None,
            segmentation: segmentation(&["mei", "guan", "xi"]),
        };
        assert_eq!(
            correction.typo_pair(9),
            Some(("gan".to_owned(), "guan".to_owned()))
        );
        let correction = Correction {
            original: "zhegge".into(),
            corrected: "zhege".into(),
            edit: Edit::Delete {
                index: 3,
                removed: 'g',
            },
            second: None,
            segmentation: segmentation(&["zhe", "ge"]),
        };
        // 多敲的 g 算在前一个音节上
        assert_eq!(
            correction.typo_pair(5),
            Some(("zheg".to_owned(), "zhe".to_owned()))
        );
        // 换位落在没敲完的末尾音节里：不记
        let correction = Correction {
            original: "mingita".into(),
            corrected: "mingtia".into(),
            edit: Edit::Transpose {
                index: 4,
                first: 'i',
                second: 't',
            },
            second: None,
            segmentation: Segmentation {
                syllables: vec![Syllable::complete("ming"), Syllable::partial("tia")],
            },
        };
        assert_eq!(correction.typo_pair(7), None);
    }

    #[test]
    fn two_edits_strike_both_letters_and_learn_nothing() {
        let correction = Correction {
            original: "dinbushabg".into(),
            corrected: "dianbushang".into(),
            edit: Edit::Insert { index: 2 },
            second: Some(Edit::Substitute {
                index: 9,
                from: 'b',
            }),
            segmentation: segmentation(&["dian", "bu", "shang"]),
        };
        assert_eq!(
            texts(&correction.marked_segments()),
            [
                ("dian'bu'sha".to_owned(), MarkedKind::Typed),
                ("b".to_owned(), MarkedKind::Corrected),
                ("ng".to_owned(), MarkedKind::Typed),
            ]
        );
        assert_eq!(correction.typo_pair(11), None);
    }

    #[test]
    fn extra_letter_at_the_end_and_missing_letter() {
        let correction = Correction {
            original: "nihaox".into(),
            corrected: "nihao".into(),
            edit: Edit::Delete {
                index: 5,
                removed: 'x',
            },
            second: None,
            segmentation: segmentation(&["ni", "hao"]),
        };
        assert_eq!(
            texts(&correction.marked_segments()),
            [
                ("ni'hao".to_owned(), MarkedKind::Typed),
                ("x".to_owned(), MarkedKind::Corrected),
            ]
        );
        let correction = Correction {
            original: "nhao".into(),
            corrected: "nihao".into(),
            edit: Edit::Insert { index: 1 },
            second: None,
            segmentation: segmentation(&["ni", "hao"]),
        };
        assert_eq!(
            texts(&correction.marked_segments()),
            [("ni'hao".to_owned(), MarkedKind::Typed)]
        );
    }
}
