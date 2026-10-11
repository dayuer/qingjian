# C3 中英 / 数字之间自动空格：合上游 PR #372 + WeType 两条边界 + 接 iOS 实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 把上游 qingjian-team/qingjian 的 PR #372（中西文自动空格，`[general] auto_space`，缺省关）引进素笺，补上 WeType 的「上一键是退格不补」「光标所在行有 `#` 不补」两条边界，并让 iOS 键盘也有这个开关、也生效。

**Architecture:** 不自己从零写。第 1 个提交原样 cherry-pick #372（保留作者 myzz，只解冲突，Setting 标签号 57 → 74）；第 2 个提交是素笺的 Core 补丁，新逻辑放新文件 `engine/space_guard.rs`，对 #372 的文件只改挂钩行；第 3 个提交接 iOS（桥的新文件 `session/passthrough.rs`、`session/ffi.rs`，Swift 键盘与设置页）；第 4 个提交记分叉补丁。空格逻辑全在 Core，壳只决定「把什么交给 Core、把 Core 返回的文本插进去」。

**Tech Stack:** Rust（qingjian-core、qingjian-platform、qingjian-cloud-bridge）、Swift（iOS 键盘扩展与主 App，SwiftUI）、macOS IMK 壳（objc2）。

规格：`docs/notes/wetype-impl/c1-c3-cleanup-offline-space.md` 的 C3 节；审计结论在 `docs/notes/wetype-impl/README.md`「审计」一节（C3 = 合 #372 + 两条边界 + iOS）。

---

## 与规格的差异（以真实代码为准）

1. **不建 `crates/qingjian-core/src/auto_space/`。** #372 已经给出 `crates/qingjian-core/src/spacing.rs`（`needs_space(prev, next)`、`space_inner(text)`）与 `engine/spacing.rs`（`Engine::auto_space_before(&mut self, next: &str) -> bool`、私有 `previous_char`），规格里的 `CharClass` / `apply` 不做。
2. **汉字判定用 #372 的 `is_cjk`**（U+3040–30FF 假名、U+3400–4DBF、U+4E00–9FFF、U+F900–FAFF、U+20000–3134F），不是规格说的 `sentence::is_han`；效果相同或更宽，不改。
3. **数字类只有 `0–9`。** 规格提的 `% ‰ ° ℉ ′ ″ ℃` 归数字类本次不做（列在「可选后续」）。
4. **只有一个开关 `[general] auto_space`**，不拆中英 / 中数两个（审计「要先拍板」第 3 条；任务明确本次不拆）。
5. **不加 `[apps] auto_space_off` 名单。** #372 没有，WeType 也没有黑名单；缺省关已经够保守，本次不做。
6. **规格单测表里「上一键退格」一行**：#372 只处理了「上一次上屏删了一半 → 不知道上文 → 不补」；**删光了它会退回更早一条上屏的末字照样补**——用户把自动补的空格删掉、再敲同一个字，空格又被补回来。实测：Task 3 的 `deleted_auto_space_is_not_added_back` 在只有 #372 的代码上失败。所以要补一个 `after_backspace` 标志。
7. **`#` 规则只看光标所在行**（最后一个换行之后），WeType 是前文任意位置含 `#` 就不补；这是规格建议的有意偏离（一篇文档出现过一次 `#` 不该整篇失效）。与 WeType 一样，`#` 只挡前空格，候选内部的交界（`space_inner`）照补。
8. **iOS 不是「只看开关」那么简单。** 现有 iOS 键盘有三处没告诉 Core：①组句外的退格只 `sink.deleteBackward()`，Core 的退格计数不动；②符号层的数字 / 标点与 Shift 大写字母直接 `sink.commit(text)`，Core 不知道；③用户在同一个输入框里挪光标不断开上文。不补这三处，iOS 上会按过时的末字补空格。所以桥要加 `qj_pass_through` / `qj_note_backspace` / `qj_auto_space`，`set_context` 时比对前文末字（`Engine::sync_app_context`），iOS 能同步读到的宿主前文优先于上屏记录（`Engine::auto_space_before_with`，与 WeType「ContextText 优先、lastInputChar 兜底」同序）。
9. **Mac 与 iOS 的设置项标签号**：#372 的 `Setting::AutoSpace => 57` 在素笺与 `Setting::CloudProvider => 57` 撞号（上游 main 上 57 又是 `HorizontalGrid`），素笺取 **74**（素笺现有最大的顺排号是 73，99 是 `EnterCommitsCandidate`）。

## 在临时工作树里试过的结果（2026-10-11，origin/sujian = dd1ce657）

`git cherry-pick pr-372`（单提交 1345c280，父提交 c08ae57c 不在 sujian 上；**cherry-pick 比 merge 冲突小**：merge 会把 #372 分支落后上游的几百个提交一起拉进来）：

| 文件 | 情况 | 冲突原因 |
|---|---|---|
| `apps/macos/src/host/config/mod.rs` | 冲突 | 同一位置素笺有 `set_emoji_candidates` / `set_english_in_chinese` |
| `apps/macos/src/host/settings.rs` | 冲突 | 同一位置素笺有 `EnglishInChinese` / `Emoji` 两个分支 |
| `apps/macos/src/imk/controller/display.rs` | 冲突 | 素笺已改成「有没有模型都读前文」（左上下文排序），#372 改的是旧条件 |
| `apps/macos/src/preferences/pages/general.rs` | 冲突 | 素笺有 `chinese_first.setEnabled(...)` |
| `apps/macos/src/preferences/setting/mod.rs` | 冲突 + **撞号** | 素笺有 `Emoji` / `EnglishInChinese`（71 / 72）；57 已被 `CloudProvider` 用 |
| `crates/qingjian-platform/src/config/general.rs` | 冲突 | 素笺有 `english_in_chinese` / `emoji` 字段 |
| `crates/qingjian-platform/src/config/mod.rs` | 冲突 | 配置模板同一位置 |
| `docs/notes/crate-notes.md` | 冲突 | 同一行素笺加了 `english_in_chinese` / `emoji` |
| `docs/user/settings/preferences.md` | 冲突 | 「通用」「候选窗口」两行素笺改过 |
| `apps/macos/src/imk/controller/text.rs`、Core 全部 9 个文件、`english-mode.md` | 自动合并 | — |

解完冲突后：`cargo test -p qingjian-core -p qingjian-platform` 全过（core 376 个，含 #372 的 13 个新测试）；`cargo clippy -p qingjian-macos -p qingjian-core -p qingjian-platform --all-targets -- -D warnings` 干净；`cargo test -p qingjian-macos tags_round_trip` 过（用 57 会撞）。
Task 3 的 Core 补丁：core 386 个测试全过；Task 4 的桥：`--test auto_space`（4 个）与 `--test settings`（5 个）过，`clippy -p qingjian-cloud-bridge --all-targets -D warnings` 干净。
**注意基线**：用本机 `data/generated` 跑 `--test session` 时，`host_context_reorders_candidates`、`private_input_ignores_host_context` 两个在**未改动的 origin/sujian 上就失败**（数据与测试期望不一致，与本计划无关）；判据是「改前改后失败集合相同」。

## 前置 / 约定（仓库规矩，照做）

- 执行时开独立工作树：`git -C /Users/liyuqing/sproot/qingjian fetch origin && git -C /Users/liyuqing/sproot/qingjian worktree add ../qingjian-auto-space -b feat/auto-space origin/sujian`，所有改动在那里做；主检出不动。做完 PR 合了就删工作树。
- PR 只开到自己的仓库：`gh pr create --repo dayuer/qingjian --base sujian`。**永远不往上游 qingjian-team/qingjian 开 PR、推送任何东西。** `gh` 命令一律带 `--repo dayuer/qingjian`。读 #372 只用 `gh pr view/diff 372 --repo qingjian-team/qingjian`（只读）；`upstream` 远端的 push 已设成 DISABLED。
- 提交信息：Conventional Commits，`<类型>(<范围>): <中文说明>`，范围用 crate / 壳名（core、macos、cloud、docs…），正文写为什么。**不要加 Co-Authored-By 或任何署名尾行**（仓库规矩，.githooks/commit-msg 会拦格式）。例外：Task 1 是 cherry-pick，保留 #372 原作者与原第一行（英文说明，格式本身合规，钩子放行），见 Task 1。
- 代码组织：一个类型一个文件；子模块用 `foo/mod.rs` 目录；同前缀兄弟文件收进目录；导入写精确路径不用 glob；单文件 ≤ 800 行，测试超 200 行搬 `tests.rs`；新文件要 `//!` 头；注释中文、只写读代码看不出的约束；不写装饰性分隔注释。详见 docs/contributing.md 与 CLAUDE.md。
- 提交前：`cargo fmt --all`、`cargo clippy --all-targets -- -D warnings`（在 cloud/ 工作区改了就在 cloud/ 下也跑）、相关 crate 的 `cargo test -p <crate>`。
- 排序 / 整句 / 纠错改动要跑回放与评测：本计划不改排序，但改了 `commit` / `take_raw` 的出口，所以 Task 6 跑一遍 `tools/release/gate-pass.sh`，判据是改前 / 改后数字**完全一致**（`auto_space` 缺省关，`auto_space_before` 第一行就返回）。
- 用户能感知的行为变了，同一提交里改 docs/user/ 对应页（按键没变，`keys.md` 不用改）；改了 crate 实现要点同步 docs/notes/crate-notes.md。
- 改了上游已有文件（crates/、apps/ 下非 cloud/ 的文件）要在 cloud/docs/fork-patch.md 加一节记录「分叉补丁」（文件 | 改动 | 说明），原则是新代码放新文件、对上游文件只加挂钩行。
- 最后一个 Task 是：推分支、开 PR（--repo dayuer/qingjian --base sujian）、PR 描述写「怎么验证的」。不合并（只按用户明确指令合并）。
- 壳里行为变了（macOS 直通字符改由 `pass_through` 插入、iOS 退格与符号上屏改道），**PR 前必须真机验**（Task 6），只写编译与测试通过不算。

## #372 将来被上游合入时怎么办（写进 fork-patch.md，Task 5）

素笺同步上游用 merge（见 PR #29 `sync/upstream-0.2-batch1`），不是 rebase，所以不会「自动丢掉重复提交」；而且 Task 1 解过冲突、标签号改成 74，patch-id 本来就和 #372 不同。做法：

1. **保持 Task 1 的提交内容 = #372 原样 + 只解冲突 + 标签号**，不在这个提交里夹带任何素笺改动；素笺的补丁全在 Task 3 / Task 4 的独立提交里。提交正文带 `(cherry picked from commit 1345c280…)`（`-x`），同步时一眼能认出。
2. 同步时 #372 涉及的冲突块（上表 9 个文件 + `engine/spacing.rs`、`engine/commit/*`、`composing.rs` 等）**一律取上游的版本**——上游合入时可能按评审改过，以它为准。
3. 取完上游版本后把素笺的挂钩重新加回去（fork-patch.md 表里标「素笺挂钩」的行）：`engine/spacing.rs` 的 `auto_space_before` 委托到 `auto_space_before_with`、`previous_char` 改 `pub(super)`；`engine/mod.rs` 的 `after_backspace` 字段与 `mod space_guard;`；`composing.rs` 三行；`tests/spacing.rs` 的最后一个断言。上游若改了 `previous_char` / `LastCommit.tail` / `recent_commits` 的名字，跟着改 `space_guard.rs`。
4. `Setting::AutoSpace` 的标签号：上游合入时会取它自己的顺排号；同步时以上游的号为准，但必须跑 `cargo test -p qingjian-macos tags_round_trip`，撞了素笺自己的号（`CloudProvider` 等）就把**素笺侧**的号挪走。
5. 验证：`cargo test -p qingjian-core spacing space_guard`、`cd cloud && cargo test -p qingjian-cloud-bridge --test auto_space --test settings`。
6. fork-patch.md 里 Task 1 那几行（「上游 PR #372 原样」）在上游合入后删掉，只留素笺挂钩。

## File Structure

Task 1（#372 原样 cherry-pick，文件清单即 `gh pr view 372 --json files`）：

| 文件 | 职责 |
|---|---|
| `crates/qingjian-core/src/spacing.rs`（新） | `needs_space`、`space_inner`：字符交界判定 |
| `crates/qingjian-core/src/engine/spacing.rs`（新） | `Engine::auto_space_before`、`previous_char`：上文是什么、要不要补 |
| `crates/qingjian-core/src/engine/{mod,setup,composing}.rs`、`engine/commit/{mod,last}.rs`、`engine/prediction/mod.rs`、`lib.rs` | 开关字段、`set_auto_space` / `auto_space`、上屏出口补空格、`LastCommit.tail` |
| `crates/qingjian-core/src/engine/tests/spacing.rs`（新） | #372 的 10 个引擎测试 |
| `crates/qingjian-platform/src/config/{general,mod}.rs` | `[general] auto_space` 与配置模板 |
| `apps/macos/src/imk/controller/text.rs` | 直通字符走 `pass_through`（必要时连空格一起插） |
| `apps/macos/src/host/{config/mod,settings}.rs`、`preferences/{pages/general,setting/mod}.rs` | 套用配置、勾选框、写回 |
| `docs/notes/crate-notes.md`、`docs/user/input/english-mode.md`、`docs/user/settings/preferences.md` | 文档 |

Task 3（素笺 Core 补丁）：

| 文件 | 职责 |
|---|---|
| `crates/qingjian-core/src/engine/space_guard.rs`（新） | `auto_space_before_with`（退格、`#`、壳给的前文）、`sync_app_context`、`recent_tail`、`line_has_hash` |
| `crates/qingjian-core/src/engine/tests/space_guard.rs`（新） | 10 个测试 |
| `crates/qingjian-core/src/engine/spacing.rs` | 挂钩：`auto_space_before` 委托、`previous_char` 改 `pub(super)`、删掉不再用的 import |
| `crates/qingjian-core/src/engine/mod.rs` | 挂钩：`mod space_guard;`、`after_backspace` 字段与初值 |
| `crates/qingjian-core/src/engine/composing.rs` | 挂钩：`note_backspace` 置位、`remember_commit` / `push` 清零 |
| `crates/qingjian-core/src/engine/tests/{mod,spacing}.rs` | 挂钩：`mod space_guard;`；#372 的一个断言按新规则改 |

Task 4（iOS）：

| 文件 | 职责 |
|---|---|
| `cloud/crates/qingjian-cloud-bridge/src/session/passthrough.rs`（新） | `Session::pass_through` / `note_backspace` / `auto_space` |
| `cloud/crates/qingjian-cloud-bridge/src/session/ffi.rs`（新） | `qj_pass_through` / `qj_note_backspace` / `qj_auto_space`（`lib.rs` 已 809 行，不往里加） |
| `cloud/crates/qingjian-cloud-bridge/src/session/{mod,config,cloud}.rs` | `mod` 声明、套用开关、`set_context` 时 `sync_app_context` |
| `cloud/crates/qingjian-cloud-bridge/src/settings/mod.rs` | 设置页 JSON 加 `auto_space` |
| `cloud/crates/qingjian-cloud-bridge/include/qingjian_bridge.h` | 三个 C 声明 |
| `cloud/crates/qingjian-cloud-bridge/tests/auto_space.rs`（新）、`tests/settings.rs` | 集成测试 |
| `cloud/ios/Keyboard/Sources/Engine.swift` | `passThrough` / `noteBackspace` / `autoSpace` |
| `cloud/ios/Keyboard/Sources/KeyboardModel.swift` | 符号与 Shift 字母走 `passThrough`，组句外退格 `noteBackspace` |
| `cloud/ios/App/Settings/{KeyboardSettings,KeyboardSettingsView}.swift` | 设置项与开关 |
| `cloud/ios/README.md` | 「设置」一节 |

---

### Task 1: 原样引入 #372（cherry-pick，只解冲突）

**Files:** 见 File Structure「Task 1」；冲突文件按上表。

- [ ] **Step 1: 开工作树、取 #372、记基线**

```bash
git -C /Users/liyuqing/sproot/qingjian fetch origin
git -C /Users/liyuqing/sproot/qingjian worktree add ../qingjian-auto-space -b feat/auto-space origin/sujian
cd /Users/liyuqing/sproot/qingjian-auto-space
git fetch upstream pull/372/head:pr-372
git log --oneline -1 pr-372      # 期望：1345c280 feat(core): auto-insert spaces between CJK and Latin/digits
gh pr view 372 --repo qingjian-team/qingjian --json state,headRefOid --jq '.state+" "+.headRefOid'
# 期望 OPEN 1345c280a611d80dcf95077a3114a7a4471903ff；head 变了就停下来，先对比新的 diff 再继续
cargo test -p qingjian-core 2>&1 | grep "test result" | head -1   # 记下基线个数（2026-10-11 是 363 左右）
```

- [ ] **Step 2: cherry-pick**

```bash
git cherry-pick -x pr-372
```

期望：9 个文件冲突（见上表），其余自动合并。

- [ ] **Step 3: 解冲突——「两边都留」的 7 个文件**

每个冲突块都是两边在同一位置各加了几行，保留 HEAD 的行，再把 #372 的行接在后面：

`apps/macos/src/host/config/mod.rs`：
```rust
        self.engine.set_emoji_candidates(config.general.emoji);
        self.engine
            .set_english_in_chinese(config.general.english_in_chinese);
        self.engine.set_auto_space(config.general.auto_space);
```

`apps/macos/src/host/settings.rs`（注意 `Emoji` 分支自己的右花括号不要丢）：
```rust
            (Setting::EnglishInChinese, SettingValue::Bool(on)) => {
                self.settings.set_bool("general", "english_in_chinese", on);
            }
            (Setting::Emoji, SettingValue::Bool(on)) => {
                self.settings.set_bool("general", "emoji", on);
            }
            (Setting::AutoSpace, SettingValue::Bool(on)) => {
                self.settings.set_bool("general", "auto_space", on);
            }
```

`apps/macos/src/preferences/pages/general.rs`：
```rust
        self.chinese_first.setEnabled(general.english_in_chinese);
        set_checked(&self.auto_space, general.auto_space);
```

`apps/macos/src/preferences/setting/mod.rs`（三处；**标签号用 74**，57 是 `CloudProvider`）：
```rust
    /// `[general] emoji`，勾选框：候选后面配 emoji。
    Emoji,

    /// `[general] auto_space`，勾选框：中西文之间自动补空格。
    AutoSpace,
```
```rust
            Self::Emoji => 71,
            Self::EnglishInChinese => 72,
            Self::AutoSpace => 74,
```
```rust
            71 => Self::Emoji,
            72 => Self::EnglishInChinese,
            74 => Self::AutoSpace,
```

`crates/qingjian-platform/src/config/general.rs`（字段之间空一行）：
```rust
    /// 候选后面配 emoji（`kaixin` → 开心 😄）。关掉候选里就只有字词。
    pub emoji: bool,

    /// 中西文混排自动空格：汉字与半角字母 / 数字之间补一个空格（「用 Go 写了 30 行」）。缺省关；目前只有 macOS 用。
    pub auto_space: bool,
```
```rust
            english_in_chinese: true,
            emoji: true,
            auto_space: false,
```

`crates/qingjian-platform/src/config/mod.rs`：
```toml
# 候选后面配 emoji（kaixin → 开心 😄）；false 时候选里只有字词
emoji = true
# 中西文之间自动补空格：汉字与半角字母 / 数字相邻时补一个空格，「用Go写了30行」上屏成「用 Go 写了 30 行」。缺省关；只有 macOS 用
auto_space = false
```
（「只有 macOS 用」在 Task 4 改成「macOS 与 iOS」；Task 1 保持 #372 原文。）

`docs/notes/crate-notes.md`：冲突的那一行取 HEAD，在 `emoji 候选 ...（`Engine::set_emoji_candidates`）/` 后面插入 #372 的 `中西文自动空格 `auto_space`（直通字符经 `text.rs` 的 `pass_through`，开着时首键也读光标前文）/`。#372 另加的三行「中西文自动空格（`spacing` 判交界……」是自动合并进来的，保留。

- [ ] **Step 4: 解冲突——取 HEAD 的 2 个文件**

`apps/macos/src/imk/controller/display.rs`：**整块取 HEAD**。素笺已经「有没有模型都读前文、私密输入不读」（`h.engine.composition().text().chars().count() == 1 && !h.engine.is_private()`），自动空格要的首键前文已经有了，#372 的 `|| h.engine.auto_space()` 不再需要。

`docs/user/settings/preferences.md`：「通用」「候选窗口」两行取 HEAD；在「通用」那行的 `「输入拼音时中文候选排在英文词前面」、` 后面插入 `「中文与英文、数字之间自动加空格」（macOS）、`。删掉 #372 带进来的两行旧版「通用」「候选窗口」。

- [ ] **Step 5: 确认没有残留冲突标记，跑测试**

```bash
git diff --check
grep -rn '^<<<<<<<\|^>>>>>>>' apps crates docs && echo "还有冲突标记" || echo ok
cargo fmt --all --check
cargo test -p qingjian-core -p qingjian-platform 2>&1 | grep -E "test result|FAILED"
cargo test -p qingjian-macos tags_round_trip
cargo clippy -p qingjian-macos -p qingjian-core -p qingjian-platform --all-targets -- -D warnings
```

期望：全过；core 比基线多 13 个（`engine::tests::spacing::*` 10 个、`spacing::tests::*` 3 个）。

- [ ] **Step 6: 完成 cherry-pick（保留作者与原信息）**

```bash
git add -A
git cherry-pick --continue --no-edit
git log --format='%an | %s%n%b' -1
```

期望：作者 `myzz`，第一行 `feat(core): auto-insert spaces between CJK and Latin/digits`，正文末尾 `(cherry picked from commit 1345c280a611d80dcf95077a3114a7a4471903ff)`。commit-msg 钩子放行（格式合规）；pre-commit 钩子跑 fmt + clippy，等它过。

---

### Task 2: 确认 #372 没处理的退格情形（写失败测试前先看清）

**Files:** 只读 `crates/qingjian-core/src/engine/spacing.rs`、`composing.rs`。

- [ ] **Step 1: 读 `previous_char`**

`engine/spacing.rs` 里：从最新的上屏往回找，`erased == 0` 返回它的 `tail`；**删光了（`erased >= chars`）就 `continue` 到更早一条**；删了一半返回 `None`。#372 的 `partly_erased_commit_is_unknown` 只覆盖「删一半」。

结论（写进 PR 描述）：「开发」后敲 `G` 自动补成「开发 G」（补的空格记成一条直通），用户退格两次删掉 `G` 和空格，再敲 `G`——`G`、空格两条都删光，退回「开发」的末字「发」，又补空格。WeType 的 `lastKeyDownDelete` 正是挡这个。

---

### Task 3: Core 补丁——退格后不补、本行有 `#` 不补、壳给的前文优先

**Files:**
- Create: `crates/qingjian-core/src/engine/space_guard.rs`
- Create: `crates/qingjian-core/src/engine/tests/space_guard.rs`
- Modify: `crates/qingjian-core/src/engine/spacing.rs`（挂钩）
- Modify: `crates/qingjian-core/src/engine/mod.rs`（`mod space_guard;`、字段、初值）
- Modify: `crates/qingjian-core/src/engine/composing.rs`（3 行）
- Modify: `crates/qingjian-core/src/engine/tests/mod.rs`（1 行）、`tests/spacing.rs`（1 个断言）
- Modify: `docs/notes/crate-notes.md`、`docs/user/input/english-mode.md`

- [ ] **Step 1: 写失败测试**

`crates/qingjian-core/src/engine/tests/space_guard.rs`：

```rust
//! 自动空格的素笺补丁：退格后不补、本行有 `#` 不补、iOS 带前文判断与前文对不上时断开上文。

use super::*;

fn spaced_engine() -> Engine {
    let mut engine = engine();
    engine.set_auto_space(true);
    engine
}

fn commit_kaifa(engine: &mut Engine) -> String {
    engine.set_input("kaifa");
    let kaifa = engine
        .query()
        .unwrap()
        .candidates
        .items
        .into_iter()
        .find(|c| c.text == "开发")
        .unwrap();
    engine.commit(&kaifa)
}

#[test]
fn deleted_auto_space_is_not_added_back() {
    let mut engine = spaced_engine();
    commit_kaifa(&mut engine);
    assert!(engine.auto_space_before("G"));
    engine.note_passthrough('G');
    // 删掉 G 和补的空格，再敲 G：用户不要这个空格
    engine.note_backspace();
    engine.note_backspace();
    assert!(!engine.auto_space_before("G"));
    engine.note_passthrough('G');
    // 敲了别的键之后照常判断
    assert!(engine.auto_space_before("开"));
}

#[test]
fn typing_pinyin_after_backspace_clears_the_guard() {
    let mut engine = spaced_engine();
    engine.note_passthrough('G');
    engine.note_passthrough('o');
    engine.note_passthrough('!');
    engine.note_backspace();
    assert_eq!(commit_kaifa(&mut engine), " 开发");
}

#[test]
fn no_leading_space_on_a_line_with_hash() {
    let mut engine = spaced_engine();
    engine.note_passthrough('#');
    engine.note_passthrough('G');
    engine.note_passthrough('o');
    assert_eq!(commit_kaifa(&mut engine), "开发");
}

#[test]
fn hash_on_an_earlier_line_does_not_count() {
    let mut engine = spaced_engine();
    engine.note_passthrough('#');
    engine.note_passthrough('\n');
    engine.note_passthrough('G');
    assert_eq!(commit_kaifa(&mut engine), " 开发");
}

#[test]
fn hash_in_application_context_blocks_the_first_commit() {
    let mut engine = spaced_engine();
    engine.set_rescoring_context(Some("# 标题".to_owned()));
    engine.set_input("go");
    assert_eq!(engine.take_raw(), "go");
}

#[test]
fn inner_spaces_stay_on_a_hash_line() {
    let mut engine = spaced_engine();
    engine
        .set_custom_phrases(vec![crate::CustomPhrase {
            code: "dk".into(),
            text: "用docker部署".into(),
            position: 1,
            enabled: true,
        }])
        .unwrap();
    engine.note_passthrough('#');
    engine.set_input("dk");
    let phrase = engine.query().unwrap().candidates.items[0].clone();
    assert_eq!(engine.commit(&phrase), "用 docker 部署");
}

#[test]
fn application_context_wins_over_recent_commits() {
    let mut engine = spaced_engine();
    commit_kaifa(&mut engine);
    assert!(!engine.auto_space_before_with("G", Some("开发 ")));
    assert!(engine.auto_space_before_with("G", Some("说")));
    assert!(!engine.auto_space_before_with("G", Some("#话题")));
}

#[test]
fn empty_application_context_falls_back_to_recent_commits() {
    let mut engine = spaced_engine();
    commit_kaifa(&mut engine);
    assert!(engine.auto_space_before_with("3", Some("")));
}

#[test]
fn moved_cursor_breaks_the_chain() {
    let mut engine = spaced_engine();
    commit_kaifa(&mut engine);
    engine.sync_app_context("我们开发");
    assert!(engine.auto_space_before("3"));
    engine.note_passthrough('3');
    // 光标挪到了一段英文后面：最近上屏的 3 不再是光标前的字
    engine.sync_app_context("hello");
    assert!(!engine.auto_space_before("开"));
}

#[test]
fn sync_does_nothing_when_off() {
    let mut engine = engine();
    commit_kaifa(&mut engine);
    engine.sync_app_context("hello");
    engine.set_auto_space(true);
    assert!(engine.auto_space_before("3"));
}
```

`crates/qingjian-core/src/engine/tests/mod.rs`：在 `mod spacing;` 上面加一行（rustfmt 按字母序，`space_guard` 在 `spacing` 前）：

```rust
mod space_guard;
mod spacing;
```

- [ ] **Step 2: 跑，确认失败**

Run: `cargo test -p qingjian-core space_guard`
Expected: 编译失败（`auto_space_before_with`、`sync_app_context` 不存在）。先只保留 `deleted_auto_space_is_not_added_back`、`no_leading_space_on_a_line_with_hash`、`hash_in_application_context_blocks_the_first_commit` 三个（其余注释掉）再跑一次，期望这三个 **FAIL**（只有 #372 时会补空格），确认测试确实抓到了缺口；然后恢复全部。

- [ ] **Step 3: 写 `space_guard.rs`**

`crates/qingjian-core/src/engine/space_guard.rs`：

```rust
//! 自动空格的两条边界（照微信输入法，素笺分叉）：上一键是组句外的退格不补，光标所在行有 `#`（话题、Markdown 标题、注释）不补前空格。
//! 另给读得到光标前文的壳（iOS）两个入口：带前文判断、前文与上屏记录对不上时断开上文。见 cloud/docs/fork-patch.md。

use super::Engine;
use crate::spacing::needs_space;

impl Engine {
    /// [`Self::auto_space_before`] 的完整形式。`app_before` 是壳同步读到的应用里光标前文（iOS 的 `documentContextBeforeInput`）：
    /// 给了且非空就以它为准，否则按最近上屏记录与组句开始时的前文判断。补了空格记成一次直通并返回 true。
    pub fn auto_space_before_with(&mut self, next: &str, app_before: Option<&str>) -> bool {
        let Some(first) = next.chars().next() else {
            return false;
        };
        if !self.auto_space || self.after_backspace {
            return false;
        }
        let app_before = app_before.filter(|before| !before.is_empty());
        let previous = match app_before {
            Some(before) => before.chars().last(),
            None => self.previous_char(),
        };
        if !previous.is_some_and(|prev| needs_space(prev, first)) || self.line_has_hash(app_before)
        {
            return false;
        }
        self.note_passthrough(' ');
        true
    }

    /// 壳读到了光标前文（iOS 每段组句第一键）：开着自动空格、且前文末字与最近上屏记录的末字对不上
    /// （用户挪了光标、宿主改了文字）时断开上文，免得按旧位置的末字补空格。前文为空（宿主不给）不动。
    pub fn sync_app_context(&mut self, before: &str) {
        if !self.auto_space {
            return;
        }
        let Some(last) = before.chars().last() else {
            return;
        };
        if self.recent_tail().is_some_and(|tail| tail != last) {
            self.break_chain();
        }
    }

    /// 最近一条没删光的上屏的末字；删了一半或没有记录时 `None`。
    fn recent_tail(&self) -> Option<char> {
        let commit = self.recent_commits.iter().rev().find(|c| !c.is_erased())?;
        if commit.erased == 0 {
            commit.tail
        } else {
            None
        }
    }

    /// 光标所在行（最后一个换行之后）有没有 `#`。只看当前行：一篇文档里出现过一次 `#` 不该让整篇都不补。
    /// 有壳给的前文就只看它；否则先看最近上屏记录（最多 4 条），再看组句开始时的前文。
    fn line_has_hash(&self, app_before: Option<&str>) -> bool {
        if let Some(before) = app_before {
            return hash_on_last_line(before).unwrap_or(false);
        }
        for commit in self.recent_commits.iter().rev().filter(|c| !c.is_erased()) {
            if let Some(found) = hash_on_last_line(&commit.text) {
                return found;
            }
        }
        if self.composition.is_empty() {
            return false;
        }
        self.rescoring_before
            .as_deref()
            .and_then(hash_on_last_line)
            .unwrap_or(false)
    }
}

/// 从后往前找：先碰到 `#` 是 `Some(true)`，先碰到换行是 `Some(false)`，都没有是 `None`（要接着往更早的文字里找）。
fn hash_on_last_line(text: &str) -> Option<bool> {
    text.chars().rev().find_map(|c| match c {
        '#' => Some(true),
        '\n' => Some(false),
        _ => None,
    })
}
```

- [ ] **Step 4: 挂钩——`engine/spacing.rs`**

把 `auto_space_before` 的函数体换成委托（doc 注释不动），`previous_char` 改 `pub(super)`，删掉 `use crate::spacing::needs_space;`（只剩 `use super::Engine;`）：

```rust
    pub fn auto_space_before(&mut self, next: &str) -> bool {
        // 素笺分叉：退格与 `#` 两条边界在 space_guard.rs（cloud/docs/fork-patch.md）
        self.auto_space_before_with(next, None)
    }
```
```rust
    pub(super) fn previous_char(&self) -> Option<char> {
```

- [ ] **Step 5: 挂钩——`engine/mod.rs`**

```rust
mod setup;
mod space_guard;
mod spacing;
```

字段（紧跟 #372 的 `auto_space: bool,` 之后，空一行）：
```rust
    /// 中西文混排自动补空格（配置 `[general] auto_space`，缺省关）。见 [`crate::spacing`]。
    auto_space: bool,

    /// 上一个告知 Core 的键是组句外的退格（[`Self::note_backspace`]）：这时不补自动空格，用户删掉的空格不会被补回来。
    after_backspace: bool,
```

构造里（紧跟 `auto_space: false,`）：
```rust
            auto_space: false,
            after_backspace: false,
```

- [ ] **Step 6: 挂钩——`engine/composing.rs` 三行**

```rust
    pub fn note_backspace(&mut self) {
        self.after_backspace = true;
        let Some(commit) = self
```
```rust
    pub(super) fn remember_commit(&mut self, commit: LastCommit) {
        self.after_backspace = false;
        if commit.chars == 0 {
```
```rust
    pub fn push(&mut self, c: char) {
        self.after_backspace = false;
        if self.composition.is_empty() {
```

（`remember_commit` 被 `note_passthrough`、全角标点、`commit_with` 调，`push` 管组句的第一键：退格之后敲任何别的键都清掉标志。`auto_space_before_with` 自己补空格时调 `note_passthrough(' ')` 也会清，但那是在判断之后。）

- [ ] **Step 7: 改 #372 的一个断言**

`crates/qingjian-core/src/engine/tests/spacing.rs` 的 `inner_spaces_count_toward_backspaces` 末尾，删满 11 个退格后 #372 断言 `auto_space_before("3")` 为真——新规则下刚按过退格不补，改成先敲一个键再看上文：

```rust
    engine.note_backspace();
    // 刚按过退格不补（space_guard.rs），打一个字母再看上文是不是回到了「开发」
    engine.push('g');
    assert_eq!(engine.take_raw(), " g");
}
```

- [ ] **Step 8: 跑测试**

```bash
cargo fmt --all
cargo test -p qingjian-core 2>&1 | grep -E "test result|FAILED"
cargo clippy -p qingjian-core --all-targets -- -D warnings
```

Expected：core 全过（2026-10-11 试跑 386 个），clippy 干净。

- [ ] **Step 9: 文档**

`docs/notes/crate-notes.md`：在 #372 加的「中西文自动空格（`spacing` 判交界……不知道上文就不补。」那段末尾接一句：

```markdown
  素笺加了两条边界（`engine/space_guard.rs`，照微信输入法）：组句外刚按过退格（`note_backspace` 置 `after_backspace`，下一次 `remember_commit` / `push` 清）不补，
  光标所在行（最后一个换行之后）有 `#` 不补前空格（内部照补）；读得到宿主前文的壳（iOS）用 `auto_space_before_with` 以前文为准，`sync_app_context` 在前文末字与最近上屏对不上时断开上文。
```

`docs/user/input/english-mode.md`「中西文自动空格」一节，在「标点与已有空格两侧不补；……不补空格。」后面加一句：

```markdown
刚按过退格时不补（删掉的空格不会被补回来）；光标所在这一行有 `#`（话题、标题）时不在前面补空格。
```

- [ ] **Step 10: 提交**

```bash
git add crates/qingjian-core/src/engine docs/notes/crate-notes.md docs/user/input/english-mode.md
git commit -m "feat(core): 自动空格补两条边界：刚退格不补、本行有 # 不补

上游 #372 只在上一次上屏删了一半时不补；删光了会退回更早的上屏照样补，用户删掉的空格敲下一个字又回来。
照微信输入法的 lastKeyDownDelete：组句外退格后、敲别的键之前一律不补。
#（话题、Markdown 标题、注释）所在行不补前空格；只看当前行，是对微信「前文任意位置有 # 就不补」的有意收窄。
另给读得到宿主前文的壳（iOS）auto_space_before_with / sync_app_context。逻辑在新文件 space_guard.rs，对 #372 的文件只改挂钩行。"
```

---

### Task 4: 接 iOS（桥 + 键盘 + 设置页）

**Files:** 见 File Structure「Task 4」。

- [ ] **Step 1: 记桥的测试基线**

```bash
cd /Users/liyuqing/sproot/qingjian-auto-space/cloud
QINGJIAN_DATA=/Users/liyuqing/sproot/qingjian/data/generated cargo test -p qingjian-cloud-bridge 2>&1 | grep -E "^test .*FAILED|test result"
```

记下失败集合（2026-10-11 本机数据下 `--test session` 的 `host_context_reorders_candidates`、`private_input_ignores_host_context` 在未改动代码上就失败）。Task 4 结束时失败集合必须相同。

- [ ] **Step 2: 写集成测试（失败）**

`cloud/crates/qingjian-cloud-bridge/tests/auto_space.rs`：

```rust
//! 自动空格在 iOS 会话里：设置打开后上屏与组句外直通都补空格，刚退格不补，宿主前文说了算，光标挪过不按旧上屏判断。
//! 用产品数据，没有就跳过。

mod support;

use std::path::PathBuf;

use qingjian_cloud_bridge::Session;

use self::support::data_dir;

/// 学习数据目录，里面的 `config.toml` 打开了自动空格（`on = false` 时不写配置，按缺省关）。
fn user_dir(name: &str, on: bool) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("qj-auto-space-{name}-{}", std::process::id()));
    std::fs::remove_dir_all(&dir).ok();
    std::fs::create_dir_all(&dir).unwrap();
    if on {
        std::fs::write(dir.join("config.toml"), "[general]\nauto_space = true\n").unwrap();
    }
    dir
}

fn type_and_commit(session: &mut Session, pinyin: &str) -> String {
    for c in pinyin.chars() {
        session.push(c);
    }
    session.commit(0).unwrap()
}

#[test]
fn off_by_default() {
    let Some(data) = data_dir() else {
        return;
    };
    let user = user_dir("off", false);
    let mut session = Session::open(&data, Some(&user), None, None).unwrap();
    assert!(!session.auto_space());
    assert_eq!(type_and_commit(&mut session, "nihao"), "你好");
    assert_eq!(session.pass_through("3", None), "3");
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn pass_through_after_a_commit_and_after_backspace() {
    let Some(data) = data_dir() else {
        return;
    };
    let user = user_dir("commit", true);
    let mut session = Session::open(&data, Some(&user), None, None).unwrap();
    assert!(session.auto_space());
    assert_eq!(type_and_commit(&mut session, "nihao"), "你好");
    assert_eq!(session.pass_through("3", None), " 3");
    assert_eq!(session.pass_through("0", None), "0");
    // 删掉 0、3 和补的空格再敲 3：用户不要这个空格
    for _ in 0..3 {
        session.note_backspace();
    }
    assert_eq!(session.pass_through("3", None), "3");
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn host_context_decides_for_pass_through() {
    let Some(data) = data_dir() else {
        return;
    };
    let user = user_dir("host", true);
    let mut session = Session::open(&data, Some(&user), None, None).unwrap();
    assert_eq!(session.pass_through("G", Some("今天")), " G");
    assert_eq!(session.pass_through("o", Some("今天 G")), "o");
    assert_eq!(session.pass_through("3", Some("#话题")), "3");
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn moved_cursor_uses_host_context_not_stale_commits() {
    let Some(data) = data_dir() else {
        return;
    };
    let user = user_dir("moved", true);
    let mut session = Session::open(&data, Some(&user), None, None).unwrap();
    assert_eq!(type_and_commit(&mut session, "nihao"), "你好");
    // 光标挪到了一个英文词后面，下一段组句第一键读到的前文是 hello
    session.set_context("hello", "");
    assert_eq!(type_and_commit(&mut session, "shijie"), " 世界");
    std::fs::remove_dir_all(&user).ok();
}
```

`cloud/crates/qingjian-cloud-bridge/tests/settings.rs` 末尾加：

```rust

#[test]
fn auto_space_round_trips() {
    let dir = temp_dir("auto-space");
    let config = dir.join("config.toml");
    let mut settings = Settings::read(&config, &dir.join("dicts"));
    assert!(!settings.auto_space);
    settings.auto_space = true;
    settings.write(&config).unwrap();
    assert!(Settings::read(&config, &dir.join("dicts")).auto_space);
    std::fs::remove_dir_all(&dir).ok();
}
```

Run: `QINGJIAN_DATA=/Users/liyuqing/sproot/qingjian/data/generated cargo test -p qingjian-cloud-bridge --test auto_space --test settings`
Expected：编译失败（`pass_through`、`auto_space`、`Settings::auto_space` 不存在）。

- [ ] **Step 3: `session/passthrough.rs`**

```rust
//! 组句外直接交给宿主的文字（符号层的数字与标点、Shift 大写字母）与组句外的退格：告诉 Core，
//! 自动空格、退格撤销与输入日志才数得对（素笺分叉，见 cloud/docs/fork-patch.md）。

use super::Session;

impl Session {
    /// 组句外直接上屏的一段文字，返回真正要插入的：开了自动空格、且与光标前是中西文交界时前面带一个空格。
    /// `before` 是宿主光标前文（`documentContextBeforeInput`），读不到给 `None`，这时按最近上屏判断。
    pub fn pass_through(&mut self, text: &str, before: Option<&str>) -> String {
        let lead = self.engine.auto_space_before_with(text, before);
        for c in text.chars() {
            self.engine.note_passthrough(c);
        }
        let out = if lead {
            format!(" {text}")
        } else {
            text.to_owned()
        };
        self.note_committed(&out);
        self.update_hint();
        out
    }

    /// 组句外按了退格：Core 据此数退格撤销，并让紧接着的字不补自动空格（用户删掉的空格不补回来）。
    pub fn note_backspace(&mut self) {
        self.engine.note_backspace();
    }

    /// 开了中西文自动空格；关着时 Swift 不必为它读宿主前文。
    pub fn auto_space(&self) -> bool {
        self.engine.auto_space()
    }
}
```

- [ ] **Step 4: `session/ffi.rs`（`lib.rs` 已 809 行，不往里加）**

```rust
//! 自动空格与组句外退格的 C 接口（素笺分叉），与 `include/qingjian_bridge.h` 一一对应。会话只在主线程上用，全部折掉 panic。

use std::ffi::c_char;
use std::ptr;

use super::Session;
use crate::{owned, path_arg, with};

/// 组句外直接上屏的文字，返回要插入的文字（开了自动空格时可能前面带一个空格）；`text` 为空指针返回空指针。
///
/// # Safety
/// `session` 来自 `qj_session_open` 且未释放；`text`、`before` 为空或有效 UTF-8 C 字符串。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qj_pass_through(
    session: *mut Session,
    text: *const c_char,
    before: *const c_char,
) -> *mut c_char {
    let Some(text) = (unsafe { path_arg(text) }) else {
        return ptr::null_mut();
    };
    let before = unsafe { path_arg(before) };
    with(session, ptr::null_mut(), |s| {
        owned(&s.pass_through(text, before))
    })
}

/// 组句外按了退格。
///
/// # Safety
/// `session` 来自 `qj_session_open` 且未释放。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qj_note_backspace(session: *mut Session) {
    with(session, (), Session::note_backspace);
}

/// 开了中西文自动空格。
///
/// # Safety
/// 同 [`qj_note_backspace`]。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qj_auto_space(session: *mut Session) -> bool {
    with(session, false, |s| s.auto_space())
}
```

`session/mod.rs` 的 `mod` 声明：
```rust
mod cloud;
mod config;
mod ffi;
mod memory;
mod model;
mod passthrough;
mod rescore;
```

- [ ] **Step 5: 套用开关、`set_context` 时比对前文**

`session/config.rs` 的 `apply`，紧跟 `set_english_in_chinese(...)`：
```rust
        self.engine
            .set_english_in_chinese(config.general.english_in_chinese);
        // 中西文自动空格（上游 PR #372 + 素笺的退格 / `#` 边界）；组句外的直通由 Swift 走 `qj_pass_through`
        self.engine.set_auto_space(config.general.auto_space);
```

`session/cloud.rs` 的 `set_context`：
```rust
    pub fn set_context(&mut self, before: &str, after: &str) {
        let before = before.strip_suffix(self.preedit.as_str()).unwrap_or(before);
        // 光标挪过（前文末字与最近上屏对不上）就断开上文，自动空格不按旧位置判断
        self.engine.sync_app_context(before);
        self.context = Some(SurroundingText {
```

- [ ] **Step 6: 设置页 JSON**

`src/settings/mod.rs`，`Settings` 结构体里 `learning` 之后：
```rust
    pub learning: bool,

    /// 中文与英文、数字之间自动加空格（`[general] auto_space`，与 Mac 同一个开关）。旧的 JSON 没有这一项按关。
    #[serde(default)]
    pub auto_space: bool,
```
`read` 里 `learning: config.general.learning,` 之后：
```rust
            auto_space: config.general.auto_space,
```
`write` 里 `set_bool("general", "learning", settings.learning)?;` 之后：
```rust
        set_bool("general", "auto_space", settings.auto_space)?;
```

- [ ] **Step 7: C 头文件**

`include/qingjian_bridge.h`，`void qj_note_passthrough(...)` 之后：
```c
// 组句外直接上屏的文字（符号层的数字与标点、Shift 大写字母），返回要插入的文字（开了自动空格时可能前面带一个空格）；
// before 是宿主光标前文，读不到传 NULL。
char *qj_pass_through(QjSession *session, const char *text, const char *before);
// 组句外按了退格（删的是宿主里已上屏的字）。
void qj_note_backspace(QjSession *session);
// 开了中西文自动空格（[general] auto_space）。
bool qj_auto_space(QjSession *session);
```

- [ ] **Step 8: 跑桥的测试**

```bash
cd /Users/liyuqing/sproot/qingjian-auto-space/cloud
cargo fmt --all
QINGJIAN_DATA=/Users/liyuqing/sproot/qingjian/data/generated QINGJIAN_REQUIRE_DATA=1 cargo test -p qingjian-cloud-bridge --test auto_space --test settings
QINGJIAN_DATA=/Users/liyuqing/sproot/qingjian/data/generated cargo test -p qingjian-cloud-bridge 2>&1 | grep -E "^test .*FAILED|test result"
cargo clippy -p qingjian-cloud-bridge --all-targets -- -D warnings
git status --short Cargo.lock   # 期望没变
```

Expected：`auto_space` 4 个、`settings` 5 个通过；全量的失败集合与 Step 1 基线相同；clippy 干净。

- [ ] **Step 9: Swift——`Engine.swift`**

`cloud/ios/Keyboard/Sources/Engine.swift`，`notePassthrough(_:)` 之后：

```swift
    /// 组句外直接上屏的文字（符号层的数字与标点、Shift 大写字母），返回要插入的（开了自动空格时可能前面带空格）。
    /// `before` 是宿主光标前文，读不到给 nil。
    func passThrough(_ text: String, before: String?) -> String {
        let raw = text.withCString { t in
            Self.withOptionalCString(before) { qj_pass_through(session, t, $0) }
        }
        return take(raw) ?? text
    }

    /// 组句外按了退格：退格撤销与自动空格要知道。
    func noteBackspace() { qj_note_backspace(session) }

    /// 开了中西文自动空格（`[general] auto_space`）。
    var autoSpace: Bool { qj_auto_space(session) }
```

- [ ] **Step 10: Swift——`KeyboardModel.swift`**

`typeLetter` 的 Shift 分支：
```swift
        // Shift 字母是临时打英文：组到一半的拼音先原样上屏
        if upper {
            let before = composing ? nil : hostBefore()
            if composing { sink.commit(engine.takeRaw()) }
            sink.commit(engine.passThrough(letter.uppercased(), before: before))
            refresh()
            return
        }
```

`typeSymbol`：
```swift
    private func typeSymbol(_ text: String) {
        let before = composing ? nil : hostBefore()
        commitFirst()
        sink.commit(engine?.passThrough(text, before: before) ?? text)
        if Self.returnsToLetters.contains(text) { layer = .letters }
    }
```

`backspace`：
```swift
    private func backspace() {
        if composing, let engine {
            engine.backspace()
            refresh()
        } else {
            engine?.noteBackspace()
            sink.deleteBackward()
        }
    }
```

新增（放在 `typeSymbol` 之后）：
```swift
    /// 自动空格要的宿主光标前文：开着才读（每次读是一次跨进程往返）。组句中刚上屏的字宿主未必已经反映出来，
    /// 那时传 nil，交给 Core 自己记的上屏。
    private func hostBefore() -> String? {
        guard engine?.autoSpace == true else { return nil }
        return sink.contextBefore
    }
```

（`KeyboardModel.swift` 已 989 行，超过 800 行的约定；本次只加约 12 行，不顺手拆，拆分另立项。）

- [ ] **Step 11: Swift——设置页**

`cloud/ios/App/Settings/KeyboardSettings.swift`，`learning` 之后：
```swift
    var learning: Bool

    /// 中文与英文、数字之间自动加空格，与 Mac 同一个开关。
    var autoSpace: Bool
```

`cloud/ios/App/Settings/KeyboardSettingsView.swift`，第三个 Section 里「学习输入习惯」之后：
```swift
            Toggle("学习输入习惯", isOn: binding(\.learning))
                .tint(ColorUsage.appToggle.role.color)
            Toggle("中文与英文、数字之间加空格", isOn: binding(\.autoSpace))
                .tint(ColorUsage.appToggle.role.color)
```
该 Section 的 footer 改为：
```swift
            Text("关掉学习后不再记新词与词频，已学到的保留。加空格：「用Go写了30行」上屏成「用 Go 写了 30 行」，刚按过退格或这一行有 # 时不补。云联想经素笺云 让大模型补候选与整句，和 Mac 是同一个开关。")
```
文件头注释「拼音方案、模糊音、繁体、全角标点、学习、领域词库……」里在「学习」后加「、自动空格」。

- [ ] **Step 12: 编 iOS**

```bash
cd /Users/liyuqing/sproot/qingjian-auto-space
cp -R /Users/liyuqing/sproot/qingjian/data/generated.shipped data/   # 新工作树要补（见 iOS 构建备忘）
cd cloud/ios
scripts/build-bridge.sh
xcodegen generate
xcodebuild -project QingjianCloud.xcodeproj -scheme App -destination 'generic/platform=iOS Simulator' build 2>&1 | tail -5
```

Expected：`** BUILD SUCCEEDED **`。scheme 名以 `xcodebuild -list -project QingjianCloud.xcodeproj` 为准。

- [ ] **Step 13: 文档**

`cloud/ios/README.md`「设置」一节「键盘设置」那条：`学习开关、` 后面加 `中英 / 数字之间自动空格（`[general] auto_space`，与 Mac 同一个开关）、`。

`crates/qingjian-platform/src/config/mod.rs` 模板注释末尾 `缺省关；只有 macOS 用` → `缺省关；macOS 与 iOS 用`；`crates/qingjian-platform/src/config/general.rs` 字段注释 `目前只有 macOS 用` → `macOS 与 iOS 用`。

`docs/user/input/english-mode.md`「中西文自动空格」第一句改为：
```markdown
macOS 可勾选「偏好设置 → 通用 → 中文与英文、数字之间自动加空格」，iOS 在素笺 App 的「键盘设置 → 中文与英文、数字之间加空格」（都缺省关，同一个设置，开了同步时两边互通）。
```
`docs/user/settings/preferences.md`：`「中文与英文、数字之间自动加空格」（macOS）` 保持（这一页是 Mac / Windows 偏好设置）。

- [ ] **Step 14: 提交**

```bash
cd /Users/liyuqing/sproot/qingjian-auto-space
git add cloud/crates/qingjian-cloud-bridge cloud/ios/Keyboard/Sources/Engine.swift cloud/ios/Keyboard/Sources/KeyboardModel.swift \
        cloud/ios/App/Settings/KeyboardSettings.swift cloud/ios/App/Settings/KeyboardSettingsView.swift cloud/ios/README.md \
        crates/qingjian-platform/src/config docs/user/input/english-mode.md
git commit -m "feat(cloud): iOS 键盘接上中西文自动空格

设置页加开关，写同一份 config.toml 的 [general] auto_space。
符号层的数字 / 标点与 Shift 大写字母原来直接插入宿主、Core 不知道，改走 qj_pass_through；组句外退格原来只删宿主的字，改为先告诉 Core（qj_note_backspace）。
后者顺带让 iOS 也有 Mac 早就有的退格撤销学习（删掉刚上屏的词再重打选别的，那次学习退回）。
iOS 能同步读到宿主前文：直通时以它为准，组句第一键读到的前文与最近上屏对不上（挪过光标）就断开上文，不按旧位置补空格。"
```

---

### Task 5: 记分叉补丁

**Files:**
- Modify: `cloud/docs/fork-patch.md`（在「## 合并上游时」之前加一节；「## 合并上游时」加一条）

- [ ] **Step 1: 加一节**

```markdown
### 中西文自动空格（上游 PR #372 原样 + 素笺两条边界 + iOS，2026-10-11）

上游 #372（作者 myzz，OPEN）原样 cherry-pick 进来（提交带 `cherry picked from commit 1345c280…`），只解了冲突、Setting 标签号 57 → 74（素笺 57 是 `CloudProvider`）。
素笺在它之上补了微信输入法的两条边界（刚退格不补、本行有 `#` 不补），并接上 iOS。

| 文件 | 改动 | 说明 |
|---|---|---|
| `crates/qingjian-core/src/spacing.rs`、`engine/spacing.rs`、`engine/tests/spacing.rs` | 新文件（#372 原样） | 交界判定、上文判断、#372 的测试 |
| `crates/qingjian-core/src/engine/{mod,setup,composing}.rs`、`engine/commit/{mod,last}.rs`、`engine/prediction/mod.rs`、`lib.rs` | #372 原样 | 开关字段、上屏出口补空格、`LastCommit.tail` |
| `crates/qingjian-platform/src/config/{general,mod}.rs` | #372 原样（注释改成 macOS 与 iOS） | `[general] auto_space` |
| `apps/macos/src/imk/controller/text.rs`、`host/{config/mod,settings}.rs`、`preferences/{pages/general,setting/mod}.rs` | #372 原样，标签号 74 | 直通走 `pass_through`、勾选框 |
| `crates/qingjian-core/src/engine/space_guard.rs` | 新文件（素笺） | `auto_space_before_with`、`sync_app_context`、退格与 `#` 边界 |
| `crates/qingjian-core/src/engine/tests/space_guard.rs` | 新文件（素笺） | 10 个测试 |
| `crates/qingjian-core/src/engine/spacing.rs` | 素笺挂钩：改 3 行 | `auto_space_before` 委托到 `auto_space_before_with`；`previous_char` 改 `pub(super)` |
| `crates/qingjian-core/src/engine/mod.rs` | 素笺挂钩：加 5 行 | `mod space_guard;`、`after_backspace` 字段与初值 |
| `crates/qingjian-core/src/engine/composing.rs` | 素笺挂钩：加 3 行 | `note_backspace` 置位，`remember_commit` / `push` 清零 |
| `crates/qingjian-core/src/engine/tests/{mod,spacing}.rs` | 素笺挂钩：加 1 行、改 1 个断言 | 刚退格不补之后，#372 的 `inner_spaces_count_toward_backspaces` 末尾改成先敲一个键再看 |
| `docs/notes/crate-notes.md`、`docs/user/input/english-mode.md`、`docs/user/settings/preferences.md` | #372 原样 + 素笺加 1～2 句 | 边界与 iOS 入口 |
| `cloud/crates/qingjian-cloud-bridge/src/session/{passthrough,ffi}.rs`、`tests/auto_space.rs` | 新文件（cloud/） | iOS 直通、退格、开关的 C 接口 |

iOS 键盘的行为变化：组句外退格现在会告诉 Core，iOS 因此也有了 Mac 的退格撤销学习。

不做（可选后续）：拆成中英 / 中数两个开关；`% ‰ ° ℉ ′ ″ ℃` 归数字类；尾部空格（要发方向键 / Forward Delete）；
iOS 云整句（`Entry::Sentence` → `accept_prediction`）上屏不补空格；Windows / Linux 接线。
```

- [ ] **Step 2: 「## 合并上游时」加一条**

```markdown
4. 上游合入 #372（或改过的版本）时：#372 那些文件的冲突块一律取上游的版本；再把上表标「素笺挂钩」的几行加回去（上游改了 `previous_char` / `LastCommit.tail` / `recent_commits` 的名字就跟着改 `space_guard.rs`）；
   `Setting::AutoSpace` 用上游的标签号，`cargo test -p qingjian-macos tags_round_trip` 撞了就挪素笺自己的号；
   跑 `cargo test -p qingjian-core spacing space_guard` 与 `cd cloud && cargo test -p qingjian-cloud-bridge --test auto_space --test settings`；
   最后把上表「#372 原样」的那几行删掉，只留素笺的。
```

- [ ] **Step 3: 提交**

```bash
git add cloud/docs/fork-patch.md
git commit -m "docs(cloud): 分叉补丁记上自动空格（#372 原样 + 素笺边界 + iOS）

写清哪些行是 #372 原样、哪些是素笺挂钩，上游合入 #372 后同步时按这张表取上游版本、再补挂钩。"
```

---

### Task 6: 全量检查、回放、真机验证

- [ ] **Step 1: 全量检查**

```bash
cd /Users/liyuqing/sproot/qingjian-auto-space
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo test 2>&1 | grep -E "test result|FAILED"
(cd cloud && cargo fmt --all --check && cargo clippy --all-targets -- -D warnings)
cargo check -p qingjian-windows-server --target x86_64-pc-windows-gnu 2>&1 | tail -2   # GeneralConfig 加了字段，Windows 只要能编；package 名以 apps/windows/server/Cargo.toml 为准
```

- [ ] **Step 2: 回放与评测（判据：与改前逐项一致）**

```bash
# 改前（origin/sujian）跑一遍存档；在主检出跑会动 data/generated/GATE_PASSED，所以在本工作树先 checkout 基线
git stash -u 2>/dev/null; git checkout --detach origin/sujian
tools/release/gate-pass.sh; cp /tmp/gate-eval-none.txt /tmp/gate-eval-none.base; cp /tmp/gate-replay-none.txt /tmp/gate-replay-none.base
cp /tmp/gate-eval-fusion.txt /tmp/gate-eval-fusion.base; cp /tmp/gate-replay-fusion.txt /tmp/gate-replay-fusion.base
git checkout feat/auto-space
tools/release/gate-pass.sh
for f in eval-none replay-none eval-fusion replay-fusion; do diff /tmp/gate-$f.base /tmp/gate-$f.txt && echo "$f 一致"; done
```

Expected：四份都「一致」。`auto_space` 缺省关，`commit` / `take_raw` 里的 `auto_space_before` 第一行就返回 false，数字不该有任何变化；有变化就是 bug，停下查。需要的 `data/eval/` 与通变模型从主检出拷（`QINGJIAN_P2C_MODEL_DIR` 可指目录）。

- [ ] **Step 3: macOS 真机（TextEdit）**

```bash
apps/macos/scripts/bundle.sh --install
printf '\n[general]\nauto_space = true\n' >> ~/Library/Application\ Support/Qingjian/config.toml   # 路径以 docs/user 的配置文件页为准；已有 [general] 就改那一节
```

在 TextEdit 新文档、切到青简，逐条操作并读回文本（可用 `osascript -e 'tell application "System Events" to keystroke "kaifa "'` 发键，`osascript -e 'tell application "TextEdit" to get text of document 1'` 读回）：

| 操作 | 期望 |
|---|---|
| `kaifa` ␣ → `Go`（Shift+G、o） | `开发 Go` |
| `shengyu` ␣ → `30` → `tian` ␣ | `剩余 30 天` |
| `kaifa` ␣ → `G` → 退格两次 → `G` | `开发G`（删掉的空格不回来） |
| 行首 `#`、`Go` → `kaifa` ␣ | `#Go开发` |
| 换行后 `Go` → `kaifa` ␣ | `Go 开发` |
| 「偏好设置 → 通用」取消勾选，重复第一条 | `开发Go` |

- [ ] **Step 4: iOS 真机**

```bash
cd cloud/ios && scripts/install-device.sh
```

素笺 App → 键盘设置 → 打开「中文与英文、数字之间加空格」。在备忘录里：

| 操作 | 期望 |
|---|---|
| `kaifa` 选「开发」→ 符号层 `3` | `开发 3` |
| 再敲 `0` | `开发 30` |
| 退格删到「开发」→ 符号层 `3` | `开发3` |
| Shift `G` | 紧接中文时 ` G` 带空格 |
| 新一行 `#` → `kaifa` 选「开发」 | `#开发` |
| 光标点到一个英文词后面 → `shijie` 选「世界」 | `hello 世界` |
| 设置里关掉，重复第一条 | `开发3` |

两端的系统版本、机型、应用与结果记下来，写进 PR 描述。

---

### Task 7: 推分支、开 PR（不合并）

- [ ] **Step 1: 推送**

```bash
git push -u origin feat/auto-space
```

（`pre-push` 钩子跑全 workspace 测试，等它过。）

- [ ] **Step 2: 开 PR**

```bash
gh pr create --repo dayuer/qingjian --base sujian --head feat/auto-space \
  --title "feat: 中英 / 数字之间自动空格（合上游 #372 + 退格 / # 边界 + iOS）" \
  --body "$(cat <<'EOF'
## 改了什么
- 原样 cherry-pick 上游 qingjian-team/qingjian#372（作者 myzz，`[general] auto_space`，缺省关；Core + macOS），只解冲突，Setting 标签号 57 → 74。
- 素笺补丁（Core，`engine/space_guard.rs`）：组句外刚按过退格不补（#372 删光一次上屏后会退回更早的上屏照样补，用户删掉的空格又回来）；光标所在行有 `#` 不补前空格。
- iOS：设置页开关；符号层与 Shift 字母走 `qj_pass_through`、组句外退格走 `qj_note_backspace`；宿主前文优先、挪过光标断开上文。iOS 因此也有了退格撤销学习。
- 分叉补丁记在 cloud/docs/fork-patch.md，含上游合入 #372 后的同步办法。

## 怎么验证的
- `cargo test`（core 新增 #372 的 13 个 + 素笺 10 个）、桥 `--test auto_space` 4 个、`--test settings` 新增 1 个；clippy / fmt 干净。
- `tools/release/gate-pass.sh` 改前改后四份结果逐项一致（缺省关）。
- macOS <系统版本>，TextEdit：<Task 6 Step 3 的表与结果>。
- iOS <系统版本 / 机型>，备忘录：<Task 6 Step 4 的表与结果>。

## 不做（可选后续）
拆中英 / 中数两个开关；`% ‰ ° ℃` 归数字类；尾部空格；iOS 云整句上屏补空格；Windows / Linux 接线。
EOF
)"
```

- [ ] **Step 3: 收尾**

不合并，等用户明确说「合 #N」。PR 合了之后：`git -C /Users/liyuqing/sproot/qingjian worktree remove ../qingjian-auto-space`，`git -C /Users/liyuqing/sproot/qingjian branch -D pr-372`。

---

## 可选后续（本次不做）

1. 拆成 `auto_space_english` / `auto_space_digit` 两个开关（WeType 是两个，数字缺省继承英文）。
2. `% ‰ ° ℉ ′ ″ ℃` 归数字类（`50%的人` → `50% 的人`）：改 #372 的 `needs_space`，等上游定了再跟。
3. 尾部空格：后文首字与上屏末字是交界时补在尾部并把光标左移一格（iOS `adjustTextPosition(byCharacterOffset: -1)`、Mac 发 ←），后面已有多余空格时删掉。
4. iOS 云整句（`Entry::Sentence` → `Engine::accept_prediction`）上屏不经 `auto_space_before`，与 Mac 的整句补全同样不补。
5. Windows / Linux 壳接 `set_auto_space` 与直通路径。
6. `KeyboardModel.swift` 已 989 行，按职责拆文件。

## Self-Review

- 规格覆盖：合 #372（Task 1）；退格边界（Task 2 确认缺口、Task 3 测试 + 实现）；`#` 边界（Task 3）；iOS bridge / 键盘 / 设置项（Task 4）；文档（Task 3 Step 9、Task 4 Step 13、Task 5）；重复提交的同步办法（「#372 将来被上游合入时怎么办」+ Task 5 Step 2）；双开关与 `% ° ℃` 列在可选后续。
- 无占位：所有代码步骤给了完整代码；PR 描述里的 `<系统版本>` 等是执行时填的真机记录，不是代码占位。
- 名字一致：`auto_space_before_with(next, app_before)`、`sync_app_context(before)`、`after_backspace`、`Session::pass_through(text, before)` / `note_backspace` / `auto_space`、`qj_pass_through` / `qj_note_backspace` / `qj_auto_space`、Swift `passThrough(_:before:)` / `noteBackspace()` / `autoSpace`、`KeyboardSettings.autoSpace` ↔ JSON `auto_space` ↔ `Settings.auto_space`，各 Task 一致。Rust 部分 2026-10-11 在临时工作树编译并跑过测试；Swift 部分对照真实文件写，未编译（Task 4 Step 12 编）。
