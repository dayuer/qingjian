//! 本地记忆的单元测试，按主题分文件；这里放共用的临时目录与样例对象、卡片。

mod bad_card;
mod date;
mod display_name;
mod hint;
mod initial;
mod materials;
mod scene;
mod store;
mod sync;

use std::path::{Path, PathBuf};

use qingjian_cloud_proto::{CardKind, CardSource};

use crate::memory::{Card, Contact, MemoryStore, Pronoun, Scene};
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

/// 老模型里场景是写死的三个（现在要显式建）；多数用例沿用这三个 id，省得每条都建。
const THREE_SCENES: [(&str, &str); 3] = [("daily", "日常"), ("dating", "恋爱"), ("work", "工作")];

/// 开一个临时目录里的 store，并把上面三个场景建好。
fn open_with_scenes(user: &Path) -> MemoryStore {
    let store = MemoryStore::open(user);
    memory_scenes(&store);
    store
}

/// 给一个已经开好的 store 建好那三个场景（等锁上限自定、由调用方自己开的那些用例用）。
fn memory_scenes(store: &MemoryStore) {
    for (id, name) in THREE_SCENES {
        store
            .put_scene(Scene::new(id.to_owned(), name.to_owned(), 0))
            .unwrap();
    }
}

fn contact(n: u32, scene: &str) -> Contact {
    Contact {
        id: id(n),
        name: format!("人{n}"),
        display_name: None,
        initial: None,
        pronoun: Pronoun::Ta,
        scene: scene.to_owned(),
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
