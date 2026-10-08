//! 照键盘的会话查一段拼音的前几个候选：`query_probe <键盘 Data 目录> <拼音>…`（`'` 照常当音节分隔）。
//! 每条标出本地 / 云端与候选类型，复现用户截图时用。

use std::path::PathBuf;

use qingjian_cloud_bridge::{Entry, Session};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let data = PathBuf::from(args.next().ok_or("给键盘 Data 目录")?);
    let mut session = Session::open(&data, None, None, None)?;
    for keys in args {
        session.clear();
        for c in keys.chars() {
            session.push(c);
        }
        println!("## {keys}  组字 {}", session.preedit());
        for (i, entry) in session.entries().iter().take(8).enumerate() {
            let (origin, kind) = match entry {
                Entry::Local(c) => ("本地", format!("{:?}", c.kind)),
                Entry::Cloud(c) => ("云端", format!("{:?}", c.kind)),
                Entry::Sentence(_) => ("联想", String::new()),
            };
            println!("{:>2}. {}  [{origin} {kind}]", i + 1, entry.text());
        }
    }
    Ok(())
}
