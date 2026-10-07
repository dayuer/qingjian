use super::*;
use crate::replay::line::Line;

fn entries(log: &str) -> Vec<(usize, InputLogEntry)> {
    log.lines()
        .enumerate()
        .map(|(i, raw)| (i + 1, serde_json::from_str::<Line>(raw).unwrap().entry))
        .collect()
}

fn commit(id: u64, scope: &str, text: &str) -> String {
    format!(
        r#"{{"event":"commit","id":{id},"scope":"{scope}","keys":"{scope}","pinyin":"","corrected":false,"text":"{text}","source":"word","index":0,"top":[],"scheme":"","english":false}}"#
    )
}

fn no_words(_: &str) -> bool {
    false
}

#[test]
fn retract_drops_the_latest_commit_with_that_id() {
    // id 每个会话从头计：撤销的是第二个会话里那条 id 1
    let log = [
        commit(1, "ku", "哭"),
        commit(1, "ku", "哭"),
        r#"{"event":"retract","of":1,"text":"哭","chosen":"库"}"#.to_owned(),
    ]
    .join("\n");
    let entries = entries(&log);
    let excluded = scan(entries.iter().map(|(n, e)| (*n, e)), no_words);
    assert_eq!(excluded.get(&2), Some(&Rule::Retracted));
    assert!(!excluded.contains_key(&1));
}

#[test]
fn retype_drops_only_the_erased_commit() {
    let log = [
        // 跨上屏：shene 上屏后删掉、重打 shenme，of 指被删掉的那次
        commit(5, "shene", "神阿"),
        r#"{"event":"retype","before":"shene","after":"shenme","of":5}"#.to_owned(),
        // 组句内：of 指这次上屏本身，作用域等于 after，不剔
        commit(6, "kankanrizhi", "看看日志"),
        r#"{"event":"retype","before":"kankanri","after":"kankanrizhi","of":6}"#.to_owned(),
    ]
    .join("\n");
    let entries = entries(&log);
    let excluded = scan(entries.iter().map(|(n, e)| (*n, e)), no_words);
    assert_eq!(excluded.get(&1), Some(&Rule::Retyped));
    assert!(!excluded.contains_key(&3));
}

#[test]
fn latin_tail_spares_words_made_of_dictionary_words() {
    let is_word = |s: &str| matches!(s, "C盘" | "修改" | "U盘" | "拷到");
    assert!(latin_tail("修改U", is_word));
    assert!(latin_tail("先IE", is_word));
    assert!(!latin_tail("C盘", is_word));
    assert!(!latin_tail("拷到U盘", is_word));
    // 字母段长过两个是正常的中英混输
    assert!(!latin_tail("用docker部署", is_word));
    assert!(!latin_tail("修改", is_word));
}
