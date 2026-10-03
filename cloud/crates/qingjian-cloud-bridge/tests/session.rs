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
    let mut session = Session::open(&data, Some(&user), None).unwrap();
    for c in "nihaoshijie".chars() {
        session.push(c);
    }
    assert!(session.composing());
    assert!(!session.preedit().is_empty());
    assert_eq!(session.entries()[0].text(), "你好世界");
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
    let mut session = Session::open(&data, None, None).unwrap();
    for c in "zhongw".chars() {
        session.push(c);
    }
    session.backspace();
    session.push('w');
    assert_eq!(session.take_raw(), "zhongw");
    assert!(!session.composing());
}

/// 连了青简 Cloud 才记输入日志（给服务器上的纠错闭环）；服务器连不上不影响打字与记日志。
#[test]
fn logs_only_when_connected() {
    let Some(data) = data_dir() else {
        return;
    };
    let type_and_commit = |session: &mut Session| {
        for c in "nihao".chars() {
            session.push(c);
        }
        session.commit(0);
        session.note_passthrough('\n');
        session.flush();
    };

    let offline = std::env::temp_dir().join(format!("qj-bridge-offline-{}", std::process::id()));
    std::fs::create_dir_all(&offline).unwrap();
    let mut session = Session::open(&data, Some(&offline), None).unwrap();
    type_and_commit(&mut session);
    assert!(!offline.join("input-log.jsonl").exists());

    let online = std::env::temp_dir().join(format!("qj-bridge-online-{}", std::process::id()));
    std::fs::create_dir_all(&online).unwrap();
    let cloud = toml::from_str::<qingjian_cloud_bridge::CloudConfig>(
        "server = \"http://127.0.0.1:9\"\ntoken = \"t\"\n",
    )
    .unwrap();
    let mut session = Session::open(&data, Some(&online), Some(cloud)).unwrap();
    type_and_commit(&mut session);
    let log = std::fs::read_to_string(online.join("input-log.jsonl")).unwrap();
    assert!(log.contains("你好"), "{log}");

    std::fs::remove_dir_all(&offline).ok();
    std::fs::remove_dir_all(&online).ok();
}
