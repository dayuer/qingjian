//! 话题补词：大模型读用户最近写的东西，列出用户接下来很可能还会打、但通用词库多半没有的专名与术语。
//! 只加现在根本打不出来的（全拼敲出来前几个候选里都没有）；已经在候选里只是不排第一的，留给用户自己选一次就学会。

use std::collections::HashSet;
use std::path::Path;

use qingjian_cloud_client::{Snapshot, Table};
use serde_json::Value;

use crate::engine_cli::EngineCli;
use crate::error::TunerError;
use crate::llm::Llm;
use crate::pinyin::{fits_text, normalize};
use crate::proposal::{Origin, Proposal};
use crate::state::TunerState;

/// 素材少于这么多字就不做：猜不准。
const MIN_TEXT: usize = 300;

const SYSTEM: &str = "你帮中文拼音输入法预测用户的用词。根据用户最近写的文字，判断用户关注的领域和话题，\
列出用户接下来很可能还会打、但通用输入法词库多半没有的词：人名、地名、机构与产品名、专业术语、圈内说法。不要列常用词。";

pub fn topics(
    llm: &Llm,
    cli: &EngineCli,
    learning: &Path,
    recent: &str,
    snapshot: &Snapshot,
    state: &mut TunerState,
    max: usize,
) -> Result<Vec<Proposal>, TunerError> {
    if recent.chars().count() < MIN_TEXT || max == 0 {
        return Ok(Vec::new());
    }
    let user = format!(
        "用户最近写的文字：\n{recent}\n\n最多列 {max} 个，回答一个 JSON 数组，每项是 \
         {{\"text\": 词, \"pinyin\": 拼音（音节空格分隔，ü 写作 v，不带声调）, \"reason\": 为什么觉得用户会用到}}。"
    );
    let existing: HashSet<&str> = snapshot.set_entries(Table::Words).map(|(t, _)| t).collect();
    let mut picked = Vec::new();
    for item in llm.ask_array(SYSTEM, &user)?.into_iter().take(max) {
        let Some(text) = item.get("text").and_then(Value::as_str) else {
            continue;
        };
        if existing.contains(text) || state.seen.contains(text) {
            continue;
        }
        let Some(pinyin) = item
            .get("pinyin")
            .and_then(Value::as_str)
            .and_then(normalize)
        else {
            continue;
        };
        if !fits_text(&pinyin, text) || pinyin.len() < 2 {
            continue;
        }
        let why = item
            .get("reason")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned();
        picked.push((text.to_owned(), pinyin, why));
    }
    let keys: Vec<String> = picked.iter().map(|(_, p, _)| p.concat()).collect();
    let current = cli.candidates(&keys, learning)?;
    let mut proposals = Vec::new();
    for (text, pinyin, why) in picked {
        state.seen.insert(text.clone());
        let typable = current
            .get(&pinyin.concat())
            .is_some_and(|list| list.contains(&text));
        if !typable {
            proposals.push(Proposal::AddWord {
                text,
                pinyin,
                keys: None,
                origin: Origin::Topic,
                why,
            });
        }
    }
    Ok(proposals)
}
