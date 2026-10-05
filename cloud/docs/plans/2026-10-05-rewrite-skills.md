# 改写做成可切换的技能包（润色 / 高情商）

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.
> 审计会话给出的**任务大纲**，2026-10-05 按 [技能包规格](../specs/rewrite-skills.md) 展开成逐步代码与命令。先发审计会话「素笺输入法」审过再动手。

**Goal:** 把「改写」从一份硬编码提示词改成**随包的数据文件**（技能包：名字 + 说明 + 提示词 + 一组词），
默认两个（**润色**、**高情商**），**用户只能选、不能自己写**（2026-10-05 用户定），键盘上**点技能名直接开始、结果条上可换**。

**Spec:** [改写技能包](../specs/rewrite-skills.md)（格式、字段约束、C 接口、安全约束、与「人设包」的关系）。

**依赖与顺序：** 与[场景改成用户可管理](2026-10-05-scene-management.md)互不耦合，那件已合进 `sujian`（PR #4）。本计划自己能让键盘一起编译。

**状态（2026-10-05）：** 只有计划，未开工。

## 背景：现在长什么样

- 只有**一个写死的改写模式**：`qingjian-cloud-bridge/src/rewrite/mod.rs` 里的
  `const PROMPT`（「你是中文写作助手。把用户给的这段文字改得更通顺自然……」）、`const MODEL`、`temperature: 0.3`、
  `reasoning_effort: "none"`，请求体在私有的 `rewrite(client, text)` 里拼。
- `Rewriter::start(&self, text)` **没有模式参数**；FFI `qj_rewrite_start(session, text)` 同样；
  Swift 的 `RewriteState`（`idle` / `pending(original)` / `ready(original, result)` / `failed`）不带「用了哪个技能」。
- 没有 `[rewrite]` 配置节；`cloud.toml` 里只有一个布尔 `llm`（「用服务器大模型：润色 + 云联想」）。
- 工具栏按钮文案写死「改写」（`IdleBar.swift`），点一下就用那唯一的模式开始。
- **改写现在一个测试都没有**（Rust 与 Swift 都没有）；`qingjian-cloud-client/tests/support` 有假服务端设施可复用。
- 设计稿 `01 键盘.dc.html` 只画了一颗静态的「改写」文字按钮，**从没画点击后的样子**，更没有风格或技能选择。

## 定下来的（2026-10-05 用户答）

| 问题 | 决定 |
|---|---|
| 包里装什么 | **提示词 + 一组词**（话术词库，随请求发给模型） |
| 增删 | 随包的可增删（加文件即可）；**这一版不做用户自定义**，只能在随包的技能里选（审计意见：自写提示词可被用来写诈骗话术） |
| 键盘上怎么切 | **点技能名直接开始，结果条上可换** |
| 旧的改写 | **润色就是其中一个技能** |

## Task 1：技能包文件与桥

- [ ] `assets/skills/polish.toml`：把现有 `const PROMPT` 的原话搬进去，`phrases = []`，`id = "polish"`、`order = 10`
  （文件内容见 [规格](../specs/rewrite-skills.md)）。
- [ ] `assets/skills/tactful.toml`：`id = "tactful"`、`name = "高情商"`、`order = 20`。
  提示词与词表**先起草一版**，按规格里「安全」那一节的约束写（只改语气与措辞、不改事实、不编造承诺、
  不替人隐瞒、不替用户表态、长度相当）；**定稿后交审计会话再审一遍**才算完成。
- [ ] `assets/skills/README.md`：格式、放哪、怎么加一个、谁写的（提示词是自己写的，无第三方内容）。
- [ ] `scripts/build-bridge.sh`：照 `dicts` 的做法，把 `assets/skills/*.toml` 拷进 `Keyboard/Data/skills/`
  （`mkdir -p` + 逐个 `cmp -s || cp`），并保证目录为空时也不报错。
- [ ] 新 `rewrite/skill.rs`（一个类型一个文件，`Skill` 与加载、校验都在这里）：
  - `pub struct Skill { id, name, summary, prompt, phrases, temperature, order }`，`Deserialize`；
    `temperature` 缺省 0.3、`order` 缺省 0；
  - `pub fn system_prompt(&self) -> String`：`prompt`，有 `phrases` 时再接「尽量自然地用上这些说法：…（不合适就不用）。」
  - `pub const DEFAULT_SKILL_ID: &str = "polish";`
  - `pub fn load_skills(dir: &Path) -> Vec<Skill>`：读 `*.toml`，不合规的**跳过并记日志**，按 `order` 再名字排；
  - `pub fn validate(skill: &Skill) -> Result<(), &'static str>`：id 是安全 slug（小写字母/数字/`-`/`_`，≤ 32）、
    名字非空 ≤ 12 字、提示词非空 ≤ 2000 字、词 ≤ 20 条每条 1–12 字。
- [ ] `rewrite/mod.rs`：
  - `pub fn body(skill: &Skill, text: &str) -> Value`（**纯函数，为了能单测**，不连网）：`model` 不变，
    `messages[0]` 是 `skill.system_prompt()`，`temperature` 取技能的，`reasoning_effort` 仍固定 `none`；
  - `Rewriter::new(client, skills: Vec<Skill>)`；`pub fn skills(&self) -> &[Skill]`；
  - `pub fn start(&self, text: &str, skill_id: Option<&str>)`：按 id 找，认不得时用默认、再不行用第一个；
  - 删掉 `const PROMPT` 与私有的 `rewrite(client, text)`。
- [ ] `session/mod.rs` / `session/cloud.rs`：`Session::open` 读一次 `data_dir/skills` 存进会话；
  `connect` 里 `cloud.llm` 为真**且技能列表非空**才建 `Rewriter`（一个技能都没有时记 `error` 日志、
  不建——「没有技能」说明包没打进去，不该悄悄退回某个内置口气）。
- [ ] 测试（`rewrite/skill.rs` 里 `#[cfg(test)]` 或 `rewrite/tests.rs`）：临时目录放两个 TOML（顺序、字段、缺省值对得上）；
  坏 TOML 与不合规字段被跳过；`system_prompt` 有 / 无词表两种；`body()` 的 system message 含提示词与词表、
  `temperature` 取自技能；`load_skills` 在目录不存在时给空表。

## Task 2：C 接口与设置

- [ ] `include/qingjian_bridge.h`：
  - `char *qj_rewrite_skills(QjSession *session);` → `[{"id","name","summary"}]`，按 `order` 排；没有技能或会话无效时空指针；
  - `qj_rewrite_start` 加第三个参数 `const char *skill_id`（空指针或认不得时用默认）。
- [ ] `lib.rs` 里同步这两个 FFI；`qj_rewrite_start` 的参数按 `path_arg` 解。
- [ ] 设置加 `rewrite_skill`：`settings/mod.rs` 的 `Settings` 加 `rewrite_skill: String`（缺省 `"polish"`，
  老设置文件缺这个字段时按缺省——看现有的 `#[serde(default = ...)]` 怎么写）。
- [ ] 测试（`tests/rewrite_ffi.rs` 新建，或并进 `tests/ffi.rs`）：`qj_rewrite_skills` 的 JSON 与排序；
  会话没有技能包时空指针；设置读写带上新字段、老 JSON 缺字段按缺省。

## Task 3：键盘

- [ ] `Keyboard/Sources/RewriteState.swift`：`pending` / `ready` 带上 `skill: String`（技能名，显示用）。
- [ ] `Keyboard/Sources/Engine.swift`：`rewriteSkills` 解码 `qj_rewrite_skills`、`startRewrite(_ text:, skillId:)`。
- [ ] `Keyboard/Sources/KeyboardModel.swift`：
  - 缓存技能列表（键盘起来 / 换引擎时读一次）；
  - `var rewriteSkill: Skill?`（当前技能）与 `var rewriteSkills: [Skill]`；
  - `startRewrite(skillId:)`；`setRewriteSkill(id)` 写回设置（经 `SettingsBridge`）；
  - 当前技能 id 认不得时回退列表第一个；列表为空时 `rewriteAvailable` 为假（按钮不出现）。
- [ ] `Keyboard/Sources/IdleBar.swift`：按钮文案从写死的「改写」改成**当前技能名**。
- [ ] `Keyboard/Sources/RewriteBar.swift`：三种状态下都带一排技能（`RewriteSkillRow`，新建一个文件），
  点另一个就用它重改并记为当前；`failed` 也能换一个重试。原来只有一颗 ✕。
- [ ] 测试：技能列表解析、当前技能回退、`RewriteState` 带技能。

## Task 4：（取消）App 里写自己的技能

这一版不做（用户定）。以后要做，先补服务端「改写用途固定外层 system prompt」与桥里的技能校验（禁止词表），再开。
见 [规格](../specs/rewrite-skills.md) 的「安全」一节。

## Task 5：文档与截图走查

- [ ] `cloud/docs/specs/relationship-memory.md`：注明「人设包 Persona」与本地技能包是两回事（规格里已写清，那份里补一句指过来）。
- [ ] `cloud/docs/plans/2026-10-05-ui-implementation.md`：工具栏按钮从「改写」变成技能名、结果条上的技能排，
  记进约束 7 的差异表（设计稿画的是「改写」两个字，且从没画点击后的样子）。
- [ ] `cloud/ios/README.md` 的「润色」那一条改写成「技能包」：文件格式、放哪、怎么加一个。
- [ ] 截图走查（浅深各一套）：工具栏显示当前技能名 / 结果条上换技能重改 / 失败态重试。
  需要一个假服务端（或把 `llm` 指向本地），否则只能验到「改写中」。

## 验证

1. `cargo fmt --manifest-path cloud/Cargo.toml --all`、`cargo clippy --manifest-path cloud/Cargo.toml --all-targets -- -D warnings`、
   `cargo test --manifest-path cloud/Cargo.toml -p qingjian-cloud-bridge`。
2. 技能包：临时目录两个 TOML 的顺序与字段；坏文件与非法字段被跳过；`body()` 的 system message 含提示词与词表；
   `qj_rewrite_skills` 的 JSON；设置新字段的读写与老文件回退。
3. `xcodegen generate` + `xcodebuild ... -only-testing:QingjianCloudTests test`。
4. 模拟器：起一个假服务端跑通「点技能名 → 出结果 → 换个技能重改 → 上屏」；验完还原数据。
5. Core 没动，`apps/cli` 不用跑。

## 禁止

不装真机、不碰手机、不碰 Mac 输入法。字体文件不能 `git add`。`git add` 只写明确路径、不用 `--no-verify`、不推送、不提交生成物。
提交用 Conventional Commits、范围 `cloud`、说明中文，正文写「为什么」。
