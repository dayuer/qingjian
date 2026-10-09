//! 选一次照样翻正：同一段拼音选了不是首选的整句 / 整词后，下一次同样的输入首选就是它。
//! `QINGJIAN_DATA` 指向键盘的 Data 目录（含 dict.qj），没给就跳过，惯例同 `tests/session.rs`。

mod support;

use qingjian_cloud_bridge::Session;

use self::support::data_dir;

/// 打一串拼音，返回前 8 个候选。
fn candidates(session: &mut Session, keys: &str) -> Vec<String> {
    session.clear();
    for c in keys.chars() {
        session.push(c);
    }
    session
        .entries()
        .iter()
        .take(8)
        .map(|e| e.text().to_owned())
        .collect()
}

/// 每条拼音：选第一个「把整段都吃掉」的非首选候选（字数等于音节数），再打一次，它必须成为首选。
#[test]
fn picking_a_lower_candidate_once_makes_it_first() {
    let Some(data) = data_dir() else {
        return;
    };
    let user = std::env::temp_dir().join(format!("qj-learning-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&user);
    std::fs::create_dir_all(&user).unwrap();
    let mut session = Session::open(&data, Some(&user), None, None).unwrap();
    // 三音节：姓 + 双字名的形态；两音节：普通词
    let cases = [
        ("liyuqing", 3),
        ("wangxiaoming", 3),
        ("shiyan", 2),
        ("zhangwei", 2),
    ];
    let mut failures = Vec::new();
    for (keys, syllables) in cases {
        let before = candidates(&mut session, keys);
        let Some(index) = before
            .iter()
            .enumerate()
            .skip(1)
            .find(|(_, text)| text.chars().count() == syllables)
            .map(|(i, _)| i)
        else {
            eprintln!("{keys}: 前 8 个里没有整段的非首选候选，跳过 {before:?}");
            continue;
        };
        let picked = before[index].clone();
        let committed = session.commit(index).unwrap();
        assert_eq!(committed, picked);
        if session.composing() {
            // 字数对上但只吃掉了一部分拼音（简拼、多音节读法），不是这条测试要的「整段选一次」
            eprintln!("{keys}: 「{picked}」只吃掉一部分拼音，跳过");
            session.clear();
            continue;
        }
        let after = candidates(&mut session, keys);
        eprintln!(
            "{keys}: 选第 {} 个「{picked}」；之前 {before:?}；之后 {after:?}",
            index + 1
        );
        if after.first() != Some(&picked) {
            failures.push(format!(
                "{keys}: 选了「{picked}」，之后首选是 {:?}",
                after.first()
            ));
        }
        session.clear();
    }
    let _ = std::fs::remove_dir_all(&user);
    assert!(failures.is_empty(), "{failures:#?}");
}

/// 照用户的路数：姓 + 双字名先分两次选完（两字词 + 单字），再打同一段拼音；拼出来的那个名字出现在候选里时选它，
/// 第三次打必须首选它。不写具体人名：从候选里现挑一个两字词和一个单字，拼出来的就是这次的「名字」。
#[test]
fn a_name_picked_in_pieces_then_whole_becomes_first() {
    let Some(data) = data_dir() else {
        return;
    };
    let user = std::env::temp_dir().join(format!("qj-learning-pieces-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&user);
    std::fs::create_dir_all(&user).unwrap();
    let mut session = Session::open(&data, Some(&user), None, None).unwrap();
    let keys = "liyuqing";
    let first = candidates(&mut session, keys);
    // 先选一个两字词（吃掉前两个音节），剩下的再选一个单字
    let head = first
        .iter()
        .position(|t| t.chars().count() == 2)
        .expect("前 8 个里有两字词");
    let head_text = session.commit(head).unwrap();
    assert!(session.composing(), "两字词只吃掉一部分拼音");
    let rest: Vec<String> = session
        .entries()
        .iter()
        .map(|e| e.text().to_owned())
        .collect();
    // 挑一个不是剩余首选的单字，模拟「名字用字词图不排第一」
    let tail = rest
        .iter()
        .enumerate()
        .skip(1)
        .find(|(_, t)| t.chars().count() == 1)
        .map(|(i, _)| i)
        .expect("剩下的拼音有单字候选");
    let tail_text = session.commit(tail).unwrap();
    assert!(!session.composing());
    let name = format!("{head_text}{tail_text}");

    let second = candidates(&mut session, keys);
    eprintln!("分次选成「{name}」之后：{second:?}");
    let Some(index) = second.iter().position(|t| t == &name) else {
        panic!("分次选过一次后，「{name}」没进前 8：{second:?}");
    };
    if index > 0 {
        assert_eq!(session.commit(index).as_deref(), Some(name.as_str()));
        let third = candidates(&mut session, keys);
        eprintln!("整个选一次之后：{third:?}");
        assert_eq!(third.first(), Some(&name), "整个选过一次，首选还不是它");
    }
    let _ = std::fs::remove_dir_all(&user);
}
