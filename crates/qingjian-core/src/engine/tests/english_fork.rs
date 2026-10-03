//! 分叉补丁的英文规则（见 `cloud/docs/fork-patch.md`）：两字母带大写的英文让中文、敲错的拼音回车上屏不学成英文词。

use super::*;

/// 记英文学习的 Learner：选过的英文词有次数，学到的英文词进个人词表。
#[derive(Default)]
struct EnglishLearner {
    picked: HashMap<String, u32>,
    learned: Vec<String>,
    list: Option<WordList>,
}

impl Learner for EnglishLearner {
    fn record(&mut self, candidate: &Candidate) {
        *self.picked.entry(candidate.text.clone()).or_default() += 1;
    }
    fn weight(&self, text: &str) -> u32 {
        self.picked.get(text).copied().unwrap_or(0)
    }
    fn learn_english(&mut self, word: &str) {
        self.learned.push(word.to_owned());
        let tsv: String = self
            .learned
            .iter()
            .map(|w| format!("{w}\t{}\t1\n", w.to_ascii_lowercase()))
            .collect();
        self.list = WordList::parse(&tsv).ok();
    }
    fn user_english(&self) -> Option<&WordList> {
        self.list.as_ref()
    }
}

#[test]
fn two_letter_mixed_case_english_yields_to_chinese_until_picked() {
    // `gd` 是 该地 / 广东 的简拼，Gd（钆）不该抢第一；与 MP 一样，选过一次之后才排第一
    let dictionary = Dictionary::parse("该地\tgai di\t5000\n广东\tguang dong\t4000\n").unwrap();
    let words = WordList::parse("Gd\tgd\t3210\nGPU\tgpu\t3220\n").unwrap();
    let mut engine = Engine::new(dictionary)
        .with_english(words)
        .with_learner(Box::new(EnglishLearner::default()));
    engine.set_input("gd");
    let all = texts_of(&engine);
    assert_eq!(all[0], "该地");
    assert!(all.iter().any(|t| t == "Gd"), "英文词仍在候选里");

    let gd = engine
        .query()
        .unwrap()
        .candidates
        .items
        .into_iter()
        .find(|c| c.text == "Gd")
        .unwrap();
    engine.commit(&gd);
    engine.set_input("gd");
    assert_eq!(texts_of(&engine)[0], "Gd");

    // 三个字母的大写缩写不受影响（GPU / SQL 是正经要打的）
    engine.set_input("gpu");
    assert_eq!(texts_of(&engine)[0], "GPU");
}

#[test]
fn raw_committed_pinyin_typos_are_not_learned_as_english() {
    let words = WordList::parse("hello\thello\t4720\n").unwrap();
    let mut engine = Engine::new(Dictionary::parse(SAMPLE).unwrap())
        .with_english(words)
        .with_learner(Box::new(EnglishLearner::default()));
    let english_candidate = |engine: &mut Engine, input: &str| {
        engine.set_input(input);
        engine.query().is_ok_and(|query| {
            query
                .candidates
                .items
                .iter()
                .any(|c| c.kind == CandidateKind::English && c.text == input)
        })
    };

    // 多敲了一个 i：去掉就是 wo lai ce shi，是敲错的中文，回车上屏后不学
    engine.set_input("woilaiceshi");
    assert_eq!(engine.take_raw(), "woilaiceshi");
    assert!(!english_candidate(&mut engine, "woilaiceshi"));

    // 切不成拼音、改一处也不行的照旧学（gist）
    engine.set_input("gist");
    assert_eq!(engine.take_raw(), "gist");
    assert!(english_candidate(&mut engine, "gist"));
}

#[test]
fn three_letter_english_completions_do_not_take_first_place() {
    // `wod` 是 我的 的简拼：Wodehouse 补全不该排第一；四个字母起的补全（`compa` → company）照旧
    let dictionary = Dictionary::parse("我的\two de\t9000\n凑\tcou\t100\n").unwrap();
    let words = WordList::parse(
        "Wodehouse\twodehouse\t2900\ncompany\tcompany\t5600\ncompare\tcompare\t4450\n",
    )
    .unwrap();
    let mut engine = Engine::new(dictionary).with_english(words);
    engine.set_input("wod");
    let all = texts_of(&engine);
    assert_eq!(all[0], "我的");
    assert!(all.iter().any(|t| t == "Wodehouse"), "补全仍在候选里");

    engine.set_input("compa");
    assert_eq!(texts_of(&engine)[0], "company");
}
