//! 只有模糊音才读得通的拼写（`tin` → ting、`suang` → shuang）：切分、整段纠错与双拼解码的回归。

use qingjian_core::parser::{self, SyllableTrie};
use qingjian_core::{Engine, FuzzyRules, ShuangpinScheme};
use qingjian_dictionary::Dictionary;

const DICT: &str = "听\tting\t9000\n我\two\t9000\n说\tshuo\t9000\n请\tqing\t8000\n亲\tqin\t8000\n\
替\tti\t5000\n你\tni\t9000\n所\tsuo\t500\n双\tshuang\t9000\n人\tren\t8000\n双人\tshuang ren\t12000\n\
定\tding\t9000\n案\tan\t5000\n定案\tding an\t8000\n岗\tgang\t3000\n定岗\tding gang\t2000\n";

fn rules(names: &[&str]) -> FuzzyRules {
    let mut rules = FuzzyRules::default();
    for name in names {
        assert!(rules.enable(name));
    }
    rules
}

fn trie(rules: FuzzyRules) -> SyllableTrie {
    let sources = rules.sources();
    let refs: Vec<&str> = sources.iter().map(String::as_str).collect();
    SyllableTrie::build(&refs)
}

fn first(engine: &mut Engine, keys: &str) -> String {
    engine.set_input(keys);
    engine.query().unwrap().candidates.items[0].text.clone()
}

#[test]
fn sources_are_the_illegal_spellings_one_rule_away() {
    assert_eq!(rules(&["in-ing"]).sources(), ["din", "tin"]);
    assert_eq!(
        rules(&["s-sh"]).sources(),
        ["sei", "shong", "sua", "suai", "suang"]
    );
    assert!(FuzzyRules::default().sources().is_empty());
    assert_eq!(FuzzyRules::ALL.sources().len(), 60);
}

#[test]
fn segmentation_reads_fuzzy_sources_as_complete_but_after_legal_ones() {
    let in_ing = trie(rules(&["in-ing", "s-sh"]));
    let best = &parser::segment_with("tinwosuo", Some(&in_ing)).unwrap()[0];
    assert_eq!(best.to_string(), "tin wo suo");
    assert!(best.syllables[0].fuzzy && !best.syllables[1].fuzzy);
    // 合法切分排第一时与不开模糊音逐条一致
    for input in ["dingan", "dinan", "nihao"] {
        assert_eq!(
            parser::segment_with(input, Some(&in_ing)).unwrap(),
            parser::segment(input).unwrap(),
            "{input}"
        );
    }
    // 末尾没敲完的前缀排在模糊音读法前面：shoufe 是 shou fe…，不是 shou fe→he
    let all = trie(FuzzyRules::ALL);
    assert_eq!(
        parser::segment_with("shoufe", Some(&all)).unwrap()[0].to_string(),
        "shou fe…"
    );
}

#[test]
fn tinwosuo_is_read_by_fuzzy_rules_not_corrected() {
    let mut engine = Engine::new(Dictionary::parse(DICT).unwrap());
    engine.set_fuzzy(rules(&["in-ing", "s-sh"]));
    assert_eq!(first(&mut engine, "tinwosuo"), "听我说");
    assert!(engine.query().unwrap().correction.is_none());
    // 关掉模糊音：行为同改前（不出 听我说）
    engine.set_fuzzy(FuzzyRules::default());
    assert_ne!(first(&mut engine, "tinwosuo"), "听我说");
    // 合法拼写不受影响
    engine.set_fuzzy(rules(&["in-ing"]));
    assert_eq!(first(&mut engine, "dingan"), "定案");
}

#[test]
fn suang_needs_s_sh() {
    let mut engine = Engine::new(Dictionary::parse(DICT).unwrap());
    engine.set_fuzzy(rules(&["s-sh"]));
    assert_eq!(first(&mut engine, "suang"), "双");
    engine.set_fuzzy(rules(&["n-l"]));
    engine.set_input("suang");
    assert!(
        engine
            .query()
            .map_or(true, |q| q.candidates.items.iter().all(|c| c.text != "双"))
    );
}

#[test]
fn shuangpin_pairs_decode_through_the_same_sources() {
    let mut engine = Engine::new(Dictionary::parse(DICT).unwrap());
    engine.set_shuangpin(Some(ShuangpinScheme::Ziranma));
    engine.set_fuzzy(rules(&["s-sh"]));
    assert_eq!(first(&mut engine, "sd"), "双");
    assert_eq!(first(&mut engine, "sdrf"), "双人");
    engine.set_fuzzy(FuzzyRules::default());
    engine.set_input("sd");
    assert!(
        engine
            .query()
            .map_or(true, |q| q.candidates.items.iter().all(|c| c.text != "双"))
    );
    // 小鹤 tb = t + in：开 in/ing 后同一条路
    engine.set_shuangpin(Some(ShuangpinScheme::Xiaohe));
    engine.set_fuzzy(rules(&["in-ing"]));
    assert_eq!(first(&mut engine, "tb"), "听");
}

#[test]
fn committing_a_fuzzy_word_consumes_its_typed_keys() {
    let mut engine = Engine::new(Dictionary::parse(DICT).unwrap());
    engine.set_fuzzy(rules(&["in-ing", "s-sh"]));
    engine.set_input("tinwosuo");
    let query = engine.query().unwrap();
    let ting = query
        .candidates
        .items
        .iter()
        .find(|c| c.text == "听")
        .unwrap()
        .clone();
    assert_eq!(engine.commit(&ting), "听");
    assert_eq!(engine.composition().text(), "wosuo");
}
