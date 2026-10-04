# 上下文预测实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.
> 本文件是审计会话给出的**任务大纲**。执行前由实施会话用 writing-plans 把每个任务展开成逐步的代码与命令（写在本文件各任务下），展开后先发审计会话「素笺输入法」审一遍再动手。

**Goal:** 候选按宿主前文排序（油箱 / 又想），加本地 Tab 续写；只在 `sujian` 分支。

**Architecture:** Core 加最小挂钩（前文 → Context、choice 改加分、词级重排入口），新逻辑放新文件；神经部分在 `qingjian-neural` 加续写接口；macOS 壳加载知微并接续写显示；iOS 只接非模型部分。每处上游文件改动记进 `cloud/docs/fork-patch.md`。

**Tech Stack:** Rust（qingjian-core、qingjian-lm、qingjian-neural、apps/cli、apps/macos、cloud bridge）。

**Spec:** `cloud/docs/specs/2026-10-04-context-prediction-design.md`。

---

## Task 1：评测集与评测命令（先做，后面每步都靠它）

**Files:** Create `data/eval/context-pairs.tsv`（≥300 对）、`apps/cli/src/eval/context.rs`、`apps/cli/src/eval/continuation.rs`；Modify `apps/cli/src/args.rs`、`apps/cli/src/eval/mod.rs`。

- `--eval-context <tsv> [--no-context]`：逐行建新会话，`set_rescoring_context(前文)`，查询拼音，判词级首选是否等于期望；输出命中率与逐条失败表（`--eval-details`）。
- `--eval-continuation <文本文件>`：每 20 字取一个位置，前文 = 之前 64 字，取下 2–8 字为真值，报告代理精度（续写与真值共同前缀 ≥2 字的比例）、平均续写长度、p50 / p90 时延。
- 先跑出现状基线写进本文件（「基线」小节），后续每个任务都更新这张表。
- 验收：两条命令在当前代码上跑通、数字稳定（连跑两次一致）。

## Task 2：宿主前文进词级排序

**Files:** Create `crates/qingjian-core/src/engine/query/left_context.rs`（前文末尾 8 字最大匹配切词 → `Context`）；Modify `query/phonetic.rs`（链为空且有前文时用它）、`sentence/viterbi.rs:357`（首词 Context）、`engine/mod.rs`（mod 声明）。

- 链非空时完全保持旧行为；前文为空时完全保持旧行为。
- 单测：前文「汽车」时 `youxiang` 的 `transition_log_prob(汽车, 油箱)` 参与计分（用测试 LM）；链非空时前文不起作用；标点结尾的前文视为句首。
- 验收：`--eval-context` 提升；`--eval-text`、`--replay` 不降超过 0.5 个百分点。记 fork-patch.md 三行。

## Task 3：choice 改为加分

**Files:** Modify `crates/qingjian-core/src/engine/ranking/scored.rs:76-87`；Create `ranking/choice_bonus.rs`（`β` 常数与函数）。

- 用 `--replay`（作者本人日志）扫 β ∈ {0.5, 1, 2, 4}，取 `--replay` 词首选不降且 `--eval-context` 最好的那个。
- 单测：选过 1 次的词在强上文下可以被超过；选过 ≥5 次的词在弱上文下仍第一。
- 记 fork-patch.md。

## Task 4：知微词级重排

**Files:** Create `crates/qingjian-core/src/engine/rescoring/word_rescore.rs`；Modify `rescoring/mod.rs`（新任务种类）、macOS 壳的重排调度（停键 80 ms 那条通道）。

- 对第一页前 6 个词级候选与整句首选打 `CharScorer(前文, 候选)`；`λ_w` 用 `--eval-context` 扫 {0.3, 0.5, 0.8}。
- 结果晚到：与现有整句重排一样「到了再查一次重画」，不阻塞按键。
- 单测（用小测试模型或打桩的 scorer）：重排只动第一页前 6 个；scorer 失败时保持静态顺序。
- 时延：按键同步部分 p99 不变（用现有逐键计时 `apps/cli` 计时命令对比）。

## Task 5：模型加载 `scorers = "both"`

**Files:** Modify `crates/qingjian-platform/src/config/model.rs`（新键与缺省 `"both"`）、`apps/macos/src/host/model/mod.rs`（同时加载通变与知微）、`docs/user/` 不改（不进上游用户文档），素笺的说明写进 `cloud/docs/design.md`。

- 内存：在 Mac 上实测加载前后常驻内存，写进本文件。
- 验收：`"tongbian"` 时行为与现状逐字一致（`--eval-text` 结果相同）。

## Task 6：续写接口 `continue_text`

**Files:** Create `crates/qingjian-neural/src/continuation.rs`；Modify `crates/qingjian-neural/src/lib.rs`。

- 贪心解码，遇中文标点、换行或 `max_chars` 停；返回 `(文字, 平均对数概率)`；复用 `PrefixCache`。
- 单测：遇标点停；`max_chars` 生效；空前文返回 `None`。
- `--eval-continuation` 跑出 τ 与时延，选 τ 使代理精度 ≥60%。

## Task 7：macOS 显示与 Tab 接受

**Files:** Create `crates/qingjian-core/src/engine/prediction/local_continuation.rs`（把续写包装成与云端整句补全同一形状的结果）；Modify macOS 壳的 display / command（本地先到先显示，云端只在本地没有时显示）。

- 接受后切词记个人 n-gram（同云端补全）。
- 真机验收（按仓库约定必须）：在 TextEdit 与一个聊天应用里各试 10 句，记录显示率与接受率，写进本文件；`osascript` 往 TextEdit 发键的端到端脚本附上。

## Task 8：iOS 接非模型部分

**Files:** Modify `cloud/crates/qingjian-cloud-bridge/src/session/cloud.rs`（`set_context` 同时喂给 Engine 的前文入口）。

- 验收：桥测试里前文「汽车」时 `youxiang` 首选为「油箱」（用产品数据，没有数据就跳过）。

## 提交与审计

- 每个任务一个或几个提交（`feat(core): …` / `feat(macos): …` / `feat(cloud): …`），每处上游文件改动在同一提交里记 fork-patch.md。
- 每个任务完成后把提交哈希与三项评测数字发给审计会话；任何一项评测下降超过门槛先停下告诉审计会话。
