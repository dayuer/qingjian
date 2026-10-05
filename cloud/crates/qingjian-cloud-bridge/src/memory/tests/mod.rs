//! 本地记忆的单元测试，按主题分文件；这里放共用的临时目录与样例对象、卡片。

mod bad_card;
mod contact;
mod date;
mod display_name;
mod hint;
mod initial;
mod materials;
mod store;
mod sync;

use std::path::PathBuf;

use qingjian_cloud_proto::{CardKind, CardSource};

use crate::memory::{Card, Contact, Pronoun};
use crate::scope::ContactPick;

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

/// 指定第 `n` 个样例对象。
fn pick(n: u32) -> ContactPick {
    ContactPick::Contact(id(n))
}

fn contact(n: u32) -> Contact {
    Contact {
        id: id(n),
        name: format!("人{n}"),
        display_name: None,
        initial: None,
        pronoun: Pronoun::Ta,
        pinned_at: None,
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
