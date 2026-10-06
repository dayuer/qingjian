//! 中英混输的英文候选与纠错态组字（2026-10-06 iOS 真机报告：打 android 时 r 被纠错吃掉、
//! 也没有英文候选）。`QINGJIAN_DATA` 需含 `english.tsv`（`data/generated` 就有），惯例同 `tests/session.rs`。

mod support;

use self::support::data_dir;
use qingjian_cloud_bridge::Session;

/// 逐键敲 `andro` → `android`：组字里敲的字母一个不少（纠错不改写宿主里的字母），英文候选在列表里。
#[test]
fn typed_letters_stay_and_english_candidates_appear() {
    let Some(data) = data_dir() else {
        eprintln!("没有 QINGJIAN_DATA，跳过");
        return;
    };
    let mut session = Session::open(&data, None, None, None).unwrap();
    let mut typed = String::new();
    for c in "android".chars() {
        session.push(c);
        typed.push(c);
        // andr / andro / android 三个节点核对（真机报告的阶段）
        if [4, 5, 7].contains(&typed.len()) {
            let shown = session.preedit();
            assert_eq!(
                shown.chars().filter(|c| *c != '\'').collect::<String>(),
                typed,
                "组字必须是敲的原样字母（可带音节分隔符）"
            );
        }
    }
    // 纠错把 andro 读成 an dao 出中文候选，组字仍是敲的 andro；英文候选 android 同时在
    assert!(session.entries().iter().any(|e| e.text() == "android"));
    // 懒加载：纯拼音阶段不读表（你好照常），敲到 hello（ll 触发）才有英文候选
    session.clear();
    for c in "nihao".chars() {
        session.push(c);
    }
    assert!(session.entries().iter().any(|e| e.text() == "你好"));
    session.clear();
    for c in "hello".chars() {
        session.push(c);
    }
    assert!(session.entries().iter().any(|e| e.text() == "hello"));
    session.clear();
    for c in "woxiangxueandroid".chars() {
        session.push(c);
    }
    // 整句里的英文尾巴（EnglishTail）：我想学好 android
    assert!(
        session
            .entries()
            .iter()
            .any(|e| e.text().contains("android")),
        "句末英文词应并入整句候选：{:?}",
        session
            .entries()
            .iter()
            .map(|e| e.text())
            .take(5)
            .collect::<Vec<_>>()
    );
}
