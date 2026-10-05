//! 名字的拼音首字母（通讯录按字母分组与右侧索引用）：拿样例词库核几个字，
//! 以及不是汉字、词库不认识、名字为空时的行为。

use std::path::Path;

use qingjian_dictionary::Dictionary;

use crate::memory::initial_of;

fn sample() -> Dictionary {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../assets/sample/dict.tsv");
    Dictionary::from_path(&path).unwrap()
}

#[test]
fn takes_the_initial_of_the_first_char() {
    let dictionary = sample();
    assert_eq!(initial_of(&dictionary, "我"), Some('W'));
    assert_eq!(
        initial_of(&dictionary, "人在"),
        Some('R'),
        "只看名字的第一个字"
    );
    assert_eq!(initial_of(&dictionary, "  你"), Some('N'), "首尾空白先去掉");
}

#[test]
fn foreign_names_take_their_own_letter() {
    let dictionary = sample();
    assert_eq!(initial_of(&dictionary, "Alice"), Some('A'));
    assert_eq!(initial_of(&dictionary, "bob"), Some('B'));
}

#[test]
fn names_without_a_reading_have_no_initial() {
    let dictionary = sample();
    assert_eq!(initial_of(&dictionary, ""), None);
    assert_eq!(initial_of(&dictionary, "   "), None);
    assert_eq!(initial_of(&dictionary, "𠮷"), None, "词库不认得的字");
    assert_eq!(initial_of(&dictionary, "🐱"), None, "表情没有读音");
}
