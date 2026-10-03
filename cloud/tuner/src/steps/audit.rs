//! 词库体检：把用户词（自动造词、云端选中的）交给大模型，只挑出读音错的与明显是垃圾的。
//! 宁缺毋滥：大模型拿不准的不改；改的读音必须是合法音节、个数等于字数。

use std::collections::HashMap;

use qingjian_cloud_client::{Snapshot, Table};
use serde_json::Value;

use crate::error::TunerError;
use crate::llm::Llm;
use crate::pinyin::{fits_text, is_han, normalize};
use crate::proposal::Proposal;
use crate::state::TunerState;

/// 一次问多少个词。
const BATCH: usize = 60;

const SYSTEM: &str = "你是中文拼音输入法的词库审校。用户词库里的词来自自动造词和云端联想，可能有读音标错或者根本不成词的条目。\
只指出你很确定有问题的条目，拿不准的一律不列。";

pub fn audit(
    llm: &Llm,
    snapshot: &Snapshot,
    state: &mut TunerState,
    max: usize,
) -> Result<Vec<Proposal>, TunerError> {
    let words: Vec<(String, String)> = snapshot
        .set_entries(Table::Words)
        .filter(|(text, _)| is_han(text) && !state.audited.contains(*text))
        .take(max)
        .map(|(t, p)| (t.to_owned(), p.to_owned()))
        .collect();
    let mut proposals = Vec::new();
    for batch in words.chunks(BATCH) {
        let list: String = batch.iter().map(|(t, p)| format!("{t}\t{p}\n")).collect();
        let user = format!(
            "下面每行是「词<Tab>拼音」（音节空格分隔，ü 写作 v，不带声调）：\n{list}\n\
             只列出有问题的条目，回答一个 JSON 数组，每项是 \
             {{\"word\": 词, \"action\": \"fix\" 或 \"delete\", \"pinyin\": 正确拼音（fix 时给，格式同上）, \"reason\": 一句话原因}}。\
             fix：词是对的但读音标错了（多音字读错最常见）。delete：错别字、半截词、几个字凑在一起不成词。没有问题就回答 []。"
        );
        let current: HashMap<&str, &str> = batch
            .iter()
            .map(|(t, p)| (t.as_str(), p.as_str()))
            .collect();
        for item in llm.ask_array(SYSTEM, &user)? {
            let Some(word) = item.get("word").and_then(Value::as_str) else {
                continue;
            };
            let Some(old) = current.get(word) else {
                continue;
            };
            let why = item
                .get("reason")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned();
            match item.get("action").and_then(Value::as_str) {
                Some("fix") => {
                    let Some(new) = item
                        .get("pinyin")
                        .and_then(Value::as_str)
                        .and_then(normalize)
                    else {
                        continue;
                    };
                    if fits_text(&new, word) && new.join(" ") != *old {
                        proposals.push(Proposal::FixPinyin {
                            text: word.to_owned(),
                            old: (*old).to_owned(),
                            new,
                            why,
                        });
                    }
                }
                Some("delete") => proposals.push(Proposal::DeleteWord {
                    text: word.to_owned(),
                    why,
                }),
                _ => {}
            }
        }
        state.audited.extend(batch.iter().map(|(t, _)| t.clone()));
    }
    Ok(proposals)
}
