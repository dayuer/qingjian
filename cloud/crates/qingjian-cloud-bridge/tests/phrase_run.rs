//! 连打短语：`shufu` 选 舒服、`zhe` 选 着、`ne` 选 呢，分三段打满三轮后，`shufuzhene` 整串打的首选是 舒服着呢。
//! 用随包数据跑（`QINGJIAN_DATA`）：整串的最优切分是 zhen'e，冷启动首选 舒服真饿，着 + 呢 在整句词图里出不来。

mod support;

use qingjian_cloud_bridge::Session;

use self::support::data_dir;

fn first(session: &mut Session, keys: &str) -> String {
    session.clear();
    for c in keys.chars() {
        session.push(c);
    }
    let first = session
        .entries()
        .first()
        .map(|e| e.text().to_owned())
        .unwrap_or_default();
    session.clear();
    first
}

fn pick(session: &mut Session, keys: &str, text: &str) {
    for c in keys.chars() {
        session.push(c);
    }
    let index = session
        .entries()
        .iter()
        .position(|e| e.text() == text)
        .unwrap_or_else(|| panic!("{keys} 的候选里没有 {text}"));
    session.commit(index);
}

#[test]
fn shufuzhene_typed_in_three_pieces_three_times_comes_first() {
    let Some(data) = data_dir() else { return };
    let user = std::env::temp_dir().join(format!("qj-phrase-run-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&user);
    std::fs::create_dir_all(&user).unwrap();
    let mut session = Session::open(&data, Some(&user), None, None).unwrap();
    assert_ne!(first(&mut session, "shufuzhene"), "舒服着呢");
    for round in 1..=3 {
        pick(&mut session, "shufu", "舒服");
        pick(&mut session, "zhe", "着");
        pick(&mut session, "ne", "呢");
        session.note_passthrough('\n');
        let top = first(&mut session, "shufuzhene");
        if round < 3 {
            assert_ne!(top, "舒服着呢", "第 {round} 轮还不该造词");
        } else {
            assert_eq!(top, "舒服着呢");
        }
    }
    let _ = std::fs::remove_dir_all(&user);
}

/// 键盘换了输入框或收起（壳调 `reset_context`）：之前打的 舒服 不和之后的 着 呢 连成短语。
#[test]
fn changing_the_host_between_pieces_breaks_the_phrase() {
    let Some(data) = data_dir() else { return };
    let user = std::env::temp_dir().join(format!("qj-phrase-run-host-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&user);
    std::fs::create_dir_all(&user).unwrap();
    let mut session = Session::open(&data, Some(&user), None, None).unwrap();
    for _ in 0..4 {
        pick(&mut session, "shufu", "舒服");
        session.reset_context();
        pick(&mut session, "zhe", "着");
        pick(&mut session, "ne", "呢");
        session.note_passthrough('\n');
    }
    assert_ne!(first(&mut session, "shufuzhene"), "舒服着呢");
    let _ = std::fs::remove_dir_all(&user);
}
