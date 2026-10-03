//! 一次就学会：用户把一段拼音分几次选完的词。输入法自己要记够两次才自动造词，这里让大模型确认是个词就直接补上，
//! 并记一次「这串键选了它」，下次整段敲出来就是首选。

use std::collections::{BTreeMap, HashSet};
use std::path::Path;

use qingjian_cloud_client::{Snapshot, Table};
use serde_json::Value;

use crate::engine_cli::EngineCli;
use crate::error::TunerError;
use crate::llm::Llm;
use crate::logs::Composition;
use crate::pinyin::{fits_text, is_han, matches_keys, normalize};
use crate::proposal::{Origin, Proposal};
use crate::state::TunerState;

const BATCH: usize = 60;

const SYSTEM: &str = "你是中文拼音输入法的词库编辑。用户敲一段拼音时，输入法没有把整段作为一个候选给出，用户只好分几次选字拼出来。\
判断这些拼出来的文字值不值得作为一个整体加进用户词库：是常用词、专名、术语、固定搭配就值得；只是两个普通词碰巧连在一起、或者是句子片段就不值得。";

pub fn from_compositions(
    llm: &Llm,
    cli: &EngineCli,
    learning: &Path,
    compositions: &[Composition],
    snapshot: &Snapshot,
    state: &mut TunerState,
    max: usize,
) -> Result<Vec<Proposal>, TunerError> {
    let existing: HashSet<&str> = snapshot.set_entries(Table::Words).map(|(t, _)| t).collect();
    // 同一个 (键, 文字) 合在一起，次数多的在前
    let mut grouped: BTreeMap<(String, String), (usize, String)> = BTreeMap::new();
    for c in compositions {
        let len = c.text.chars().count();
        if !is_han(&c.text)
            || !(2..=8).contains(&len)
            || existing.contains(c.text.as_str())
            || state.seen.contains(&c.text)
        {
            continue;
        }
        let entry = grouped
            .entry((c.keys.clone(), c.text.clone()))
            .or_insert((0, c.context.clone()));
        entry.0 += 1;
    }
    let mut candidates: Vec<((String, String), (usize, String))> = grouped.into_iter().collect();
    candidates.sort_by_key(|c| std::cmp::Reverse(c.1.0));
    candidates.truncate(max);
    if candidates.is_empty() {
        return Ok(Vec::new());
    }

    // 学习数据里已经让它排第一的（比如输入法自己造过词了）不用再管
    let keys: Vec<String> = candidates.iter().map(|((k, _), _)| k.clone()).collect();
    let current = cli.candidates(&keys, learning)?;
    candidates.retain(|((k, t), _)| current.get(k).and_then(|list| list.first()) != Some(t));

    let mut proposals = Vec::new();
    for batch in candidates.chunks(BATCH) {
        let list: String = batch
            .iter()
            .map(|((k, t), (n, ctx))| format!("{k}\t{t}\t{n} 次\t前文：{ctx}\n"))
            .collect();
        let user = format!(
            "每行是「敲的键<Tab>拼出的文字<Tab>出现次数<Tab>前文」：\n{list}\n\
             回答一个 JSON 数组，只包含值得加的，每项是 {{\"text\": 文字, \"pinyin\": 拼音（音节空格分隔，ü 写作 v，不带声调）, \"reason\": 一句话原因}}。都不值得就回答 []。"
        );
        let wanted: BTreeMap<&str, &str> = batch
            .iter()
            .map(|((k, t), _)| (t.as_str(), k.as_str()))
            .collect();
        for item in llm.ask_array(SYSTEM, &user)? {
            let Some(text) = item.get("text").and_then(Value::as_str) else {
                continue;
            };
            let Some(keys) = wanted.get(text) else {
                continue;
            };
            let Some(pinyin) = item
                .get("pinyin")
                .and_then(Value::as_str)
                .and_then(normalize)
            else {
                continue;
            };
            if !fits_text(&pinyin, text) || !matches_keys(&pinyin, keys) {
                continue;
            }
            proposals.push(Proposal::AddWord {
                text: text.to_owned(),
                pinyin,
                keys: Some((*keys).to_owned()),
                origin: Origin::Composition,
                why: item
                    .get("reason")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_owned(),
            });
        }
        // 问过的都记下：被否掉的下次不再问
        state.seen.extend(batch.iter().map(|((_, t), _)| t.clone()));
    }
    Ok(proposals)
}
