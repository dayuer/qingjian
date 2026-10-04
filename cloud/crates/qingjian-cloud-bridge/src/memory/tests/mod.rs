//! 本地记忆的单元测试，按主题分文件；这里放共用的临时目录与样例对象、卡片。

mod date;
mod store;
mod sync;

use std::path::PathBuf;

use qingjian_cloud_proto::{CardKind, CardSource, Scene};

use crate::memory::{Card, Contact, Pronoun};

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("qj-memory-{name}-{}", std::process::id()));
    std::fs::remove_dir_all(&dir).ok();
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// 第 `n` 个样例 id（32 位十六进制）。
fn id(n: u32) -> String {
    format!("{n:032x}")
}

fn contact(n: u32, scene: Scene) -> Contact {
    Contact {
        id: id(n),
        name: format!("人{n}"),
        pronoun: Pronoun::Ta,
        scene,
        created_at: 1_791_043_200,
        hint_on: true,
        remind_on: true,
    }
}

/// 卡片 id 取 `id(1000 + n)`，与对象 id 错开。
fn card(
    n: u32,
    kind: CardKind,
    text: &str,
    keywords: &[&str],
    when: Option<&str>,
    touched_at: i64,
) -> Card {
    Card {
        id: id(1000 + n),
        kind,
        text: text.to_owned(),
        keywords: keywords.iter().map(|k| (*k).to_owned()).collect(),
        when: when.map(str::to_owned),
        source: CardSource::Manual,
        confirmed: true,
        faded: false,
        seq: 0,
        updated_at: 0,
        created_at: 1_791_043_200,
        touched_at,
    }
}
