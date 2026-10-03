//! 一条修正建议，以及把一批建议换成学习数据的变化推给服务器（各设备经学习数据同步拿到）。

use qingjian_cloud_proto::{CountDelta, LearningPush, SetDelete, SetPut};

/// 建议从哪一步来；回放门槛没过时先去掉把握最小的「话题补词」再试。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    Audit,
    Composition,
    Topic,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Proposal {
    /// 加用户词；`keys` 是用户实际敲过的键，有的话顺带记一次「选过它」：按输入串的选择、词频与句首 n-gram 各加一，
    /// 和用户自己选过一次后输入法记的一样。只记选择不够：整句候选按 n-gram 打分，不受词级选择影响，会照样排在它前面。
    AddWord {
        text: String,
        pinyin: Vec<String>,
        keys: Option<String>,
        origin: Origin,
        why: String,
    },

    /// 用户词读音错了，改正。
    FixPinyin {
        text: String,
        old: String,
        new: Vec<String>,
        why: String,
    },

    /// 用户词是垃圾（错字、半截、读音对不上又改不对），删掉。
    DeleteWord { text: String, why: String },
}

impl Proposal {
    pub fn origin(&self) -> Origin {
        match self {
            Self::AddWord { origin, .. } => *origin,
            Self::FixPinyin { .. } | Self::DeleteWord { .. } => Origin::Audit,
        }
    }

    /// 报告里的一行。
    pub fn describe(&self) -> String {
        match self {
            Self::AddWord {
                text,
                pinyin,
                keys,
                why,
                ..
            } => match keys {
                Some(keys) => format!("加词 {text}（{}，键 {keys}）：{why}", pinyin.join(" ")),
                None => format!("加词 {text}（{}）：{why}", pinyin.join(" ")),
            },
            Self::FixPinyin {
                text,
                old,
                new,
                why,
            } => format!("改读音 {text}：{old} → {}：{why}", new.join(" ")),
            Self::DeleteWord { text, why } => format!("删词 {text}：{why}"),
        }
    }
}

pub fn to_push(proposals: &[Proposal]) -> LearningPush {
    let mut push = LearningPush::default();
    for proposal in proposals {
        match proposal {
            Proposal::AddWord {
                text, pinyin, keys, ..
            } => {
                push.puts.push(SetPut {
                    table: "words".to_owned(),
                    key: text.clone(),
                    value: pinyin.join(" "),
                });
                if let Some(keys) = keys {
                    for (table, key) in [
                        ("choices", format!("{keys}\t{text}")),
                        ("user", text.clone()),
                        ("ngram", format!("<s>\t{text}")),
                    ] {
                        push.counts.push(CountDelta {
                            table: table.to_owned(),
                            key,
                            delta: 1,
                            value: None,
                        });
                    }
                }
            }
            Proposal::FixPinyin { text, new, .. } => push.puts.push(SetPut {
                table: "words".to_owned(),
                key: text.clone(),
                value: new.join(" "),
            }),
            Proposal::DeleteWord { text, .. } => push.deletes.push(SetDelete {
                table: "words".to_owned(),
                key: text.clone(),
            }),
        }
    }
    push
}
