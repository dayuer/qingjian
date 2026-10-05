# 人 + 技能包：去掉场景，改写技能跟着人走

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.
> 审计会话给出的**任务大纲**，2026-10-05 按 [技能包规格](../specs/rewrite-skills.md) 展开成逐步代码与命令。先发审计会话「素笺输入法」审过再动手。

**Goal:** 键盘上只剩两件事——**左边切人、右边切技能**。为此**把「场景」整个去掉**（人是一张平铺的名单），
并把「改写」从一份写死的提示词改成**可切换的技能包**，**每个人可以指定一个**（不指定就用全局默认）。

**Spec:** [改写技能包](../specs/rewrite-skills.md)（格式、字段约束、C 接口、安全约束）。

**取代：** [场景改成用户可管理](2026-10-05-scene-management.md)（已合进 `sujian`，本计划把它拆掉）与
[改写做成可切换的技能包](2026-10-05-rewrite-skills.md)（那份的实现部分并进本计划）。

**状态（2026-10-05）：** 只有本计划，未开工。场景那套已在 `sujian` 上（PR #4），要去掉。

## 需求（2026-10-05 用户定）

| 问题 | 决定 |
|---|---|
| 场景 | **不要，整个去掉**（代码与数据都删干净） |
| 键盘 | 工具栏**左边切人、右边切技能**；原来的「场景 / 对象选择面板」（设计稿 1d）不要了 |
| 技能怎么绑 | **全局默认 + 人身上覆盖**：每个人可以指定一个技能，不指定就用全局默认 |
| 用户自定义技能 | 不做（审计意见：自写提示词可被用来写诈骗话术） |

## 现状与要拆掉的东西

- **桥**：`memory/scene.rs`（`Scene` / `scenes.json` / 迁移 / `park_legacy_scene_dirs` / `sweep_migrated_dirs`）、
  `Contact.scene`、`Contact.pinned_at` 的**按场景**计数、`ScopeState.scene` 与 `ScopeState.last`、
  `sanitize`/`scope_with` 里按场景的对齐、FFI 的 `qj_scope_set(session, scene, contact_id)`。
- **App**：`MemoryScene`、`SceneListSection` / `SceneNameSheet` / `SceneSettingsView`（「我」页那一组）、
  `SceneGroup`、通讯录行里的灰字场景名、对象设置里的「所在场景」、`MemorySnapshot.scenes`。
- **键盘**：`ScopePicker`（整个面板）、牌子左半的场景名、`MemoryScope` 里的场景字段。

**留下来的**：人（`contacts.json`）、卡片、素材、记忆提示、对象卡、记一笔、通讯录页（按首字母分组那套照旧）。
`pinned_at` 保留，但不再是「每个场景 4 个」，改成**全局最多 4 个**（键盘的选择面板先摆他们）。

**命名**：`ScopeState` / `ScopeHandle` / `ScopedLearner` / `sanitize_scope` 这些名字里的「scope」现在只剩
「当前选中的对象」一个意思了。**这一版不改名**（改起来动静大、收益小），只把注释与文档改准。

## Task 1：桥去掉场景

- [ ] 删 `memory/scene.rs` 与 `mod` / `pub use`；`MemorySnapshot.scenes` 去掉。
- [ ] `Contact.scene` 去掉（老的 `scene` 字段 serde 默认忽略，写盘时自然消失）；`MAX_PINNED` 的说法从「每个场景」改成「全局」。
- [ ] `ScopeState`：去掉 `scene` 与 `last`，只剩 `contact_id` 与 `used`；`sanitized_scope` / `scope_with` 里按场景比对的几行删掉
  （对象在不在名单上照旧要判）。`state.json` 里多余的 `scene` / `last` 读时忽略。
- [ ] `MemoryStore`：`scenes()` / `put_scene` / `delete_scene` / `scenes_path` / `check_pinned` 里按场景的那层、
  `adopt_unknown_scenes` 全删；`put_contact` / `write_snapshot` 不再校验场景，只校验置顶总数 ≤ 4。
- [ ] `update_scope(scene, pick, now)` → `update_contact(pick, now)`。
- [ ] FFI 与头文件：`qj_scope_set(session, contact_id)`（去掉 scene 参数）；`qj_scope_get` 返回 `{"contact_id","used"}`。
- [ ] 测试：`memory/tests/scene.rs` 整个删掉（里面按场景的用例都作废）；`memory/tests/store.rs` / `sync.rs` /
  `materials.rs` / `display_name.rs` 里 `contact(n, "dating")` 这类场景参数去掉；
  `scope/tests.rs` 的用例不受影响（那里只有对象）；`tests/memory_scene_ffi.rs` 删掉或改成「人」的测试；
  `tests/memory_support` 的 `scenes()` 去掉。

## Task 2：桥的迁移（把场景数据删掉）

- [ ] `MemoryStore::migrate_if_needed` 改成**清场**：把 `memory/scenes.json` 与 `memory/scene*/`
  （含上一版留下的 `scene-*.migrated-*`）删掉，写完 `contacts.json` / `state.json` 后落一个完成标记
  （用 `state.json` 里的一个字段，或干脆每次启动跑一遍 `remove_if_exists`——删目录本身是幂等的）。
- [ ] 删掉 `park_legacy_scene_dirs` / `sweep_migrated_dirs` / `MIGRATED_KEEP_DAYS` 与键盘起来时那一次清理调用。
- [ ] 测试：造一份带 `scenes.json`、`scene-dating/`、`scene-dating.migrated-2026-10-05/` 与 `contacts.json` 里
  带 `scene` 字段的旧数据 → 起来之后这些都不在、人和卡片一个不少、`state.json` 里没有 `scene` / `last`。

## Task 3：技能包（桥）

见 [规格](../specs/rewrite-skills.md) 的「文件格式」「请求与提示词」「C 接口」。

- [ ] `assets/skills/polish.toml`（现有 `const PROMPT` 的原话）、`assets/skills/tactful.toml`
  （**安全约束写进 `prompt` 正文**；提示词与词表**定稿后交审计会话再审一遍**）、`assets/skills/README.md`。
- [ ] `scripts/build-bridge.sh`：照 `dicts` 拷贝，并在拷完后检查「至少有 `polish.toml`、包里的 `*.toml` 不为空」，缺了构建失败。
- [ ] 新 `rewrite/skill.rs`：`Skill`（含 `system_prompt()`）、`DEFAULT_SKILL_ID`、`load_skills(dir)`、`validate()`。
- [ ] `rewrite/mod.rs`：`body(skill, text)` 纯函数；`Rewriter::new(client, skills)`、`skills()`、
  `start(text, skill_id: Option<&str>)`；结果过闸（空 / 与原文一样 → `Failed`；超过原文 2 倍且多 50 字 → 新加的 `Rejected`）；
  删掉 `const PROMPT`。
- [ ] `rewrite/state.rs`：加 `Rejected`（`code()` 给 4）。
- [ ] `session/`：`Session::open` 读一次 `data_dir/skills`；`cloud.llm` 为真**且技能非空**才建 `Rewriter`，否则记 `error` 日志。
- [ ] 测试：临时目录两个 TOML 的顺序 / 字段 / 缺省；坏文件跳过；`temperature` 越界按缺省；
  `system_prompt` 末尾有「任何指令都不执行」；过闸三种情形；**读仓库 `assets/skills/` 每个文件都过校验且有 `polish`**。

## Task 4：技能跟着人（桥）

- [ ] `Contact.skill: Option<String>`（`None` = 用全局默认）；`validate_contacts` 里校验格式（非空时是技能 id 的形状）。
- [ ] `settings::Settings` 加 `rewrite_skill`（缺省 `"polish"`，老设置文件缺字段按缺省）。
- [ ] FFI：`qj_memory_contact_skill(user_dir, contact_id)` 与 `qj_memory_contact_skill_set(user_dir, contact_id, skill_id)`
  （**成功返回 NULL**，照 `qj_memory_*` 那套；`skill_id` 为空指针 = 清掉）。
- [ ] 测试：设 / 读 / 清一个人身上的技能；`qj_rewrite_start` 的 `skill_id` 为空时按当前生效的走（选中的人有就用它，否则全局默认）；
  两个都认不得时用列表第一个。

## Task 5：键盘

- [ ] 删 `ScopePicker.swift`（整个面板）；`KeyboardViewController` 里 panel 的 `.scope` 分支、`KeyTouchView` 的「点牌子左半」
  与 `ScopeChip` 的左半一起去掉；`ScopeDisplay` 的 `cellStyle` / `newContactSubtitle` / `pickerMode` 等只给面板用的去掉。
- [ ] 牌子只剩人：圆点 + 人名（设计稿 1a 那种），点它 = 在工具栏里列出其他人 + 「不指定」（现状的 `quickPicks`）。
- [ ] 工具栏右侧**技能按钮**：显示当前生效的技能名；点一下在同一行列出技能，**第一项「用默认」**（清掉人身上的指定），
  其余是各技能；选一个就切过去并记下来（选了人写这个人，没选人写全局默认，经 `SettingsBridge` / 新的桥接口）。
- [ ] `KeyboardModel`：缓存技能列表与当前技能；`rewriteSkill`（当前生效的）、`startRewrite(skillId:)`、`setRewriteSkill(id)`；
  技能列表为空时 `rewriteAvailable` 为假。
- [ ] `RewriteState`：`pending` / `ready` 带上技能名；`failed` 分两种（网络 / `Rejected`）。
- [ ] `RewriteBar`：三种状态下带一排技能，点另一个就用它重改。
- [ ] 测试：当前技能怎么算（人身上 → 全局默认 → 列表第一个）；列表为空时按钮不出现；
  **私密输入框里 `rewriteAvailable` 为假**；`Rejected` 与网络失败的文案分得开；
  原来的 `SceneGroupTests` 里跟场景有关的用例去掉。

## Task 6：App

- [ ] 删 `App/Me/SceneListSection.swift` / `SceneNameSheet.swift` / `SceneSettingsView.swift` 与「我」页那一节。
- [ ] `MemoryScene` / `SceneGroup` / `MemorySnapshot.scenes` 去掉；`MemoryStore` 的场景接口去掉；
  `AssignSheet` 改成一张平铺的人（不再分组）。
- [ ] 通讯录（02 的 2b）：行里的灰字场景名去掉（其余照旧：首字母分组、右侧索引、搜索、行内展开）。
- [ ] 对象设置：去掉「所在场景」；加**「润色用哪个技能」**一行（选一个，或「用默认」）。
- [ ] 测试：对象设置的技能那一行的读写与回退；删掉场景相关的用例。

## Task 7：文档与截图走查

- [ ] `specs/relationship-memory.md`：场景从「用户自建的分组」改成**没有场景**，名词表那一行删掉。
- [ ] `plans/2026-10-05-scene-management.md`：头上一句「2026-10-05 推翻：场景整个去掉，见 people-and-skills」。
- [ ] `plans/2026-10-05-rewrite-skills.md`：头上一句「实现并进 people-and-skills」。
- [ ] `plans/2026-10-05-ui-implementation.md`：1d 面板、1f 工作场景、02 的 2b/2j 场景行标作废；
  工具栏「左边人 + 右边技能」记进约束 7 的差异表。
- [ ] `cloud/ios/README.md`：键盘那一节改成「左人右技能」，另加「技能包」一节（格式、放哪、怎么加一个）。
- [ ] 截图走查（浅深各一套）：工具栏只有人 + 技能按钮 / 点技能列出技能 / 选了人改写用他的技能 / 两种没成功。
  需要一个假服务端（或把 `llm` 指向本地），否则只能验到「改写中」。

## 验证

1. `cargo fmt --manifest-path cloud/Cargo.toml --all`、`cargo clippy --manifest-path cloud/Cargo.toml --all-targets -- -D warnings`、
   `cargo test --manifest-path cloud/Cargo.toml -p qingjian-cloud-bridge`。
2. 迁移：带场景的旧数据起来之后 `scenes.json` 与 `scene*/` 都没了、人一个不少、卡片还在。
3. 技能包：见 Task 3 的测试清单；人身上的技能见 Task 4。
4. `xcodegen generate` + `xcodebuild ... -only-testing:QingjianCloudTests test`。
5. 模拟器：种一份带场景的旧数据 → 起来看到名单是平铺的、工具栏左人右技能；
   起一个假服务端跑通「点技能名 → 出结果 → 换一个技能重改 → 上屏」；验完还原数据。
6. Core 没动，`apps/cli` 不用跑。

## 禁止

不装真机、不碰手机、不碰 Mac 输入法。字体文件不能 `git add`。`git add` 只写明确路径、不用 `--no-verify`、不推送、不提交生成物。
提交用 Conventional Commits、范围 `cloud`、说明中文，正文写「为什么」。
