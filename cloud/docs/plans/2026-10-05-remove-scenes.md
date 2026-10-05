# 去掉场景（人 + 平铺名单）实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 把「场景」这一层从桥、键盘、App 里整个去掉——名单是一张平铺的人，键盘工具栏左边切人、右边（下一份计划）切技能。

**Architecture:** 先动桥的数据模型（`contacts.json` / `state.json` 不再有场景），再改 C 接口，最后两个壳跟着改。桥是唯一同时被 App 与键盘依赖的层，所以必须第一个改完并测试通过，壳才动。老的场景文件（`scenes.json`、`scene-*/`）在迁移里清掉，但**上一版承诺保留 30 天的 `scene-*.migrated-*` 备份不动**。

**Tech Stack:** Rust（`cloud/crates/qingjian-cloud-bridge`，`cargo test -p qingjian-cloud-bridge`）、Swift 6 / SwiftUI / UIKit（`cloud/ios`，XCTest）、XcodeGen。

**需求、要清掉的清单与验收标准**在 [人 + 技能包](2026-10-05-people-and-skills.md)；
技能包本身见 [改写技能包](2026-10-05-rewrite-skills.md) 与 [它的实施计划](2026-10-05-rewrite-skills.md)（第二份）。

**顺序：** 这份先做，做完是一个能跑的版本；技能包那份在它之上。

---

## 文件结构

| 文件 | 职责 | 这次的动作 |
|---|---|---|
| `cloud/crates/qingjian-cloud-bridge/src/memory/scene.rs` | 场景类型与 `scenes.json` | **删除** |
| `.../memory/contact.rs` | 一个人的字段 | 删 `scene` |
| `.../memory/snapshot.rs` | App 整份读写的 JSON | 删 `scenes` |
| `.../memory/store.rs` | `memory/` 下的文件读写与迁移 | 删场景接口、`check_pinned` 改全局、`clear_scenes` |
| `.../memory/error.rs` | 错误文案 | `PinLimit` 改文案 |
| `.../memory/mod.rs` | 模块与 `sanitized_scope` / `scope_with` | 去场景比对 |
| `.../memory/ffi.rs` | C 接口 | 三个签名去掉场景 |
| `include/qingjian_bridge.h` | C 头 | 同步 |
| `.../scope/state.rs` | `state.json` | 删 `scene` / `last` |
| `.../scope/pick.rs` | 怎么定当前对象 | `Last` → `Keep` |
| `.../session/memory/mod.rs` | 键盘侧的切人与建人 | 改签名 |
| `.../session/memory/pending/mod.rs` | 拿不到锁时的待办 | 元组去掉场景 |
| `cloud/crates/qingjian-cloud-proto/src/{memory_item,contact_registration}.rs` | 上传协议 | `scene` → `Option<String>` |
| `cloud/ios/Keyboard/Sources/ScopePicker.swift` | 场景/对象选择面板 | **删除** |
| `cloud/ios/Keyboard/Sources/{KeyboardPanel,KeyboardView,IdleBar,CandidateBar,ScopeChip,KeyboardModel,MemoryBridge,Engine}.swift` | 键盘 | 去场景 |
| `cloud/ios/Shared/Memory/{ScopeDisplay,MemoryScope,ScopePick}.swift` | 键盘与 App 共用 | 去场景 |
| `cloud/ios/App/Me/{SceneListSection,SceneNameSheet,SceneSettingsView}.swift` | 「我」页的场景一组 | **删除** |
| `cloud/ios/Shared/Memory/{MemoryScene,SceneGroup}.swift` | 场景与分组类型 | **删除** |
| `cloud/ios/App/**` | App 其余各处 | 见 Task 8 |

---

## Task 1：`Contact.scene` 与 `ScopeState.scene` / `last` 拆掉

**Files:**
- Modify: `cloud/crates/qingjian-cloud-bridge/src/memory/contact.rs`
- Modify: `cloud/crates/qingjian-cloud-bridge/src/scope/state.rs`
- Modify: `cloud/crates/qingjian-cloud-bridge/src/memory/mod.rs`
- Modify: `cloud/crates/qingjian-cloud-bridge/src/scope/pick.rs`
- Test: `cloud/crates/qingjian-cloud-bridge/src/memory/tests/contact.rs`（新建）

- [ ] **Step 1: 写失败的测试**

新建 `cloud/crates/qingjian-cloud-bridge/src/memory/tests/contact.rs`：

```rust
//! `Contact` 与 `ScopeState` 去场景之后的形状：老文件里的 `scene` / `last` 读得进、写出去就没有了。

use super::{contact, id, temp_dir};
use crate::memory::{Contact, MemoryStore};
use crate::scope::ScopeState;

#[test]
fn an_old_contact_with_a_scene_reads_and_writes_without_it() {
    let user = temp_dir("contact-no-scene");
    let store = MemoryStore::open(&user);
    store
        .put_contact(contact(1, "dating")) // 注意：这个 helper 的第二个参数这一版之后要删掉
        .unwrap();

    let path = user.join("memory").join("contacts.json");
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(!text.contains("\"scene\""), "写出去不该再有 scene：{text}");

    // 老文件（带 scene）照旧读得进来
    std::fs::write(
        &path,
        format!(
            r#"[{{"id":"{}","name":"小美","scene":"dating","created_at":1}}]"#,
            id(1)
        ),
    )
    .unwrap();
    let read: Vec<Contact> = store.contacts();
    assert_eq!(read.len(), 1);
    assert_eq!(read[0].name, "小美");
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn an_old_state_with_scene_and_last_reads_without_them() {
    let user = temp_dir("state-no-scene");
    let store = MemoryStore::open(&user);
    std::fs::write(
        user.join("memory").join("state.json"),
        format!(r#"{{"scene":"dating","contact_id":"{}","last":{{"dating":"{}"}}}}"#, id(1), id(1)),
    )
    .unwrap();
    let state: ScopeState = store.state();
    assert_eq!(state.contact_id, Some(id(1)), "多出来的 scene / last 忽略，contact_id 留着");
    std::fs::remove_dir_all(&user).ok();
}
```

在 `memory/tests/mod.rs` 的 `mod` 列表里加 `mod contact;`（按字母序放在 `mod date;` 之后）。

- [ ] **Step 2: 跑测试，确认失败**

```bash
cargo test --manifest-path cloud/Cargo.toml -p qingjian-cloud-bridge an_old_contact
```

预期：编译失败，`error[E0061]: this function takes 3 arguments but 2 arguments were supplied`（`contact()` 还要场景参数）
或断言失败 `写出去不该再有 scene`。

- [ ] **Step 3: 改代码**

`memory/contact.rs`：删掉

```rust
    /// 所属场景的 id（`scenes.json` 里的一项）；场景只是分组，人换场景不受限。
    pub scene: String,
```

`MAX_PINNED` 的文档改成：

```rust
/// 全局最多几个置顶（键盘列人时先摆他们）。
pub const MAX_PINNED: usize = 4;
```

`scope/state.rs` 整个文件换成：

```rust
//! `memory/state.json`：键盘当前选的人、各人上次被选的时间（键盘写，App 不改）。
//! 提示开关在各个对象上（`Contact`），不在这里。2026-10-05 起没有「场景」，这个文件里也就没有场景相关的东西。

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ScopeState {
    /// 当前对象。
    pub contact_id: Option<String>,

    /// 对象 id → 上次在键盘里选中这个人的 Unix 秒（列人时按沟通情况排用）；旧文件没有时为空。
    pub used: BTreeMap<String, i64>,
}

impl Default for ScopeState {
    fn default() -> Self {
        Self {
            contact_id: None,
            used: BTreeMap::new(),
        }
    }
}
```

`memory/mod.rs` 的两个函数换成：

```rust
/// 当前对象必须在名单上，否则退回不指定；`used` 里不在名单上的人也去掉。
pub(crate) fn sanitized_scope(mut state: ScopeState, contacts: &[Contact]) -> ScopeState {
    if !state
        .contact_id
        .as_deref()
        .is_some_and(|id| contacts.iter().any(|c| c.id == id))
    {
        state.contact_id = None;
    }
    state
        .used
        .retain(|id, _| contacts.iter().any(|c| &c.id == id));
    state
}

/// 按 `pick` 定当前对象。
pub(crate) fn scope_with(state: ScopeState, pick: &ContactPick, contacts: &[Contact]) -> ScopeState {
    let mut state = sanitized_scope(state, contacts);
    state.contact_id = match pick {
        // 保持现在选的人：sanitize 完直接回去（原来靠 `state.last`，场景去掉后没有「上次」了）
        ContactPick::Keep => return state,
        ContactPick::Nobody => None,
        ContactPick::Contact(id) => Some(id.clone()),
    };
    sanitized_scope(state, contacts)
}
```

`scope/pick.rs`：枚举与 `from_arg` 换成

```rust
//! 切人时怎么定当前对象：保持现在选的人、明确不指定，或指定一个人。

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContactPick {
    /// 保持现在选的人不变（幂等）。原来是「回到这个场景上次选的人」，场景去掉后没有「上次」了。
    Keep,

    /// 明确不指定。
    Nobody,

    Contact(String),
}

impl ContactPick {
    /// C 接口的约定：空指针是 [`Self::Keep`]，空字符串是 [`Self::Nobody`]，其余是对象 id。
    pub fn from_arg(arg: Option<&str>) -> Self {
        match arg {
            None => Self::Keep,
            Some("") => Self::Nobody,
            Some(id) => Self::Contact(id.to_owned()),
        }
    }
}
```

- [ ] **Step 4: 跑测试，确认通过**

```bash
cargo test --manifest-path cloud/Cargo.toml -p qingjian-cloud-bridge an_old_contact
cargo test --manifest-path cloud/Cargo.toml -p qingjian-cloud-bridge an_old_state
```

预期：两条都 `test result: ok`。

- [ ] **Step 5: 提交**

```bash
git add cloud/crates/qingjian-cloud-bridge
git commit -m "refactor(cloud): Contact 与 ScopeState 去掉场景字段"
```

---

## Task 2：删 `memory/scene.rs`，`MemoryStore` 拆掉场景

**Files:**
- Delete: `cloud/crates/qingjian-cloud-bridge/src/memory/scene.rs`
- Modify: `cloud/crates/qingjian-cloud-bridge/src/memory/{mod,snapshot,store,error}.rs`
- Test: `cloud/crates/qingjian-cloud-bridge/src/memory/tests/contact.rs`（接着写）

- [ ] **Step 1: 写失败的测试**

在 `memory/tests/contact.rs` 里加：

```rust
#[test]
fn pinning_is_counted_globally() {
    let user = temp_dir("pin-global");
    let store = MemoryStore::open(&user);
    for n in 0..crate::memory::MAX_PINNED as u32 {
        let mut who = contact(n, "daily"); // 第二个参数这一版删掉
        who.pinned_at = Some(i64::from(n));
        store.put_contact(who).unwrap();
    }
    let mut fifth = contact(9, "daily");
    fifth.pinned_at = Some(9);
    assert!(matches!(
        store.put_contact(fifth),
        Err(crate::memory::MemoryError::PinLimit)
    ));
    assert_eq!(
        store.put_contact(contact(8, "daily")).unwrap_err().message(),
        "最多置顶 4 个人"
    );
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn the_snapshot_has_no_scenes() {
    let user = temp_dir("snapshot-no-scenes");
    let store = MemoryStore::open(&user);
    store.put_contact(contact(1, "daily")).unwrap();
    let json = serde_json::to_string(&store.snapshot().unwrap()).unwrap();
    assert!(!json.contains("scenes"), "整份读写的 JSON 里不该再有 scenes：{json}");
    std::fs::remove_dir_all(&user).ok();
}
```

- [ ] **Step 2: 跑测试，确认失败**

```bash
cargo test --manifest-path cloud/Cargo.toml -p qingjian-cloud-bridge pinning_is_counted_globally
cargo test --manifest-path cloud/Cargo.toml -p qingjian-cloud-bridge the_snapshot_has_no_scenes
```

预期：第一条失败在 `PinLimit` 那步（现在按场景各自算 4 个，第 5 个不报错）；
第二条失败在 `不该再有 scenes`。

- [ ] **Step 3: 改代码**

删文件 `cloud/crates/qingjian-cloud-bridge/src/memory/scene.rs`。

`memory/mod.rs`：删 `mod scene;` 与 `pub use self::scene::{DEFAULT_SCENE_ID, DEFAULT_SCENE_NAME, MAX_SCENE_NAME_CHARS, Scene, is_scene_id, validate_scenes};`。

`memory/snapshot.rs`：删字段与解码

```rust
// 删掉这两处：
//     pub scenes: Vec<Scene>,
//     scenes = try container.decodeIfPresent([MemoryScene].self, forKey: .scenes) ?? []
// 结构体的文档改成：{"contacts":[…],"cards":{id:[…]},"revs":{id:n},"state":{…},"broken":[id…]}
```

`memory/store.rs` 删掉这些（一个不留）：`SCENES_FILE`、`scenes()`、`try_scenes()`、`put_scene`、`delete_scene`、
`scenes_path()`、`read_scenes()`、`adopt_unknown_scenes`、`check_scene_exists`（若还在）、
`park_legacy_scene_dirs`，以及 `migrate_if_needed` 里并场景那几段（Task 3 再补上清场）。
`use super::{…}` 里 `DEFAULT_SCENE_ID` / `DEFAULT_SCENE_NAME` / `Scene` / `validate_scenes` 一起去掉。

`check_pinned` 换成：

```rust
/// 全局最多 [`MAX_PINNED`] 个置顶。
fn check_pinned(contacts: &[Contact]) -> Result<(), MemoryError> {
    let pinned = contacts.iter().filter(|c| c.pinned_at.is_some()).count();
    if pinned > MAX_PINNED {
        return Err(MemoryError::PinLimit);
    }
    Ok(())
}
```

两个调用点（`put_contact` / `write_snapshot`）从 `check_pinned(&contacts, &scenes)?` 改成 `check_pinned(&contacts)?`。

`put_contact` 里 `let scenes = self.read_scenes()?;` 与 `adopt_unknown_scenes(...)` 都删掉。

`update_scope(scene, pick, now)` 换成：

```rust
    /// 键盘切当前对象：在锁里重读 `state.json` 与名单，按 `pick` 定对象；切到了某人时把 `now` 记进 `used`。
    pub fn update_contact(&self, pick: &ContactPick, now: i64) -> Result<ScopeState, MemoryError> {
        let _lock = self.lock()?;
        let contacts = self.read_contacts()?;
        let (state, _): (ScopeState, bool) = read_json(&self.state_path())?;
        let mut state = scope_with(state, pick, &contacts);
        if let Some(id) = state.contact_id.clone() {
            state.used.insert(id, now);
        }
        write_json(&self.state_path(), &state)?;
        Ok(state)
    }
```

`memory/error.rs`：`PinLimit` 的文案换成

```rust
            Self::PinLimit => format!("最多置顶 {MAX_PINNED} 个人"),
```

`lib.rs`：`pub use self::memory::{…}` 里去掉 `DEFAULT_SCENE_ID` / `DEFAULT_SCENE_NAME` / `MAX_SCENE_NAME_CHARS` /
`Scene` / `is_scene_id` / `validate_scenes`。

- [ ] **Step 4: 跑测试，确认通过**

```bash
cargo test --manifest-path cloud/Cargo.toml -p qingjian-cloud-bridge pinning_is_counted_globally
cargo test --manifest-path cloud/Cargo.toml -p qingjian-cloud-bridge the_snapshot_has_no_scenes
```

预期：两条 `test result: ok`。

- [ ] **Step 5: 提交**

```bash
git add cloud/crates/qingjian-cloud-bridge
git commit -m "refactor(cloud): 删掉场景类型与每场景的置顶计数"
```

---

## Task 3：迁移改成清场（保留 `.migrated-*` 备份）

**Files:**
- Modify: `cloud/crates/qingjian-cloud-bridge/src/memory/store.rs`
- Test: `cloud/crates/qingjian-cloud-bridge/src/memory/tests/migrate.rs`（新建）

- [ ] **Step 1: 写失败的测试**

新建 `cloud/crates/qingjian-cloud-bridge/src/memory/tests/migrate.rs`：

```rust
//! 上一版的场景数据怎么清：`scenes.json` 与没改过名的 `scene-*/` 删掉，
//! 但 `scene-*.migrated-<日期>` 是上一版承诺保留 30 天的备份，不能顺手删。

use super::{card, contact, id, temp_dir};
use crate::memory::{LocalDate, MemoryStore};

/// 造一份上一版留下的数据：场景文件、场景目录、改名过的备份、带 scene 字段的人与状态。
fn seed_legacy(user: &std::path::Path) {
    let memory = user.join("memory");
    std::fs::create_dir_all(memory.join("scene-dating").join("learning")).unwrap();
    std::fs::create_dir_all(memory.join("scene-dating.migrated-2026-10-05").join("learning")).unwrap();
    std::fs::write(
        memory.join("scene-dating.migrated-2026-10-05").join("learning").join("user.tsv"),
        "宝贝\t10\n",
    )
    .unwrap();
    std::fs::write(
        memory.join("scenes.json"),
        r#"[{"id":"daily","name":"日常","created_at":1}]"#,
    )
    .unwrap();
    std::fs::write(
        memory.join("contacts.json"),
        format!(r#"[{{"id":"{}","name":"小美","scene":"dating","created_at":1}}]"#, id(1)),
    )
    .unwrap();
    std::fs::create_dir_all(memory.join(id(1))).unwrap();
    std::fs::write(
        memory.join(id(1)).join("cards.json"),
        r#"{"rev":1,"cards":[]}"#,
    )
    .unwrap();
    std::fs::write(
        memory.join("state.json"),
        format!(r#"{{"scene":"dating","contact_id":"{}","last":{{"dating":"{}"}}}}"#, id(1), id(1)),
    )
    .unwrap();
}

#[test]
fn clearing_scenes_keeps_the_migrated_backup() {
    let user = temp_dir("clear-scenes");
    seed_legacy(&user);
    let store = MemoryStore::open(&user);
    let _ = store.contacts(); // 任何一次读写都会走一遍清场

    let memory = user.join("memory");
    assert!(!memory.join("scenes.json").exists(), "场景文件删掉");
    assert!(!memory.join("scene-dating").exists(), "没改过名的场景目录删掉");
    let backup = memory.join("scene-dating.migrated-2026-10-05");
    assert!(backup.join("learning").join("user.tsv").is_file(), "改名过的备份留着：{backup:?}");

    assert_eq!(store.contacts().len(), 1, "人一个不少");
    let state = store.state();
    assert_eq!(state.contact_id, Some(id(1)), "当前对象留着");
    let json = std::fs::read_to_string(memory.join("state.json")).unwrap();
    assert!(!json.contains("\"scene\""), "state.json 里不再写场景");
    assert!(!json.contains("\"last\""), "state.json 里不再写 last");
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn an_expired_migrated_backup_is_swept() {
    let user = temp_dir("sweep-migrated");
    let memory = user.join("memory");
    std::fs::create_dir_all(memory.join("scene-dating.migrated-2026-08-01")).unwrap();
    let store = MemoryStore::open(&user);
    store.sweep_migrated_dirs(LocalDate::today());
    assert!(
        !memory.join("scene-dating.migrated-2026-08-01").exists(),
        "改名满 30 天的备份清掉"
    );
    std::fs::remove_dir_all(&user).ok();
}
```

`memory/tests/mod.rs` 的 `mod` 列表里加 `mod migrate;`。

- [ ] **Step 2: 跑测试，确认失败**

```bash
cargo test --manifest-path cloud/Cargo.toml -p qingjian-cloud-bridge clearing_scenes_keeps_the_migrated_backup
```

预期：失败在 `场景文件删掉`（现在没有任何清场逻辑）。

- [ ] **Step 3: 改代码**

`memory/store.rs` 里 `migrate_if_needed` 换成（仍挂在 `lock()` 拿锁之后）：

```rust
    /// 上一版留下的场景数据：删 `scenes.json` 与没改过名的 `scene-*/`。
    /// `scene-*.migrated-<日期>` **不动**——那是上一版承诺保留 30 天的备份，由 [`Self::sweep_migrated_dirs`] 到期再清。
    /// 幂等（没东西可删也算成功），每次拿锁都会走一遍（就是两个 stat 加一次 readdir）。
    fn clear_scenes(&self) -> Result<(), MemoryError> {
        remove_file(&self.scenes_path())?;
        for entry in self.memory_entries()? {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name.starts_with("scene-") && !name.contains(".migrated-") {
                remove_dir(&entry.path())?;
            }
        }
        Ok(())
    }
```

`lock()` 里原来是 `self.migrate_if_needed()?;`，换成 `self.clear_scenes()?;`。
`remove_dir` 旁边加：

```rust
fn remove_file(path: &Path) -> Result<(), MemoryError> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}
```

- [ ] **Step 4: 跑测试，确认通过**

```bash
cargo test --manifest-path cloud/Cargo.toml -p qingjian-cloud-bridge clearing_scenes
cargo test --manifest-path cloud/Cargo.toml -p qingjian-cloud-bridge an_expired_migrated_backup_is_swept
```

预期：两条 `test result: ok`。

- [ ] **Step 5: 提交**

```bash
git add cloud/crates/qingjian-cloud-bridge
git commit -m "refactor(cloud): 迁移改成清掉场景数据，30 天备份留着"
```

---

## Task 4：C 接口去掉场景

**Files:**
- Modify: `cloud/crates/qingjian-cloud-bridge/src/memory/ffi.rs`
- Modify: `cloud/crates/qingjian-cloud-bridge/include/qingjian_bridge.h`
- Modify: `cloud/crates/qingjian-cloud-bridge/src/session/memory/{mod,pending/mod}.rs`
- Test: `cloud/crates/qingjian-cloud-bridge/tests/memory_ffi.rs`

- [ ] **Step 1: 写失败的测试**

`tests/memory_ffi.rs` 的 `scope_set_round_trips` 换成：

```rust
#[test]
fn scope_set_round_trips_without_scenes() {
    let (data, user) = dirs("scope");
    seed(&user);
    let session = open(&data, Some(&user));
    let scope = json_of(unsafe { qj_scope_get(session) });
    assert_eq!(scope["contact_id"], Value::Null);
    assert!(scope.get("scene").is_none(), "没有场景了：{scope}");
    assert!(scope.get("last").is_none(), "没有 last 了：{scope}");

    set_contact(session, Some(CONTACT));
    assert_eq!(json_of(unsafe { qj_scope_get(session) })["contact_id"], CONTACT);

    // 空指针 = 保持现在选的人不变（幂等）
    set_contact(session, None);
    assert_eq!(json_of(unsafe { qj_scope_get(session) })["contact_id"], CONTACT);

    // 空字符串 = 明确不指定
    set_contact(session, Some(""));
    assert_eq!(json_of(unsafe { qj_scope_get(session) })["contact_id"], Value::Null);

    // 名单上没有的对象当不指定
    set_contact(session, Some("ffffffffffffffffffffffffffffffff"));
    assert_eq!(json_of(unsafe { qj_scope_get(session) })["contact_id"], Value::Null);
    unsafe { qj_session_free(session) };
}
```

`tests/memory_support/mod.rs` 里把 `set_scope(session, scene, contact)` 换成：

```rust
/// 切当前对象；`None` 是空指针（保持现在选的人不变），`Some("")` 是不指定。
pub fn set_contact(session: *mut Session, contact: Option<&str>) {
    let raw = contact.map(c);
    let ptr = raw.as_ref().map_or(ptr::null(), |s| s.as_ptr());
    unsafe { qj_scope_set(session, ptr) };
}
```

（`set_scope` 与 `scenes()` 一起删掉；调用点全部改成 `set_contact`。）

`tests/memory_scene_ffi.rs` 整个删掉，里面关于「按人隔离」的用例搬到 `tests/memory_ffi.rs`（见 Task 9）。

- [ ] **Step 2: 跑测试，确认失败**

```bash
cargo test --manifest-path cloud/Cargo.toml -p qingjian-cloud-bridge --test memory_ffi scope_set_round_trips_without_scenes
```

预期：编译失败 `qj_scope_set` 参数个数不对。

- [ ] **Step 3: 改代码**

`memory/ffi.rs`：

```rust
/// 切当前对象：`contact_id` 为 NULL 时保持现在选的人不变（幂等），为空字符串时明确不指定。
/// 名单上没有的对象当不指定；切到了某人时记下时间（列人时按沟通情况排用）。
///
/// # Safety
/// `session` 来自 `qj_session_open` 且未释放；`contact_id` 为空或有效 UTF-8 C 字符串。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qj_scope_set(session: *mut Session, contact_id: *const c_char) {
    let pick = ContactPick::from_arg(unsafe { path_arg(contact_id) });
    with(session, (), |s| s.set_contact(&pick));
}

/// `{"contact_id":"…"|null,"used":{"<id>":秒,…}}`；没有记忆的会话返回空指针。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qj_scope_get(session: *mut Session) -> *mut c_char {
    with(session, ptr::null_mut(), |s| {
        s.scope().map_or(ptr::null_mut(), |state| {
            let json = serde_json::json!({
                "contact_id": state.contact_id,
                "used": state.used,
            });
            owned(&json.to_string())
        })
    })
}

/// 新建一个对象：成功 `{"id":"…"}`，失败 `{"code","message"}`（lock_timeout / invalid）。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qj_memory_add_contact(
    session: *mut Session,
    name: *const c_char,
    pronoun: *const c_char,
) -> *mut c_char {
    let Some(name) = (unsafe { path_arg(name) }).map(str::to_owned) else {
        return owned(&MemoryError::Invalid("参数无效").to_json());
    };
    let pronoun = unsafe { path_arg(pronoun) }
        .and_then(|text| serde_json::from_value(serde_json::Value::String(text.to_owned())).ok())
        .unwrap_or_default();
    let added = with(session, Err(MemoryError::Invalid("参数无效")), |s| {
        s.memory_add_contact(&name, pronoun)
    });
    match added {
        Ok(id) => owned(&serde_json::json!({ "id": id }).to_string()),
        Err(error) => owned(&error.to_json()),
    }
}
```

`session/memory/mod.rs`：`set_scope(scene, pick)` 换成

```rust
    /// 切当前对象：交给 `MemoryStore::update_contact` 在锁里重读 `state.json` 与名单。
    /// 只是拿不到锁（`LockTimeout`）时内存里照切，写盘进待办稍后重试。
    pub fn set_contact(&mut self, pick: &ContactPick) {
        let Some(memory) = self.memory.as_mut() else {
            return;
        };
        let mut deferred = false;
        let next = match memory.store.update_contact(pick, now_unix()) {
            Ok(state) => {
                memory.pending.take_scope();
                state
            }
            Err(MemoryError::LockTimeout) => {
                deferred = true;
                let wanted = scope_with(memory.state.clone(), pick, &memory.contacts);
                memory.pending.set_scope(wanted.contact_id.clone());
                wanted
            }
            Err(error) => {
                tracing::warn!(%error, "切对象没写进 state.json，不切");
                return;
            }
        };
        let moved = next.contact_id != memory.state.contact_id;
        memory.state = next;
        if moved {
            self.switch_layers(deferred);
        }
    }
```

`memory_add_contact(name, pronoun, scene)` 去掉第三个参数，`Contact` 字面量里 `scene` 那行删掉。

`session/memory/pending/mod.rs`：`scope: Option<(String, Option<String>)>` → `scope: Option<Option<String>>`；
`set_scope(&mut self, contact: Option<String>)`、`take_scope() -> Option<Option<String>>`、
`restore_scope(scope: Option<String>)`。

`memory/ffi.rs` 里其余调用 `set_scope` 的地方（`qj_memory_add_contact` 之外的）一并改。

`include/qingjian_bridge.h` 同步三个签名与注释（`qj_scope_set` 少一个参数、`qj_scope_get` 的返回、
`qj_memory_add_contact` 少一个参数），并删掉「场景」相关的说明段落。

- [ ] **Step 4: 跑测试，确认通过**

```bash
cargo test --manifest-path cloud/Cargo.toml -p qingjian-cloud-bridge
```

预期：`test result: ok`（全绿；`memory_scene_ffi` 已经删了）。

- [ ] **Step 5: 提交**

```bash
git add cloud/crates/qingjian-cloud-bridge
git commit -m "refactor(cloud): C 接口去掉场景，空指针改成「保持现在选的人」"
```

---

## Task 5：proto 的 `scene` 改成 `Option<String>`

**Files:**
- Modify: `cloud/crates/qingjian-cloud-proto/src/memory_item.rs`
- Modify: `cloud/crates/qingjian-cloud-proto/src/contact_registration.rs`
- Test: `cloud/crates/qingjian-cloud-proto/tests/memory.rs`

- [ ] **Step 1: 写失败的测试**

`cloud/crates/qingjian-cloud-proto/tests/memory.rs` 里把 `scene` 相关的断言换成：

```rust
#[test]
fn memory_item_carries_no_scene() {
    let item = MemoryItem {
        client_id: "c1".to_owned(),
        contact_id: None,
        scene: None,
        kind: MemoryKind::Note,
        text: "想去厦门".to_owned(),
        at: 1_791_043_200,
    };
    let json = serde_json::to_value(&item).unwrap();
    assert!(json.get("scene").is_none(), "客户端不填就不该出现：{json}");

    // 服务端回来的老数据带 scene 也读得进（忽略）
    let old: MemoryItem = serde_json::from_str(
        r#"{"client_id":"c1","contact_id":null,"scene":"dating","kind":"note","text":"x","at":1}"#,
    )
    .unwrap();
    assert_eq!(old.scene.as_deref(), Some("dating"));
}
```

- [ ] **Step 2: 跑测试，确认失败**

```bash
cargo test --manifest-path cloud/Cargo.toml -p qingjian-cloud-proto memory_item_carries_no_scene
```

预期：编译失败 `missing field scene` 或类型不匹配（现在是 `String`）。

- [ ] **Step 3: 改代码**

`memory_item.rs`：

```rust
    /// 场景 id。2026-10-05 起客户端没有「场景」了，这里恒为 `None` 且不序列化；
    /// 字段留着是为了老数据还能读进来（服务端也只要允许它缺省）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scene: Option<String>,
```

`contact_registration.rs` 同样改成 `Option<String>` + 同样的 serde 属性（注释按「登记对象时也不带场景」写）。

- [ ] **Step 4: 跑测试，确认通过**

```bash
cargo test --manifest-path cloud/Cargo.toml -p qingjian-cloud-proto
```

预期：`test result: ok`。

- [ ] **Step 5: 提交**

```bash
git add cloud/crates/qingjian-cloud-proto
git commit -m "refactor(proto)!: 素材的场景字段改成可选，客户端不再填"
```

---

## Task 6：键盘去掉场景（删面板、牌子只剩人）

**Files:**
- Delete: `cloud/ios/Keyboard/Sources/ScopePicker.swift`
- Modify: `cloud/ios/Keyboard/Sources/{KeyboardPanel,KeyboardView,IdleBar,CandidateBar,ScopeChip,KeyboardModel,MemoryBridge,Engine}.swift`
- Modify: `cloud/ios/Shared/Memory/{ScopeDisplay,MemoryScope,ScopePick}.swift`
- Modify: `cloud/ios/Tests/SceneGroupTests.swift`（改名与内容见 Task 9）

- [ ] **Step 1: 先改共用的纯值（`ScopeDisplay` / `ScopePick` / `MemoryScope`）**

`Shared/Memory/ScopePick.swift`：`.last` → `.keep`，注释改成「保持现在选的人不变」；`argument` 的 nil 分支照旧。

`Shared/Memory/MemoryScope.swift` 整个文件换成：

```swift
// 键盘当前的会话状态与各人上次被选中的时间（memory/state.json；qj_scope_get 给的就是这两项）。
// 提示开关在各个对象上（MemoryContact）。2026-10-05 起没有「场景」，这个类型里也就没有场景。

struct MemoryScope: Codable, Equatable, Sendable {
    var contactId: String?

    /// 对象 id → 上次在键盘里选中的 Unix 秒（列人时按沟通情况排用）。
    var used: [String: Int64] = [:]

    enum CodingKeys: String, CodingKey {
        case used
        case contactId = "contact_id"
    }
}

extension MemoryScope {
    /// 缺的字段按缺省（init 写在扩展里，成员逐一构造器才留得住）。
    init(from decoder: any Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        self.init()
        contactId = try container.decodeIfPresent(String.self, forKey: .contactId)
        used = try container.decodeIfPresent([String: Int64].self, forKey: .used) ?? [:]
    }
}
```

`Shared/Memory/ScopeDisplay.swift`：删 `pickerMode`、`PickerMode`、`cellStyle`、`CellStyle`、
`newContactSubtitle`、`noScopeSubtitle`、`unknownScene`；`quickPicks` 改成带截断：

```swift
    /// 点牌子列的人：本场景之外的人 + 「不指定」（当前就是不指定时不列）；nil 表示不指定。
    /// 名单平铺后人数不限，这一行不能无限长——按 `ContactOrder` 取前 `quickPickCount` 个，其余回 App 里切。
    static func quickPicks(people: [MemoryContact], current: String?, used: [String: Int64]) -> [String?] {
        let others = ContactOrder.ordered(people, used: used).map(\.id).filter { $0 != current }
        let head = Array(others.prefix(ContactOrder.quickPickCount))
        return current == nil ? head : head + [nil]
    }
```

- [ ] **Step 2: 删面板**

```bash
rm cloud/ios/Keyboard/Sources/ScopePicker.swift
```

`KeyboardPanel.swift`：删 `case scope` 与它的注释。
`KeyboardView.swift`：删 `case .scope: ScopePicker(model: model)` 那行。
`IdleBar.swift`：把两处 `model.panel == .scope ||` 里的 `.scope` 去掉（`panelBar` 与光标那处）。
`CandidateBar.swift`：`panelTakesTheBar` 去掉 `.scope`。

- [ ] **Step 3: 牌子只剩人**

`ScopeChip.swift` 整个文件换成：

```swift
// 工具栏左侧的牌子：圆点 + 人名的胶囊（设计稿 1a）。没选人时是「不指定」。
// 点它在工具栏里列出其他人 + 「不指定」（`KeyboardModel.toggleQuickPicks`）。
// 没开完全访问时读不到名单，这一块改成说明入口（见 Task 7）。

import SwiftUI

struct ScopeChip: View {
    let model: KeyboardModel

    /// 面板打开时牌子只是标题，点了不做事。
    var interactive = true

    var body: some View {
        HStack(spacing: 6) {
            Circle()
                .fill(love ? Theme.accent.color : Color(.tertiaryLabel))
                .frame(width: 7, height: 7)
            Text(ScopeDisplay.chipPerson(model.currentContact?.chipName))
                .font(.system(size: 13))
                .foregroundStyle(ColorUsage.chipPerson.role.color)
                .lineLimit(1)
        }
        .padding(.leading, 11)
        .padding(.trailing, 12)
        .frame(height: 30)
        .background(love ? Theme.accentSoft.color : KeyStyle.keyFill)
        .clipShape(Capsule())
        .shadow(color: .black.opacity(0.08), radius: 0, y: 1)
        .contentShape(Rectangle())
        .onKeyboardPress { if interactive { model.toggleQuickPicks() } }
        .accessibilityAddTraits(.isButton)
        .accessibilityLabel("对象：\(ScopeDisplay.chipPerson(model.currentContact?.chipName))")
    }

    private var love: Bool { ScopeDisplay.chipUsesAccent(fullAccess: model.fullAccess) }
```

（`showsPerson` 为假——没开完全访问——时整块换成说明入口，见 Task 7。）

- [ ] **Step 4: 改键盘侧的切人与建人**

`Engine.swift` / `MemoryBridge.swift`：

```swift
    /// 切当前对象：`.keep` 保持现在选的人不变（幂等），`.nobody` 明确不指定。
    func setScope(pick: ScopePick) {
        Self.withOptionalCString(pick.argument) { qj_scope_set(session, $0) }
    }

    /// 键盘里新建一个对象，称呼先按 TA（App 里能改）；建好返回 id。
    func addContact(name: String) -> Result<String, MemoryFailure> {
        let raw = name.withCString { qj_memory_add_contact(session, $0, nil) }
        return ContactAdd.parse(take(raw))
    }
```

`KeyboardModel.swift`：删 `chooseScene(_:)` 与 `openScopePicker()`。
现在的 `applyScope(scene:pick:)` 整段（含它调 `engine.setScope(scene:pick:)`）换成

```swift
    /// 换当前对象：先让桥在锁里读名单定对象，再按回来的状态重读名单与提示。
    private func applyScope(pick: ScopePick) {
        // 换了对象：没上屏的草稿卡先收掉
        endComposedNote()
        engine?.setScope(pick: pick)
        let next = engine?.scope ?? scope
        if next.contactId != scope.contactId { endComposedNote() }
        scope = next
        reloadContacts()
        refreshHint()
    }
```

`chooseContact(_:)` 里那一行 `applyScope(scene: scope.scene, pick: contactId.map(ScopePick.contact) ?? .nobody)`
改成 `applyScope(pick: contactId.map(ScopePick.contact) ?? .nobody)`；
`confirmNewContact` 里 `engine.addContact(name: name)`；`quickPicks` 的调用带上 `used`：

```swift
    /// 牌子右半展开时列的人（nil 是「不指定」）。
    var quickPicks: [String?] {
        ScopeDisplay.quickPicks(people: contacts, current: scope.contactId, used: scope.used)
    }
```

（原来是 `people`（本场景的人）；现在名单平铺，直接传 `contacts`。）

- [ ] **Step 5: 编译与测试**

```bash
cd cloud/ios
xcodegen generate
xcodebuild -project QingjianCloud.xcodeproj -scheme QingjianCloud \
  -destination 'platform=iOS Simulator,name=iPhone 17' -derivedDataPath build/scene-remove \
  -only-testing:QingjianCloudTests test
```

预期：`** TEST SUCCEEDED **`。App target 会因为 `MemoryScene` 等类型还在而跟着一起编——
**这一步只保证键盘 target 编得过**；App 的改动整块在 Task 8，所以这里先只跑
`-only-testing:QingjianCloudTests` 里与键盘有关的那几个类（`SceneGroupTests` 除外，它在 Task 9 改）。

- [ ] **Step 6: 提交**

```bash
git add cloud/ios
git commit -m "refactor(ios): 键盘去掉场景，牌子只剩人，选择面板删掉"
```

---

## Task 7：没开完全访问时的说明入口

**Files:**
- Modify: `cloud/ios/Shared/Memory/ScopeDisplay.swift`
- Modify: `cloud/ios/Keyboard/Sources/{ScopeChip,KeyboardModel}.swift`
- Test: `cloud/ios/Tests/NoFullAccessStyleTests.swift`

- [ ] **Step 1: 写失败的测试**

`Tests/NoFullAccessStyleTests.swift` 加：

```swift
    /// 没开完全访问时改写也用不了（iOS 键盘扩展没网络），说明要写清「改写和记忆都要它」。
    func testRewriteAlsoNeedsFullAccess() {
        XCTAssertEqual(
            ScopeDisplay.needsFullAccessForRewrite,
            "改写和记忆都要开完全访问。开了也不会上传你没让它上传的内容。")
        XCTAssertFalse(ScopeDisplay.chipShowsPerson(fullAccess: false))
        XCTAssertTrue(ScopeDisplay.chipShowsPerson(fullAccess: true))
    }
```

- [ ] **Step 2: 跑测试，确认失败**

```bash
cd cloud/ios && xcodebuild … -only-testing:QingjianCloudTests/NoFullAccessStyleTests test
```

预期：编译失败 `type 'ScopeDisplay' has no member 'needsFullAccessForRewrite'`。

- [ ] **Step 3: 改代码**

`ScopeDisplay.swift` 加：

```swift
    /// 没开完全访问时的说明：**改写与记忆都要它**——iOS 键盘扩展没开就没有网络，
    /// 改写要请求服务器，按下去必然失败，所以那颗按钮也并进这个说明入口。
    static let needsFullAccessForRewrite = "改写和记忆都要开完全访问。开了也不会上传你没让它上传的内容。"
```

`KeyboardModel` 加：

```swift
    /// 没开完全访问时点牌子展开的说明（改写与记忆都要完全访问）。
    private(set) var showsFullAccessNote = false

    func toggleFullAccessNote() { showsFullAccessNote.toggle() }
```

`ScopeChip`：`showsPerson` 为假时整块换成说明入口：

```swift
    var body: some View {
        if ScopeDisplay.chipShowsPerson(fullAccess: model.fullAccess) {
            personChip
        } else {
            Text(ScopeDisplay.needsFullAccessForRewrite)
                .font(.system(size: 12.5))
                .foregroundStyle(ColorUsage.cardNotice.role.color)
                .lineLimit(1)
                .padding(.horizontal, 10)
                .frame(height: 30)
                .contentShape(Rectangle())
                .onKeyboardPress { model.toggleFullAccessNote() }
                .accessibilityAddTraits(.isButton)
        }
    }
```

展开的那几句（`needsFullAccessText` + `fullAccessPath`）放在工具栏那一行的展开态里，照现在
`ScopePicker.noAccess` 的排版抄——那段代码删除前先把它挪到 `IdleBar` 的一个分支里。

- [ ] **Step 4: 跑测试，确认通过**

```bash
cd cloud/ios && xcodebuild … -only-testing:QingjianCloudTests test
```

预期：`** TEST SUCCEEDED **`。

- [ ] **Step 5: 提交**

```bash
git add cloud/ios
git commit -m "feat(ios): 没开完全访问时改为在牌子上说明（改写也要完全访问）"
```

---

## Task 8：App 去掉场景

**Files:**
- Delete: `cloud/ios/App/Me/{SceneListSection,SceneNameSheet,SceneSettingsView}.swift`
- Delete: `cloud/ios/Shared/Memory/{MemoryScene,SceneGroup}.swift`
- Modify: 见下

- [ ] **Step 1: 删文件**

```bash
cd cloud/ios
rm App/Me/SceneListSection.swift App/Me/SceneNameSheet.swift App/Me/SceneSettingsView.swift
rm Shared/Memory/MemoryScene.swift Shared/Memory/SceneGroup.swift
```

- [ ] **Step 2: 类型与快照**

`Shared/Memory/MemorySnapshot.swift`：删 `scenes` 字段、`CodingKeys` 里的 `scenes`、`init(from:)` 里读它那行。
`Shared/Memory/MemoryContact.swift`：`scene` 字段删掉（`new(name:pronoun:)` 不再要它）、CodingKeys 与 `init(from:)` 同步。

- [ ] **Step 3: `MemoryStore`**

删 `scenes` / `sceneName(of:)` / `defaultScene` / `addScene` / `renameScene` / `deleteScene`；
`pinnedCount(in:)` 换成

```swift
    /// 已经置顶了几个（全局最多 4 个）。
    var pinnedCount: Int { snapshot.contacts.filter { $0.pinnedAt != nil }.count }

    /// 名单里的人，按置顶与沟通情况排（补上素材时选人用）。
    var people: [MemoryContact] {
        ContactOrder.ordered(snapshot.contacts, used: snapshot.state.used)
    }
```

`groups` 删掉（`AssignSheet` 改用 `people`）。

- [ ] **Step 4: 通讯录「+」直接建人**

`App/Contacts/ContactsView.swift`：

```swift
    /// 「+」：直接开建人页（没有场景要选了）。
    private var addButton: some View {
        Button { adding = true } label: {
            Image(systemName: "plus")
                .font(AppFont.font(size: 22))
                .foregroundStyle(Theme.ink)
                .frame(width: 28, height: 28)
        }
        .disabled(!store.canEdit)
        .opacity(store.canEdit ? 1 : 0.4)
        .accessibilityLabel("加一个人")
        .accessibilityIdentifier("addContact")
    }
```

`.sheet(isPresented: $adding) { ContactEditor(store: store) }`；删 `@State newScene`；
行里的 `sceneName:` 参数删掉。

- [ ] **Step 5: 其余各处**

- `App/Memory/ContactEditor.swift`：删 `let scene: String`、「加在「X」里」那句、`MemoryContact.new(… scene:)` 的参数。
- `App/Onboarding/OnboardingView.swift`：`ContactEditor(store: store)`。
- `App/Contacts/ContactRow.swift`：删 `let sceneName` 与那行灰字。
- `App/Contacts/ContactIndex.swift`：`search(_:text:)`（去掉 `sceneName` 形参）。
- `App/Memory/ContactSettingsView.swift`：删「所在场景」Picker；`pinnedCount(in:)` → `store.pinnedCount`；
  `MemoryScene.maxPinned` → 字面量或 `SceneGroup` 删掉后新建的常量（放 `MemoryStore.pinLimit = 4`）。
- `App/Memory/MemoryWording.swift`：删 `unknownScene` / `sceneNameTooLong` / `sceneHasPeople` / `peopleCount`；
  `pinLimit()` 改成 `"最多置顶 \(MemoryStore.pinLimit) 个人"`；`pinNote(_:)` 里的「这个场景已经置顶」改成「已经置顶」。
- `App/Remember/AssignSheet.swift`：`ForEach(store.people)`，去掉 `Section(group.name)` 那层。
- `App/Remember/RememberView.swift`：功勋路回退改成 `contacts.first`。
- `App/Me/MeView.swift`：删 `SceneListSection(...)` 那行与 `@State addingScene` 与 `.sheet`。
- `App/Memory/CloudIntroView.swift`：「键盘会在你选的场景里，把你发出的话……」→
  「键盘会把你在聊天里发出的话、你记的一笔，去掉手机号、地址这类信息后上传，每天整理成记忆卡，等你确认了才生效。」
- `App/Onboarding/PlanStep.swift`：`"你开启的场景里，键盘自己记、每天整理"` → `"键盘在聊天里自己记、每天整理"`。
- `Shared/Memory/MemoryFailure.swift`：`pinLimit` 兜底文案改成「最多置顶 4 个人」。

- [ ] **Step 6: 编译与测试**

```bash
cd cloud/ios && xcodegen generate
xcodebuild -project QingjianCloud.xcodeproj -scheme QingjianCloud \
  -destination 'platform=iOS Simulator,name=iPhone 17' -derivedDataPath build/scene-remove \
  -only-testing:QingjianCloudTests test
```

预期：`** TEST SUCCEEDED **`。

- [ ] **Step 7: 提交**

```bash
git add cloud/ios
git commit -m "refactor(ios): App 去掉场景，建人不再选场景"
```

---

## Task 9：测试、走查与文档收尾

**Files:**
- Delete: `cloud/ios/UITests/SceneShots.swift`
- Modify: `cloud/ios/Tests/*`、`cloud/ios/UITests/*`、`cloud/docs/**`、`cloud/ios/README.md`

- [ ] **Step 1: 删场景的测试**

```bash
cd cloud/ios
rm UITests/SceneShots.swift
git mv Tests/SceneGroupTests.swift Tests/ContactOrderTests.swift
```

`Tests/ContactOrderTests.swift`：删掉 `SceneGroupTests` 那个类（场景那部分），
`ContactOrderTests` 留下并把「按场景」的断言改成全局置顶；`testPanelKeepsRoomForNobodyAndNewContact` 保留。
其余测试文件里场景相关的用例与文案断言（`MemoryStoreTests` / `MemoryModelTests` / `ContactAddTests` /
`ContactIndexTests` / `HintTextTests` / `MemoryDetailTextTests` / `NoFullAccessStyleTests` / `OnboardingTests`）删掉或改掉。
`UITests/{ContactsShots,KeyboardShots,ReviewShots}.swift` 里场景名相关的断言去掉；
`UITests/seed/seed.py` 里种的数据去掉 `scenes` 与 `scene` 字段。

- [ ] **Step 2: 全仓扫一遍**

```bash
grep -rn "场景\|scene" cloud/ \
  | grep -v "^cloud/design/" \
  | grep -v "2026-10-05-\(scene-management\|people-and-skills\|rewrite-skills\|remove-scenes\)\.md" \
  | grep -v "scenePhase"
```

预期：**只剩注释里可改可不改的说法**；有意义的功能性引用一条都不许剩。剩下的逐条改掉。

- [ ] **Step 3: 文档**

- `cloud/docs/specs/relationship-memory.md`：场景那两行删掉（名词表里的 `Scene` 行、`场景不串` 那条已经在，
  这次把「用户自建的分组」也删掉，写成「没有场景」）。
- `cloud/docs/plans/2026-10-05-ui-implementation.md`：1d 面板、1f 工作场景、02 的 2b/2j 场景行标作废。
- `cloud/docs/specs/memory-scene-field.md`：已经是第三版（`Option<String>`），这次**只改一句**——
  把「proto 里这两个字段现在是 `String`」改成「本计划 Task 5 改成了 `Option<String>`」。
- `cloud/ios/README.md`：键盘那一节改成「左边切人（牌子 + 工具栏里列人）、右边切技能（下一份计划）」，
  删掉选择面板的描述。

- [ ] **Step 4: 模拟器走查**

```bash
cd cloud/ios
xcrun simctl boot 97A22E82-0437-4DA2-9CA1-83A8301F5C8A || true
xcodebuild -project QingjianCloud.xcodeproj -scheme QingjianCloud \
  -destination 'platform=iOS Simulator,id=97A22E82-0437-4DA2-9CA1-83A8301F5C8A' \
  -derivedDataPath build/scene-remove -only-testing:QingjianCloudUITests/ContactsShots test
```

种一份**带场景的旧数据**（`UITests/seed/seed.py` 里那份，先把 `scenes`/`scene` 字段加回去跑一次，
验清场；再跑不带场景的），截图确认：名单是平铺的、通讯录行里没有灰字场景名、
「我」页没有场景一组、键盘工具栏只有人（键盘截图按 `UITests/README.md` 的办法）。验完还原数据。

- [ ] **Step 5: 跑齐三件套，提交**

```bash
cargo fmt --manifest-path cloud/Cargo.toml --all
cargo clippy --manifest-path cloud/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path cloud/Cargo.toml -p qingjian-cloud-bridge
cd cloud/ios && xcodebuild … -only-testing:QingjianCloudTests test
git add cloud docs
git commit -m "docs(cloud): 去掉场景之后同步文档与测试"
```

---

## 自查

- 验收：Task 9 Step 2 的那条 `grep` 没有功能性引用。
- 数据：Task 3 的两条测试（清场留备份、过期备份清掉）都在。
- 行为：`qj_scope_set` 的三态有 FFI 测试（Task 4）。
- 不许动的：`scope/scoped_learner.rs` 整份没在「Files」里出现过——按人隔离（含 `learn_word`）这一版不动。
- 下一步：技能包那份计划（[改写技能包](2026-10-05-rewrite-skills.md)）在这份之后做。
