# 场景改成用户可管理：默认只有「日常」

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.
> 审计会话给出的**任务大纲**。执行前用 writing-plans 展开成逐步代码与命令，先发审计会话「素笺输入法」审过再动手。

**Goal:** 场景从写死的三个（恋爱 / 日常 / 工作）改成**用户自己建、自己起名、可改可删**的分组；默认只有一个叫「日常」的场景；
每个场景内**人数不限**、可**自己挑 4 个置顶**、其余按**沟通情况**排序。键盘上「选择面板」不再有每场景 8 个的硬上限。

**推翻了什么：** `cloud/docs/plans/2026-10-05-ui-implementation.md` 的 **D5**（「场景固定恋爱、日常、工作三个，不提供添加」）
与设计稿 `02 记忆卡.dc.html:122` 的同一条说明；也推翻了 [2B 计划](2026-10-04-memory-2b-client.md) 里「按场景开记录」的那套设定。
两处都要在实现时改掉并记进差异表。

**状态（2026-10-05）：** 只有本大纲，未开工。

## 背景：现在长什么样

- `Scene` 是 proto 里写死的三值枚举（`qingjian-cloud-proto/src/scene.rs`），序列化成 `"daily"/"dating"/"work"`，
  同时当四种东西用：`contacts.json` 的值、`state.json` 的 key、学习目录名（`memory/scene-<名>/learning`）、FFI 的 `const char *`。
- **三个场景行为不同**：恋爱开场景学习层且 `exclusive`（写只进叠加层、不碰全局、不记个人 n-gram）；
  工作不出提示、不出日子提醒、不叠加、全中性色（`ColorUsage.role(in:)`）。日常两头都不占。
- 每个场景各自最多 8 个对象（`store.rs` 的 `check_limit`），且**对象建好后不能换场景**（`check_scenes_kept`）。
- iOS 侧 `MemoryScope` 里另有一份静态场景表：`homeOrder` / `pickerOrder` / `title(of:)` / `reminds(_:)` / `usesAccent(_:)`；
  用户可见的场景名只有一个出口 `title(of:)`。键盘的 `ScopePicker` 是键区里固定高度的 4 列 `LazyVGrid`（最多 10 格三行）。

## 定下来的（2026-10-05 用户答）

| 问题 | 决定 |
|---|---|
| 默认场景 | 只有一个，叫「日常」 |
| 场景之间的行为差异 | **取消**——每个场景一样的配置，设置页**没有开关** |
| 人数 | **不限**；每个场景自己挑 **4 个置顶**，其余按**沟通情况**排序 |
| 删场景时里面的人 | **自动挪到默认场景** |
| 已有数据 | **一律归并成一个「日常」** |
| 键盘选择面板 | 只摆置顶与最常用的几个，不提供看全部（其余在 App 里切） |
| 分区学习 | 场景不再分区；**按人隔离**（2026-10-05 用户定，审计意见）：选了人时只写这个人的对象层，不写全局、不记个人 n-gram；没选人（「不指定」）才写全局 |
| 上传同意 | **全局一个开关**，不分场景（2026-10-05 用户定，补上审计留的「未定」） |
| 工作场景的「安静」 | **不做单独区分**（2026-10-05 用户定）：场景全是用户自定义的名字，行为一模一样，没有「安静场景」这种类型 |
| 服务端素材的 `scene` 字段 | 交给服务端处理，客户端这边写了一份材料：[素材的 scene 字段](../specs/memory-scene-field.md) |

## 模型

新文件 `memory/scenes.json` = `Vec<Scene>`：

```json
[{"id": "daily", "name": "日常", "created_at": 1791043200}]
```

- **`id` 稳定不变**：老场景沿用 `daily`/`dating`/`work` 当 id（迁移后只剩 `daily`），新建的用现成的 `new_id()`（32 位小写 hex）。
  场景 id **只收小写字母与数字**（`is_scene_id`），因为它会当 JSON key；`state.last` 与老的学习目录都按 id 走，**改名不动任何文件**。
- **`name`** 用户起、可改，非空、最多 [`MAX_SCENE_NAME_CHARS`] = 8 字。
- 场景名是纯显示，**不再驱动任何行为**。

类型改动：`Contact.scene: Scene → String`、`ScopeState.scene: String`、`ScopeState.last: BTreeMap<String, String>`、
`Contact` 加 `pinned_at: Option<i64>`、`MemorySnapshot` 加 `scenes: Vec<Scene>`。

`qingjian_cloud_proto::Scene` **这轮不动**：桥里现在没有任何地方构造 `MemoryItem` / `ContactRegistration`（上传路径还没实现），
服务端契约等真做上传时再定。

## 要删掉的东西

**桥**：`scope/mod.rs` 的 `scene_name` / `scene_label` / `parse_scene` / `scene_learning_dir`；
`scope/overlay.rs` 的场景层与 `exclusive`（只留对象层）；`scope/scoped_learner.rs` 的 exclusive 三个分支
（改成按「有没有选人」分流，见 Task 2）；
`session/memory/live.rs` 的 `shows_hints()` 去掉场景判断（只留「选了人」）；
`memory/store.rs` 的 `check_limit`（每场景 8 人）与 `check_scenes_kept`（不许换场景）；`memory/mod.rs` 的 `MAX_CONTACTS`；
`memory/error.rs` 的 `ContactLimit`（换成 `PinLimit`）。

**Swift**：`MemoryScope` 的 `daily`/`dating`/`work`/`homeOrder`/`pickerOrder`/`title(of:)`/`reminds`/`usesAccent`
（这个类型只剩键盘状态 `scene`/`contactId`/`last`/`used`）；`SceneGroup` 的 `limit`/`countLabel`/`isFull`/`fullNote`/`header`；
`ColorUsage.role(in:)`（调用点改回 `role`）；`MemoryAvatar` 的 `scene` 参数；
`ScopeDisplay` 的 `hasHintRow`/`chipUsesAccent`/`accentFirstCandidate` 去掉场景判断、`maxContacts`/`newContactSubtitle`；
`MemoryFailure.contactLimit`。按场景分岔的文案（`MemoryWording.switchesNote`、`MemoryDetailText.reminderNote`）改直。

## 我替你定的（不同意就在动手前说）

1. **允许改场景**：场景变成纯分组之后「建好不能换场景」就没道理了，`ContactSettingsView` 的「所在场景」从只读改成可选，
   桥那边的 `check_scenes_kept` 删掉。
2. **至少留一个场景**，删不掉最后一个；删掉列表第一个时，剩下的第一个当默认。
3. **键盘面板只摆 8 个**（置顶的 + 按沟通排的），再多的人在键盘里选不到，只能回 App 切——这是用户选的方案，但人一多就会撞上。
4. 「我」页场景行的副标题写**人数**（设计稿那里写的是「记录中 · 人设温和 · 2 个应用」，那些东西随行为差异一起没了）。

## 代价（会发生，先说明）

- 迁移按审计意见改成：`scene-*/` 改名为 `scene-*.migrated-<日期>` 保留 30 天；三个文件 `scenes.json` 最后写作完成标记、可重跑；读到不在 `scenes.json` 里的场景 id 一律归默认场景（新旧版本并存时旧键盘可能写回 `dating`）。

- **恋爱场景单独学过的词不再生效**：`memory/scene-dating/learning/` 改名保留 30 天后清掉（对象层不受影响）。只影响已装机的。
- **设计稿被推翻**：「我」页的场景组、增删改名界面设计稿从没画过，是本计划拼的，要交 UI 审计员过一遍。

## Task 1：桥的场景模型与迁移

- [ ] 新 `memory/scene.rs`：`Scene { id, name, created_at }`、`SCENES_FILE`、`DEFAULT_SCENE_ID = "daily"`、
  `MAX_SCENE_NAME_CHARS = 8`、`is_scene_id`、`validate_scenes`（至少一个、id 合法且不重复、名字非空不超长）。
- [ ] `Contact.scene` 改成 `String`；加 `pinned_at: Option<i64>` 与 `MAX_PINNED = 4`。
- [ ] `ScopeState.scene` 改成 `String`、`last: BTreeMap<String, String>`，缺省 `DEFAULT_SCENE_ID`；`ScopeState` 的 `#[serde(default)]` 要能吃老文件。
- [ ] `memory/mod.rs` 的 `sanitized_scope` / `scope_with` 改成按字符串场景（`belongs` 的签名跟着改）；`Scene` 相关的 import 清掉。
- [ ] `MemoryStore` 加 `scenes()` / `put_scene` / `rename_scene` / `delete_scene`（把里面的人挪到默认场景）/ `scenes_path()`。
- [ ] `MemorySnapshot` 加 `scenes`；`snapshot()` 读、`write_snapshot()` 校验后写。
- [ ] `check_limit` 删掉，换成 `check_pinned`（同一场景里 `pinned_at.is_some()` 的 ≤ 4，超了报 `MemoryError::PinLimit`）；
  `check_scenes_kept` 删掉。
- [ ] **迁移**（`scenes.json` 不存在时做一次，做完不再走）：写 `scenes.json` = `[{id:"daily", name:"日常", created_at: 现在}]`；
  所有 `Contact.scene` 改成 `"daily"`、`pinned_at` 补 `None`；`state.scene` 改 `"daily"`、`state.last` 只留 `"daily"`；
  删 `memory/scene-*/`；写回三个文件。
- [ ] 测试：迁移（造三场景旧数据 → 断言只剩一个场景、人一个不少、`scene-*/` 没了、`last` 只剩一个 key）；
  置顶第 5 个报 `PinLimit`；一个场景 20 个人不报错；场景增删改名的往返与校验。

## Task 2：桥删分区学习与场景行为分支

- [ ] `scope/mod.rs`：删 `scene_name` / `scene_label` / `parse_scene` / `scene_learning_dir`；`overlay.rs` 只留对象层。
- [ ] `scoped_learner.rs`：`open(user_dir, memory_dir, contact)`；按人隔离：选了人时 `write` / `record_transition` / `unrecord_transition` 只作用于对象层（不碰全局、不记个人 n-gram，即原 `exclusive` 的行为改挂在「选了人」上）；没选人时作用于全局。读照旧是全局 + 对象层叠加。
- [ ] `handle.rs`：`switch(contact: Option<&str>)`。
- [ ] `session/`：`set_scope(scene: &str, pick)`、`memory_add_contact(..., scene: &str)`、`LiveMemory` 跟着改；
  `live.rs` 的 `shows_hints()` 只看有没有选人。
- [ ] FFI 与头文件：`qj_scope_set` 的 scene 参数语义从「三选一」改成「场景 id，认不得就用默认」；
  `qj_memory_add_contact` 的兜底从 `Scene::Daily` 改成 `DEFAULT_SCENE_ID`。
- [ ] 测试：选了人时打的词不出现在「不指定」与其他人的候选里（隔离）；没选人时写全局；换人后前一个人的词不出现。
- [ ] 测试：`scope/tests.rs` 里 `dating_writes_do_not_reach_work` / `daily_and_work_share_global` /
  `dating_does_not_write_transitions` 这类按场景分岔的用例删掉或改成「所有场景一致」；
  `memory/tests/scene.rs`、`tests/memory_scene_ffi.rs` 按新模型重写。

## Task 3：Swift 的 Shared 层

- [ ] 新 `Shared/Memory/MemoryScene.swift`（`{id, name, createdAt}`，与桥一一对应）。
- [ ] 新 `Shared/Memory/ContactOrder.swift`（纯值，单测）：`score(_:used:)` = 键盘里上次选中这个人的时间（`state.used`），
  没选过的用 `createdAt`；`ordered(_:used:limit:)` = 置顶的先（按 `pinnedAt` 升序）、其余按沟通情况降序再认识时间再名字，`limit` 截断。
- [ ] `MemoryScope` / `SceneGroup` / `ColorUsage` / `MemoryAvatar` / `ScopeDisplay` / `MemoryFailure` 按上面「要删掉的东西」清理。
- [ ] 测试：`ContactOrderTests`（置顶在前、按沟通排、截断、边界）；`SceneGroupTests` 按新模型重写；
  其它断言了 `dating/daily/work` 的用例（`MemoryModelTests`、`MemoryStoreTests`、`ContactAddTests`、`MemoryDetailTextTests`、
  `NoFullAccessStyleTests`、`ContactIndexTests`）跟着改。

## Task 4：App

- [ ] `MemoryStore` 加 `scenes` / `sceneName(of:)` / `addScene(name:)` / `renameScene(id:name:)` / `deleteScene(id:)` / `pinContact(id:pinned:)`。
- [ ] 新 `App/Me/SceneListSection.swift`（「我」页的「场景」一节：每个场景一行 + 「加一个场景」）、
  `App/Me/SceneNameSheet.swift`（新建与改名共用的输入框弹层）、`App/Me/SceneSettingsView.swift`（改名 / 删掉 / 这个场景几个人）。
- [ ] `ContactSettingsView`：「所在场景」从只读改成可改，另加「置顶（每个场景最多 4 个）」一行。
- [ ] `ContactsView`：「+」菜单遍历用户场景、行里置顶的人加小标记（设计稿没画，本计划拼的）。
- [ ] `ContactEditor` / `OnboardingView` / `MemoryContact.new` 的默认场景不再是写死的 `dating`。
- [ ] 测试：场景增删改名与删除时人挪到默认场景、最后一个场景删不掉、置顶第 5 个被拒。

## Task 5：键盘

- [ ] `KeyboardModel.people` 改用 `ContactOrder.ordered(...)`，面板取前 8、工具栏快速切人列取前 6。
- [ ] `ScopePicker` 的分段改成遍历用户场景、去场景配色分支；`ScopeChip` / `NoteComposeBar` / `NoteBar` 一并去场景分支。
- [ ] 全仓扫一遍注释里写死的「三个场景」（`ScopeDisplay`、`KeyboardModel`、`SceneGroup`、`ScopePicker` 都有）。

## Task 6：文档与截图走查

- [ ] `cloud/docs/specs/relationship-memory.md`：场景定义改成「用户自建的分组」，去掉「一套人设 + 分区学习」那半句与 `work/dating/plain` 的例子。
- [ ] `cloud/docs/plans/2026-10-05-ui-implementation.md`：改 D5、改 2j 的状态与描述、把「场景固定三个」记进约束 7 的差异表。
- [ ] `cloud/docs/plans/2026-10-04-memory-2b-client.md`：`Scene { Daily, Dating, Work }` 与「按场景开记录」的段落标注作废。
- [ ] 扫 `cloud/docs/user/` 里讲场景 / 提示 / 工作场景的页并改。
- [ ] 截图走查（浅深各一套）：「我」页场景一组 / 加 / 改名 / 删（人挪走）/ 通讯录（场景名来自用户、置顶标记）/
  键盘选择面板（置顶在前、其余按沟通）。

## 验证

1. `cargo fmt --manifest-path cloud/Cargo.toml --all`、`cargo clippy --manifest-path cloud/Cargo.toml --all-targets -- -D warnings`、
   `cargo test --manifest-path cloud/Cargo.toml -p qingjian-cloud-bridge`。
2. 迁移测试：造一份三场景的旧 `memory/`（三个场景各有人有卡），跑一次 `qj_memory_read`，
   断言只剩一个场景、名字「日常」、人一个不少且都归到它、`memory/scene-*/` 没了、`state.last` 只剩一个 key。
3. 置顶与人数：一个场景置顶第 5 个人报 `PinLimit`；删掉一个置顶后能再置顶；一个场景放 20 个人不报错。
4. `xcodebuild ... -only-testing:QingjianCloudTests test`。
5. Core 没动，`apps/cli` 不用跑。
