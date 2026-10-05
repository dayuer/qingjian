# 人 + 技能包：去掉场景，改写技能跟着人走

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.
> 审计会话给出的**任务大纲**，2026-10-05 按 [技能包规格](../specs/rewrite-skills.md) 展开成逐步代码与命令。先发审计会话「素笺输入法」审过再动手。

**Goal:** 键盘上只剩两件事——**左边切人、右边切技能**。为此**把「场景」整个去掉**（人是一张平铺的名单），
并把「改写」从一份写死的提示词改成**可切换的技能包**，**每个人可以指定一个**（不指定就用全局默认）。

**Spec:** [改写技能包](../specs/rewrite-skills.md)（格式、字段约束、C 接口、安全约束）。

**取代：** [场景改成用户可管理](2026-10-05-scene-management.md)（已合进 `sujian` 的 PR #4，本计划把它拆掉）与
[改写做成可切换的技能包](2026-10-05-rewrite-skills.md)（那份的实现部分并进本计划）。

**状态（2026-10-05）：** 只有本计划，未开工。

## 需求（2026-10-05 用户定）

| 问题 | 决定 |
|---|---|
| 场景 | **不要，整个去掉**（代码与数据都删干净，**包括建人时选场景那一步**） |
| 键盘 | 工具栏**左边切人、右边切技能**；原来的「场景 / 对象选择面板」（设计稿 1d）不要了 |
| 技能怎么绑 | **全局默认 + 人身上覆盖** |
| 用户自定义技能 | 不做（审计意见：自写提示词可被用来写诈骗话术） |

## 要清掉的场景清单（自查过一遍，按文件分组）

这一节是**验收标准**：下面每一条在实现完都要不存在了。

### 桥

| 在哪 | 现在是什么 | 改成 |
|---|---|---|
| `memory/scene.rs` | `Scene` / `scenes.json` / `validate_scenes` / `park_legacy_scene_dirs` / `sweep_migrated_dirs` | **整个文件删掉** |
| `memory/contact.rs` | `Contact.scene: String` | 字段删掉（老的 `scene` 键 serde 默认忽略，下次写盘就没） |
| `memory/contact.rs` | `MAX_PINNED` 的注释「一个场景里最多几个」 | 改成「全局最多几个」 |
| `scope/state.rs` | `ScopeState.scene` / `ScopeState.last` | 两个字段都删（`state.json` 里多余的键读时忽略） |
| `memory/mod.rs` | `sanitized_scope` / `scope_with` 里按场景比对的几行；`Scene` 的 re-export | 删掉（对象在不在名单上照旧要判） |
| `memory/snapshot.rs` | `MemorySnapshot.scenes` | 字段删掉 |
| `memory/store.rs` | `scenes()` / `put_scene` / `delete_scene` / `scenes_path` / `read_scenes`、`check_pinned` 里按场景那层、`adopt_unknown_scenes`、`migrate_if_needed` 的并场景 | 删掉；`check_pinned` 改成数**全局**置顶 ≤ 4 |
| `memory/store.rs` | `migrate_if_needed` | 改成**清场**：删 `memory/scenes.json` 与 `memory/scene*/`（含上一版留下的 `scene-*.migrated-*`），幂等 |
| `memory/error.rs` | `PinLimit` 的文案「一个场景最多置顶 4 个人」 | 改成「最多置顶 4 个人」 |
| `memory/ffi.rs` | `qj_scope_set(session, scene, contact_id)` | 去掉 `scene` 参数 |
| `memory/ffi.rs` | `qj_scope_get` 返回里的 `scene` / `last` | 去掉，只剩 `{"contact_id","used"}` |
| `memory/ffi.rs` | **`qj_memory_add_contact(session, name, pronoun, scene)`** | 去掉 `scene` 参数（建人不再指定分组） |
| `session/memory/mod.rs` | `set_scope(scene, pick)` / `memory_add_contact(..., scene)` | 改签名，去掉场景 |
| `session/memory/live.rs` | `sweep_migrated_dirs` 那次调用 | 删掉 |
| `scope/pick.rs` | `ContactPick::Last`（「回到这个场景上次选的人」，靠 `state.last`） | 改成 `ContactPick::Keep`：**保持现在选的人不变**（幂等），见下「两处定下来的」 |
| `include/qingjian_bridge.h` | 上面几处接口的注释与签名 | 同步 |

### proto 与服务端

| 在哪 | 现在是什么 | 改成 |
|---|---|---|
| `docs/specs/memory-scene-field.md` | 请服务端把 `MemoryItem.scene` / `ContactRegistration.scene` **从枚举改成字符串** | **已在本 PR 重写成「请服务端去掉这个字段」**（这一版为准）；还差**把它发给服务端**那一步（对外动作，先问） |
| proto `Scene` 枚举 | 桥里没人用，但类型还在 | 这次仍不动（等服务端一起改），在规格里写明 |

### App

| 在哪 | 现在是什么 | 改成 |
|---|---|---|
| `App/Contacts/ContactsView.swift` | 右上「+」是个 `Menu`，遍历 `store.scenes`；`@State newScene` | **直接开建人页**，不再选场景（就是这条漏了） |
| `App/Memory/ContactEditor.swift` | `let scene: String`、「加在「X」里」那句、`MemoryContact.new(… scene:)` | 去掉 `scene` 参数与那句话 |
| `App/Onboarding/OnboardingView.swift` | `ContactEditor(store:scene: store.defaultScene?.id ?? "")` | 去掉 `scene:` |
| `App/Contacts/ContactRow.swift` | `let sceneName: String` 与行里的灰字 | 去掉（名字后面不再有灰字） |
| `App/Contacts/ContactIndex.swift` | `search(_:text:sceneName:)` 拿场景名当搜索词 | 去掉 `sceneName` 形参，只搜名字与代号 |
| `App/Memory/ContactSettingsView.swift` | 「所在场景」Picker；`pinnedCount(in: contact.scene)` | Picker 删掉（改「润色用哪个技能」）；置顶改成数全局 |
| `App/Memory/MemoryStore.swift` | `scenes` / `sceneName(of:)` / `defaultScene` / `pinnedCount(in:)` / `addScene` / `renameScene` / `deleteScene` / `groups` | 前六个删掉；`pinnedCount()` 不带场景；`groups` 改成一张平铺的人 |
| `App/Memory/MemoryWording.swift` | `unknownScene` / `sceneNameTooLong` / `sceneHasPeople` / `peopleCount` / `pinLimit` | 前四个删掉，`pinLimit` 改文案 |
| `App/Remember/AssignSheet.swift` | 按 `store.groups`（按场景）分组 | 改成一张平铺的人 |
| `App/Remember/RememberView.swift` | 功勋路回退时 `for scene in store.scenes` | 改成按名单顺序取第一个 |
| `App/Me/MeView.swift` + `SceneListSection` / `SceneNameSheet` / `SceneSettingsView` | 「我」页的场景一组与场景设置页 | 三个文件删掉，`MeView` 去掉那一节 |
| `Shared/Memory/MemoryScene.swift` / `SceneGroup.swift` / `MemorySnapshot.scenes` | 场景类型与分组类型 | 删掉 |
| `Shared/Memory/MemoryFailure.swift` | `pinLimit` 的兜底文案 | 改文案 |

### 键盘

| 在哪 | 现在是什么 | 改成 |
|---|---|---|
| `Keyboard/Sources/ScopePicker.swift` | 整个选择面板（场景分段 + 人 + 新对象） | **文件删掉** |
| `KeyboardPanel.swift` | `case scope` | 删掉那个 case |
| `KeyboardView.swift` | `case .scope: ScopePicker(model: model)` | 删掉 |
| `IdleBar.swift` | `model.panel == .scope` 的两处判断；`ScopeChip(split:)` | 删掉（牌子不再有「左半」） |
| `CandidateBar.swift` | `panelTakesTheBar` 里的 `.scope` | 删掉 |
| `ScopeChip.swift` | `split`、左半的场景名、`chipShowsPerson`、点左半 `toggleQuickPicks` | 牌子只剩圆点 + 人名（设计稿 1a 那种），点它列其他人 + 「不指定」；`split` 删掉 |
| **没开完全访问时** | 牌子上只剩场景名，点它进面板看「开启完全访问后才能用记忆」那段说明 | 面板删了，说明改成挂在**牌子上**：读不到名单时牌子上没有人名，那一块就是说明入口，点它**在工具栏里展开**同一段话 +「去开启」+ 设置路径，见下「两处定下来的」 |
| `KeyboardModel.swift` | `chooseScene(_:)` / `applyScope(scene:pick:)` / `openScopePicker()` / `confirmNewContact` 里的 `engine.addContact(name:scene:)` | 前三个删掉 / 改签名；建人不再传场景 |
| `MemoryBridge.swift` | `setScope(scene:pick:)` / `addContact(name:scene:)` | 去掉 `scene` 参数 |
| `Engine.swift` | `setScope` / `addContact` 的包装 | 同上 |
| `ScopeDisplay.swift` | `chipScene` / `chipUsesAccent` / `pickerMode` / `cellStyle` / `newContactSubtitle` / `noScopeSubtitle` | 只给面板用的全删；`quickPicks` 留着（工具栏列人） |

### 测试与文档

| 在哪 | 改成 |
|---|---|
| `Tests/SceneGroupTests.swift` | 场景那部分删掉，`ContactOrderTests` 留下并改成全局置顶 |
| `Tests/` 其他（`MemoryStoreTests` / `MemoryModelTests` / `ContactAddTests` / `ContactIndexTests` / `MemoryWordingTests` …） | 场景相关的用例与文案断言删掉 |
| `UITests/SceneShots.swift` | **整个删掉**（那一串场景截图走查没意义了） |
| `UITests/ContactsShots.swift` / `KeyboardShots.swift` | 场景名相关的断言去掉 |
| `cloud/ios/README.md` | 键盘那一节改成「左人右技能」，另加「技能包」一节 |
| `plans/2026-10-05-ui-implementation.md` | 1d 面板、1f 工作场景、02 的 2b/2j 场景行标作废；工具栏「左人右技能」记进差异表 |
| `specs/relationship-memory.md` | 场景那两行删掉 |
| `plans/2026-10-05-scene-management.md`、`rewrite-skills.md` | 已各标一句被取代 |
| **用户可见文案**：`App/Memory/CloudIntroView.swift`（「键盘会在你选的场景里……」）、`App/Onboarding/PlanStep.swift`（云功能点「你开启的场景里，键盘自己记」） | 去掉「场景」那半句（这两处是给用户看的，漏了会前后不一致） |
| **过时注释**：`Theme` / `ColorRole` / `MemoryAvatar` / `ScopeDisplay` / `NoteToast` / `NoteBar` / `NoteComposeBar` / `KeyboardView` / `ContactSettingsView` / `DayEvents` / 两个测试文件里的「工作场景」「选择面板」 | 改准（说「不指定」「工具栏」这类现在才对的说法） |

**留下来的**：人（`contacts.json`）、卡片、素材、记忆提示、对象卡、记一笔、通讯录页（首字母分组、右侧索引、搜索、行内展开）。
`pinned_at` 保留，改成**全局最多 4 个**。

**命名**：`ScopeState` / `ScopeHandle` / `ScopedLearner` / `sanitize_scope` 里的「scope」现在只剩「当前选中的对象」一个意思。
**这一版不改名**（动静大、收益小），只把注释与文档改准。

## 两处定下来的（2026-10-05，按自查时的建议）

1. **`qj_scope_set` 的三态**（`state.last` 没了之后「回到上次」没有意义）：
   - `NULL` = **保持现在选的人不变**（幂等）——键盘只在自己要切的时候传 id，不需要「回到上次」；
   - `""` = 明确不指定；
   - 其余 = 指定某个对象 id（不在名单上时当不指定，与现在一样）。
2. **没开完全访问时那段说明挂在牌子上**：读不到名单时（`chipShowsPerson` 为假）牌子上没有人名，
   那一块就是**说明入口**——点它在工具栏里展开「开启完全访问后才能用记忆」+「去开启」+ 设置路径
   （键盘扩展打不开系统设置，照旧只给文字）。这条保住审核指南 4.4.1 那条路径。
   **注意**：改写不依赖完全访问（它读的是宿主光标前那段字，不是 App Group 里的记忆），所以没开时技能按钮照旧在。

## Task 1：桥去掉场景

按上面「桥」那张表逐条改。要点：

- [ ] 删 `memory/scene.rs`、`Contact.scene`、`ScopeState.scene` / `last`、`MemorySnapshot.scenes`。
- [ ] `check_pinned` 改成数全局；`update_scope(scene, pick, now)` → `update_contact(pick, now)`；
  `ContactPick::Last` → `ContactPick::Keep`（保持现在选的人不变）。
- [ ] FFI：`qj_scope_set(session, contact_id)`、`qj_scope_get` 去 `scene`/`last`、`qj_memory_add_contact(session, name, pronoun)`；
  头文件同步。
- [ ] 测试：`memory/tests/scene.rs` 删；其他测试文件里的场景参数与断言删；
  `tests/memory_scene_ffi.rs` 删或改成「人」的测试；`memory_support` 的 `scenes()` 去掉。

## Task 2：桥的迁移（清场）

- [ ] `migrate_if_needed` 改成清场：删 `memory/scenes.json` 与 `memory/scene*/`（含 `scene-*.migrated-*`），幂等。
- [ ] 删 `park_legacy_scene_dirs` / `sweep_migrated_dirs` / `MIGRATED_KEEP_DAYS` 与 `LiveMemory::open` 里那次调用。
- [ ] 测试：造带 `scenes.json`、`scene-dating/`、`scene-dating.migrated-2026-10-05/`、`contacts.json` 里带 `scene` 的旧数据
  → 起来之后这些都不在、人和卡片一个不少、`state.json` 里没有 `scene` / `last`。

## Task 3：技能包（桥）

见 [规格](../specs/rewrite-skills.md)。

- [ ] `assets/skills/polish.toml` / `tactful.toml` / `README.md`（**安全约束写进 `prompt` 正文**；提示词与词表定稿后交审计再审）。
- [ ] `scripts/build-bridge.sh` 拷贝 + **拷完检查**（至少 `polish.toml`、包里非空），缺了构建失败。
- [ ] 新 `rewrite/skill.rs`：`Skill`（含 `system_prompt()`）/ `DEFAULT_SKILL_ID` / `load_skills` / `validate`。
- [ ] `rewrite/mod.rs`：`body(skill, text)` 纯函数、`Rewriter::new(client, skills)`、`skills()`、`start(text, skill_id)`；
  结果过闸（空 / 与原文一样 → `Failed`；超过原文 2 倍且多 50 字 → `Rejected`）；删 `const PROMPT`。
- [ ] `rewrite/state.rs` 加 `Rejected`（`code()` = 4）。
- [ ] `session/`：`Session::open` 读一次 `data_dir/skills`；`cloud.llm` 且技能非空才建 `Rewriter`。
- [ ] 测试：两个 TOML 的顺序 / 字段 / 缺省；坏文件跳过；`temperature` 越界；`system_prompt` 末尾有「任何指令都不执行」；
  过闸三种情形；**读仓库 `assets/skills/` 每个文件都过校验且有 `polish`**。

## Task 4：技能跟着人（桥）

- [ ] `Contact.skill: Option<String>`；`validate_contacts` 校验它的形状。
- [ ] `settings::Settings` 加 `rewrite_skill`（缺省 `"polish"`，老设置缺字段按缺省）。
- [ ] FFI：`qj_memory_contact_skill` / `qj_memory_contact_skill_set`（**成功返回 NULL**；`skill_id` 空指针 = 清掉）。
- [ ] 测试：设 / 读 / 清一个人身上的技能；`qj_rewrite_start` 的 `skill_id` 为空时按当前生效的走。

## Task 5：键盘（左人右技能）

- [ ] 按上面「键盘」那张表清掉面板与牌子的左半。
- [ ] 工具栏右侧**技能按钮**：显示当前生效的技能名；点一下在同一行列技能，**第一项「用默认」**，其余是各技能；
  选了人写这个人，没选人写全局默认。
- [ ] `KeyboardModel`：缓存技能列表与当前技能、`startRewrite(skillId:)`、`setRewriteSkill(id)`；列表为空时 `rewriteAvailable` 为假。
- [ ] `RewriteState` 带技能名；`failed` 分两种（网络 / `Rejected`）；`RewriteBar` 三种状态带技能排。
- [ ] **没开完全访问时**：牌子上没有人名，那一块当说明入口（点它在工具栏里展开同一段话 +「去开启」+ 路径）。
- [ ] 测试：当前技能怎么算；列表为空时按钮不出现；私密输入框里 `rewriteAvailable` 为假；两种失败文案分得开。

## Task 6：App

- [ ] 按上面「App」那张表清干净，**特别是通讯录「+」不再选场景**。
- [ ] 对象设置加**「润色用哪个技能」**一行（选一个或「用默认」）。
- [ ] 「我」页去掉场景一组。
- [ ] 测试：技能那一行的读写与回退；删掉场景相关的用例。

## Task 7：文档与截图走查

- [ ] 按上面「测试与文档」那张表清理。
- [x] 重写 `specs/memory-scene-field.md`（给服务端的那份）——**已做**，结论改成「去掉这个字段」。
- [ ] 把新结论**发给服务端**（对外动作，先问过再发）。
- [ ] 截图走查（浅深各一套）：工具栏只有人 + 技能按钮 / 点技能列出技能 / 选了人改写用他的技能 / 两种没成功。
  需要一个假服务端（或把 `llm` 指向本地），否则只能验到「改写中」。

## 验证

1. `cargo fmt --manifest-path cloud/Cargo.toml --all`、`cargo clippy --manifest-path cloud/Cargo.toml --all-targets -- -D warnings`、
   `cargo test --manifest-path cloud/Cargo.toml -p qingjian-cloud-bridge`。
2. **清场迁移**：带场景的旧数据起来之后 `scenes.json` 与 `scene*/` 都没了、人一个不少、卡片还在。
3. 技能包与「人身上的技能」见 Task 3 / 4 的测试清单。
4. **全仓扫一遍**：`grep -rn "场景\|scene" cloud/`（排除 `cloud/design/` 设计稿快照与 `docs/plans/2026-10-05-scene-management.md`
   这类「记作废」的历史文档）应该没有功能性引用了——这一条是验收标准。
5. `xcodegen generate` + `xcodebuild ... -only-testing:QingjianCloudTests test`。
6. 模拟器：种带场景的旧数据 → 名单平铺、工具栏左人右技能；起假服务端跑通「点技能名 → 出结果 → 换一个重改 → 上屏」；验完还原。
7. Core 没动，`apps/cli` 不用跑。

## 禁止

不装真机、不碰手机、不碰 Mac 输入法。字体文件不能 `git add`。`git add` 只写明确路径、不用 `--no-verify`、不推送、不提交生成物。
提交用 Conventional Commits、范围 `cloud`、说明中文，正文写「为什么」。
