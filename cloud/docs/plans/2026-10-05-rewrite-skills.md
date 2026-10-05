# 改写做成可切换的技能包（润色 / 高情商）

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.
> 审计会话给出的**任务大纲**。执行前用 writing-plans 展开成逐步代码与命令，先发审计会话「素笺输入法」审过再动手。

**Goal:** 把「改写」从一份硬编码提示词改成**随包的数据文件**（技能包：名字 + 说明 + 提示词 + 一组词），
默认两个（**润色**、**高情商**），**用户只能选、不能自己写**（2026-10-05 用户定），键盘上**点技能名直接开始、结果条上可换**。

**依赖与顺序：** 与[场景改成用户可管理](2026-10-05-scene-management.md)互不耦合，但那件先做。本计划自己能让键盘一起编译。

**状态（2026-10-05）：** 只有本大纲，未开工。

## 背景：现在长什么样

- 只有**一个写死的改写模式**：`qingjian-cloud-bridge/src/rewrite/mod.rs` 里的
  `const PROMPT`（「你是中文写作助手。把用户给的这段文字改得更通顺自然……」）、`const MODEL`、`temperature: 0.3`、
  `reasoning_effort: "none"`，请求体在私有的 `rewrite(client, text)` 里拼。
- `Rewriter::start(&self, text)` **没有模式参数**；FFI `qj_rewrite_start(session, text)` 同样；
  Swift 的 `RewriteState`（`idle` / `pending(original)` / `ready(original, result)` / `failed`）也不带「用了哪个技能」这一维。
- 没有任何 `[rewrite]` 配置节；`cloud.toml` 里只有一个布尔 `llm`（「用服务器大模型：润色 + 云联想」）。
- 工具栏按钮文案写死「改写」（`IdleBar.swift`），点一下就用那唯一的模式开始。
- **改写现在一个测试都没有**（Rust 与 Swift 都没有）；`qingjian-cloud-client/tests/support` 有假服务端设施可以复用。
- 设计稿 `01 键盘.dc.html` 只画了一颗静态的「改写」文字按钮（1c / 1e / 1e-2 / 1e-3），
  **从没画点击后的样子**，更没有风格或技能选择；「人设」只作为场景行的只读副标题出现过（`02:1a`、`02:2c`、`05:2j`）。
- `cloud/docs/specs/relationship-memory.md` 里的「人设包 Persona」（话术词库 + 语气说明 + 皮肤，云端下发、带版本、
  `Engine::set_persona`）是**另一件事**，从没实现。本计划做的是**本地技能包**，不动云端。

## 定下来的（2026-10-05 用户答）

| 问题 | 决定 |
|---|---|
| 包里装什么 | **提示词 + 一组词**（话术词库，随请求发给模型） |
| 增删 | 随包的可增删（加文件即可）；**这一版不做用户自定义**，只能在随包的技能里选（2026-10-05 用户定，审计意见：自写提示词可被用来写诈骗话术） |
| 键盘上怎么切 | **点技能名直接开始，结果条上可换** |
| 旧的改写 | **润色就是其中一个技能** |

## Task 1：技能包格式与桥

- [ ] 新 `assets/skills/polish.toml`（把现有 `const PROMPT` 的原话搬进去，`phrases = []`）与 `assets/skills/tactful.toml`
  （「高情商」，提示词与一组词由维护者起草后再定稿；审计要求：只改语气与措辞，不改事实、不编造承诺、不替人隐瞒，定稿后交审计会话再审）；`assets/skills/README.md` 写格式、来源与许可（提示词是自己写的，无第三方内容）。
- [ ] `scripts/build-bridge.sh` 照 `dicts` 的做法把 `assets/skills/*.toml` 拷进 `Keyboard/Data/skills/`。
- [ ] 新 `rewrite/skill.rs`：`Skill { id, name, summary, prompt, phrases, temperature, order }`、
  `load_skills(builtin_dir) -> Vec<Skill>`（坏文件跳过并记日志，按 `order` 再名字排）、`DEFAULT_SKILL_ID = "polish"`、
  `validate_skill`（id 是安全 slug、名字非空 ≤ 12 字、提示词非空 ≤ 2000 字、词 ≤ 20 条每条 ≤ 12 字）。
- [ ] `rewrite/mod.rs`：请求体抽成纯函数 `fn body(skill: &Skill, text: &str) -> Value`（**为了能单测**）——
  system message = 技能提示词（有词表时再接一句「尽量自然地用上这些说法：…（不合适就不用）」）；
  `Rewriter::start(&self, text, skill_id: Option<&str>)`，id 认不得时回退默认。
- [ ] `session/mod.rs`：`Session::open` 读技能列表（随包的 `data_dir/skills`）交给 `Rewriter`。
- [ ] 测试：临时目录放两个 TOML → 顺序与字段对；id 认不得回退默认；坏文件跳过；`body()` 的 system message 含提示词与词表；随包技能文件的校验（空名字、超长提示词、非法 id 都拒）。

## Task 2：C 接口与设置

- [ ] FFI：
  - [ ] `char *qj_rewrite_skills(QjSession *)` → JSON `[{"id","name","summary"}]`
  - [ ] `void qj_rewrite_start(QjSession *, const char *text, const char *skill_id)`（加第三个参数，NULL = 默认）
  - [ ] `include/qingjian_bridge.h` 同步，头文件的注释写清返回约定
- [ ] 设置加 `rewrite_skill`：桥的 `settings::Settings` 与 Swift 的 `KeyboardSettings` 各加一项，缺省 `"polish"`。
- [ ] 测试：`qj_rewrite_skills` 的 JSON；设置随包读写带上新字段（老设置文件缺这个字段时按缺省）。

## Task 3：键盘

- [ ] `RewriteState` 的 `pending` / `ready` 带上用了哪个技能（显示用）。
- [ ] `KeyboardModel`：启动时读一次技能列表缓存；`startRewrite(skillId:)`；`setRewriteSkill(id)` 写回设置；
  当前技能 id 认不得时回退列表第一个。
- [ ] `IdleBar`：按钮从写死的「改写」改成**当前技能名**（默认「润色」），点一下就用它开始。
- [ ] `RewriteBar`：三种状态下都带一排技能——**改完点另一个就用它重改**（并记为当前）；`failed` 也能换个技能重试。
  原来只有一颗 ✕。
- [ ] 测试：技能列表解析；当前技能回退；`RewriteState` 带技能。

## Task 4：（取消）App 里写自己的技能

这一版不做（用户定）。以后要做，先补服务端「改写用途固定外层 system prompt」与桥里的技能校验（禁止词表），再开。

## Task 5：文档与截图走查

- [ ] `cloud/docs/specs/relationship-memory.md`：注明「人设包」这一版先做本地技能包，与云端下发的人设包是两回事（或写明分叉）。
- [ ] `cloud/docs/plans/2026-10-05-ui-implementation.md`：工具栏按钮从「改写」变成技能名、结果条上的技能排，
  记进约束 7 的差异表（设计稿画的是「改写」两个字，且从没画点击后的样子）。
- [ ] 键盘 README 加一节「技能包」：文件格式、放哪、怎么加一个。
- [ ] 截图走查（浅深各一套）：工具栏显示当前技能名 / 结果条上换技能重改 / 失败态重试。
  需要一个假服务端（或把 `llm` 指向本地），否则只能验到「改写中」。

## 验证

1. `cargo fmt --manifest-path cloud/Cargo.toml --all`、`cargo clippy --manifest-path cloud/Cargo.toml --all-targets -- -D warnings`、
   `cargo test --manifest-path cloud/Cargo.toml -p qingjian-cloud-bridge`。
2. 技能包：临时目录两个 TOML 的顺序与字段；id 认不得回退默认；坏文件跳过；`body()` 的 system message 含提示词与词表；
   `qj_rewrite_skills` 的 JSON。
3. `xcodebuild ... -only-testing:QingjianCloudTests test`。
4. 模拟器：起一个假服务端（或用 `qingjian-cloud-client/tests/support` 的假服务端思路）跑通「点技能名 → 出结果 → 换个技能重改 → 上屏」。
5. Core 没动，`apps/cli` 不用跑。
