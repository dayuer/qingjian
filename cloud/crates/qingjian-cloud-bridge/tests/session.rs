//! 用真实产品数据走一遍会话：`QINGJIAN_DATA` 指向含 `dict.qj` 的目录，没给或缺文件就跳过。

use std::path::PathBuf;

use qingjian_cloud_bridge::Session;

fn data_dir() -> Option<PathBuf> {
    let dir = PathBuf::from(std::env::var_os("QINGJIAN_DATA")?);
    dir.join("dict.qj").is_file().then_some(dir)
}

#[test]
fn types_and_commits_a_sentence() {
    let Some(data) = data_dir() else {
        eprintln!("没有 QINGJIAN_DATA，跳过");
        return;
    };
    let user = std::env::temp_dir().join(format!("qj-bridge-{}", std::process::id()));
    std::fs::create_dir_all(&user).unwrap();
    let mut session = Session::open(&data, Some(&user)).unwrap();
    for c in "nihaoshijie".chars() {
        session.push(c);
    }
    assert!(session.composing());
    assert!(!session.preedit().is_empty());
    assert_eq!(session.candidates()[0].text, "你好世界");
    assert_eq!(session.commit(0).as_deref(), Some("你好世界"));
    assert!(!session.composing());
    assert_eq!(session.punctuate(','), "，");
    session.flush();
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn backspace_and_raw() {
    let Some(data) = data_dir() else {
        return;
    };
    let mut session = Session::open(&data, None).unwrap();
    for c in "zhongw".chars() {
        session.push(c);
    }
    session.backspace();
    session.push('w');
    assert_eq!(session.take_raw(), "zhongw");
    assert!(!session.composing());
}
