# 云联想请求降频（D2）实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 云联想从「每段组句停顿就发、上屏后还补发一次」降到「只在本地不够用、且比上次多出足够信息时才发」，冻结日志上实发请求降六成以上，本地候选与排序一字不变；同时给 #341 一个缺省关的「云整句排第一」开关，并把四个壳里各写一份的云词插位收回 Core。

**Architecture:** 规则分两处放：Engine 每个键都看得到、但看不到防抖，所以无状态或要查词库的门槛（R1 字母太少、R2 短全拼、R5 整句复用、R11 按需模式）在 Core `engine/prediction/`；要和「真正发出去的那次」比的门槛（作废即不发、R3 退格、R4 小增量、R6 缓存、R8 限流、R9/R10 429 锁定）在 `qingjian-predict` 新的 `Dispatcher` 里，后台线程与 CLI `--predict-sim` 共用同一个类型，模拟出来的次数就是真实规则的次数。云端结果晚到后插在哪由 Core 新目录 `engine/late/` 的纯函数决定，壳只报「画出来了」并照 `CloudPlacement` 摆。

**Tech Stack:** Rust 2024（workspace + `cloud/` 独立 workspace），`qingjian-core` / `qingjian-predict` / `qingjian-platform` / `qingjian-cli`，macOS IMK 壳，素笺 iOS 桥 `qingjian-cloud-bridge`；测试 `cargo test`，评测 `tools/release/gate-pass.sh` 与 `qingjian-cli --predict-sim`。

**规格来源（只在主检出里、未进 git）：** `/Users/liyuqing/sproot/qingjian/docs/notes/wetype-impl/d2-cloud-request-strategy.md`（R1–R11）、`.../b5-late-merge.md`（`engine/late/`）、`.../proto/d2-request-sim.py`。工作树里没有这几份文件；代码注释一律指向本计划 Task 14 新建的 `docs/notes/cloud-request-throttle.md`。

---

## 前置 / 约定（仓库规矩，照做）

- 执行时开独立工作树：`git -C /Users/liyuqing/sproot/qingjian fetch origin && git -C /Users/liyuqing/sproot/qingjian worktree add ../qingjian-cloud-throttle -b cloud-throttle/request-gates origin/sujian`，所有改动在 `/Users/liyuqing/sproot/qingjian-cloud-throttle` 里做；主检出不动。做完 PR 合了就删工作树。下文路径都相对这个工作树根目录（记作 `$WT`）。
- PR 只开到自己的仓库：`gh pr create --repo dayuer/qingjian --base sujian`。**永远不往上游 qingjian-team/qingjian 开 PR、推送任何东西。** `gh` 命令一律带 `--repo dayuer/qingjian`。
- 提交信息：Conventional Commits，`<类型>(<范围>): <中文说明>`，范围用 crate / 壳名（core、predict、platform、cli、macos、cloud、windows、docs…），正文写为什么。**不要加 Co-Authored-By 或任何署名尾行**（仓库规矩，`.githooks/commit-msg` 会拦格式）。先 `git -C $WT config core.hooksPath .githooks` 启用钩子。
- 代码组织：一个类型一个文件；子模块用 `foo/mod.rs` 目录；同前缀兄弟文件收进目录；导入写精确路径不用 glob（`#[cfg(test)] mod tests` 里的 `use super::*` 与存量文件里已有的 glob 除外）；单文件 ≤ 800 行，测试超 200 行搬 `tests.rs`；新文件要 `//!` 头；注释中文、只写读代码看不出的约束；不写装饰性分隔注释。
- 提交前：`cargo fmt --all`、`cargo clippy --all-targets -- -D warnings`（改了 `cloud/` 就在 `cloud/` 下也跑 `cargo clippy -p qingjian-cloud-bridge --all-targets -- -D warnings`）、相关 crate 的 `cargo test -p <crate>`。
- 排序 / 整句 / 纠错改动要跑回放与评测：本计划**不碰本地排序**，但 Task 15 照样跑 `tools/release/gate-pass.sh`（它内部跑 `--eval-text` 与 `--replay … --clean2`，两种配置 none / fusion），判据：五个指标与基线逐个相等（`[predict] enabled = false` 的 `tools/eval/offline.toml` 下云联想根本不参与，任何变化都说明误伤了本地路径）。
- 用户能感知的行为变了，同一 PR 里改 `docs/user/` 对应页（按键改动同时改 `keys.md`）；改了 crate 实现要点同步 `docs/notes/crate-notes.md`。PR 是 squash 合并，同一 PR 即同一提交。
- 改了上游已有文件（`crates/`、`apps/` 下非 `cloud/` 的文件）要在 `cloud/docs/fork-patch.md` 加一节「分叉补丁」（文件 | 改动 | 说明），原则是新代码放新文件、对上游文件只加挂钩行（Task 14）。
- 最后一个 Task 是：推分支、开 PR（`--repo dayuer/qingjian --base sujian`）、PR 描述写「怎么验证的」。**不合并**（只按用户明确指令合并）。
- 壳行为（macOS 插位、快捷键，iOS 插位）的改动 PR 必须真机验过（`docs/contributing.md`「外部 PR」）；Windows 壳本机编不了 TSF DLL，拆成单独分支与 PR（Task 17），没有 Windows 真机就只推分支不开 PR。

## 缺省值（审计建议的默认，用户尚未拍板；集中在 Task 2 的 `defaults.rs` 一处，改它不影响别的任务）

| 项 | 缺省 | 位置 |
|---|---|---|
| 超时晚到的云端词 | 不丢，从第 4 位起插（`LATE_FROZEN = 3`） | `crates/qingjian-core/src/engine/prediction/defaults.rs` |
| 时限内到的云端词 | 插第 2 位、最多 1 条（`LATE_GRACE_MS = 200`、`LATE_MAX_IN_GRACE = 1`） | 同上 |
| `sentence_first` | 关 | 同上 |
| 限流（令牌桶） | 关（`RATE_BURST = 0`） | 同上 |
| 按需模式 `trigger` | `auto`（`manual` 只是配置项） | `PredictionTrigger::default()` |
| `[shortcut] predict_now` | macOS `control+option+p` / Windows `ctrl+alt+p`（同一个 `KeyCombo`） | `KeyCombo::PREDICT_NOW_DEFAULT` |
| R1 / R2 / R4 | 4 / 6 / 3 | `defaults.rs` |

## 与规格的差异（以代码为准）

1. **R3 退格 / R4 小增量不在 Core，在发送线程。** 规格把 `PredictionGate` 放 Core、和「上次实际发出的」比；但四个壳每次缓冲变化都调 `request_prediction`，防抖在 `qingjian-predict/src/worker.rs`，Core 看到的「提交」绝大多数被防抖吞掉。照规格做，用户在被 R4 跳过的那一键停顿时什么都不会发（实测会把正常停顿也吞掉）。改为 `qingjian-predict` 的 `Dispatcher` 在防抖之后和「真正发出去的」比。为此 `PredictionRequest` 加 `composition`（第几段组句，Engine 每次作废加一），R3/R4 只在同一段里比；`min_growth` 只在 `PredictConfig`，不进 `PredictionPolicy`。
2. **新增「作废即不发」（规格没列）。** 现状 `Engine::cancel_prediction` 只换序号，后台线程防抖到点照样把已作废的请求发出去（上屏在 300 ms 内的每段组句都会白发一次）。`Predictor` trait 加缺省空实现的 `supersede(sequence)`，`CloudPredictor` 用一个 `AtomicU64` 告诉后台线程。
3. **R7（在途同请求不重发）不写代码。** 后台线程本来是串行的：在途时新请求排队，回包后先进缓存，排队的同键请求防抖后直接命中缓存。唯一例外是空回包不缓存（有意的，见 `worker.rs` 注释），保持。
4. **R9 拿不到 `Retry-After`。** `async-openai 0.42`（没开 `middleware` 特性）把非 2xx 转成 `OpenAIError::ApiError`，只留状态码、不留响应头；非 JSON 错误体的 429 连状态码都没有。所以固定锁 30 秒，只认 JSON 429。R9 / R10 是对服务商限流的被动响应，**始终开**（不算「限流」）；缺省关的是 R8 令牌桶。见「未决」。
5. **R5 只复用整句、严格逐字对读音、不容错。** 规格允许 `tolerance(letters)` 个不符；复用是「不再问」，宁可多问一次，所以不容错。复用时云端词丢掉（它们对应的是更短的字母串）。
6. **问字模式不受 R1 / R2 / R5 / R11 约束**（同规格），仍受旧的 2 字母下限；按需模式下问字照常自动发（问字本身就是用户主动发问）。`request_prediction_now` 也受 2 字母下限。
7. **`engine/late/` 只做云端结果（词与整句）。** B5 规格里的整句 / 词级神经重排晚到规则（Mac `model/mod.rs`、Windows / Linux `dispatch/rescore/mod.rs`、iOS `session/rescore.rs` 的「翻页 / 动过高亮不重排」守卫）不在本计划，留作后续；本计划只让重排后的重建把云端结果按原位置带过来（`CandidateLayout::carry_cloud`、iOS `cloud_at`）。B5 的 `generation` 由联想序号代替；`note_candidates_touched` 改成 `place_prediction` 的参数 `touched: Option<usize>`（壳本来就有「动过高亮」与高亮下标）；「首选是用户常用词时受保护」（IsProtectLocalFirst）要读学习记录的最后使用时间，Learner trait 没有，暂只保护固定位置（`Custom`）与快捷候选（`Shortcut`）的首选。
8. **「纯转换」判据** 用 `cover::converts`（逐字一个完整音节、字与字母同时用完），代替规格的「字数 == 音节数且 covers 无不符」，一次判定、不依赖本地切分。
9. **`late_top1_guard_ms = 150`** 从 Core 摆放整句的时刻算，不要壳上报按键时刻（壳按键到 Core `commit` 是同步的，差几毫秒）。
10. **iOS 桥覆盖 `late_frozen = 1`**：手机候选栏窄，第 4 位常在屏幕外；保持现在「紧跟首选」的位置，只是规则换成 Core 的。iOS 不报 `touched`（键盘扩展拿不到候选栏滑动）。
11. **Linux 壳不改**：`apps/linux/server` 不调 `request_prediction`，`cloud_slots` 被强制成 0（`dispatch/mod.rs:74`），没有云端词插位（grep 见 Task 14 Step 1）。
12. **偏好设置界面不加开关**（用户：只加配置项）；`docs/user/settings/preferences.md` 不改，`docs/user/cloud/index.md` 写配置项。
13. **模拟器** 是 Python 原型的 Rust 移植，但门槛用 Engine 与 `Dispatcher` 的真代码；模拟不了回包内容，所以 R5 的收益不在数里（数是上限）。

## 我在 `origin/sujian`（dd1ce657）上的核对结果

- 计划里的代码在临时工作树 `/tmp/plan-d2` 全部写过一遍：`cargo test --workspace`、`cargo clippy --workspace --all-targets -- -D warnings`、`cloud/` 下 `cargo clippy -p qingjian-cloud-bridge --all-targets -- -D warnings` 与 `cargo test -p qingjian-cloud-bridge` 全过；`apps/windows/server` 的 `dispatch` 在 macOS 上可编译可测（67 个测试过），TSF DLL 没编。
- 冻结日志 `data/eval/input-log-2026-10-04.jsonl`（1210 段可模拟组句、6386 个字母）上 `--predict-sim` 实发请求：

| 中途停顿概率 | 旧规则（近似：门槛放回 2/0/0、不作废） | 全部新规则（缺省配置） | 降幅 |
|---|---|---|---|
| 0.0 | 750 | 249 | 67% |
| 0.1 | 1082 | 335 | 69% |
| 0.2 | 1350 | 395 | 71% |

  执行时 Task 1 在真旧代码上量的数为准，与上表差几个百分点正常。

---

## File Structure

**Core `crates/qingjian-core/src/`**

| 文件 | 新 / 改 | 职责 |
|---|---|---|
| `engine/prediction/defaults.rs` | 新 | 全部新常数（R1–R10、晚到插位、整句排第一）一处改 |
| `engine/prediction/trigger.rs` | 新 | `PredictionTrigger { Auto, Manual }`（serde 小写） |
| `engine/prediction/skip_reason.rs` | 新 | `SkipReason`：Core 与发送线程共用的「没发」原因 + 中文短名 |
| `engine/prediction/gate.rs` | 新 | `PredictionGate`：组句编号、上次提交 / 回复、没发原因、强制标记、复用结果 |
| `engine/prediction/cover.rs` | 新 | `covers` / `converts`：整句读音能否盖住 / 恰好转换字母 |
| `engine/prediction/{mod,policy,predictor,request}.rs` | 改 | 门槛接线、`request_prediction_now`、`supersede`、`composition` 字段、策略新字段 |
| `engine/late/{mod,policy,outcome,placement,state,top}.rs`、`engine/late/merge/{mod,tests}.rs` | 新 | 晚到插位：`LatePolicy`、`place_words` 纯函数、`CloudPlacement`、`Engine::{note_candidates_shown, place_prediction, commit_late_top}` |
| `engine/mod.rs`、`engine/commit/mod.rs`、`lib.rs` | 改（挂钩行） | 两个字段、`mod late;`、导出、`commit` 首行挂钩 |
| `candidate/layout/candidate_layout.rs` | 改 | `place_cloud` / `carry_cloud`，`cloud_at` 与 `top` 字段 |
| `engine/tests/cloud_gate.rs` | 新 | Engine 级门槛 / 复用 / 按需 / 整句排第一测试 |
| `engine/tests/mod.rs` | 改 | `mod cloud_gate;`，旧 `EchoPredictor` 门槛放回旧值 |

**`crates/qingjian-predict/src/`**

| 文件 | 新 / 改 | 职责 |
|---|---|---|
| `dispatcher/mod.rs`、`dispatcher/tests.rs` | 新 | `Dispatcher` / `Decision`：作废、缓存、R3/R4、R8、R9/R10 |
| `rate.rs` | 新 | `RateLimiter` 令牌桶 |
| `worker.rs`、`cloud_predictor.rs`、`cache.rs`、`config.rs`、`error.rs`、`lib.rs`、`prompt.rs`（测试） | 改 | 用 Dispatcher、supersede、缓存键、配置字段、429 识别 |

**其他**

| 文件 | 新 / 改 | 职责 |
|---|---|---|
| `crates/qingjian-platform/src/config/{key_combo,shortcut,mod}.rs` | 改 | `predict_now` 快捷键、模板注释、解析测试 |
| `apps/cli/src/predict_sim/{mod,composition,predictor,report,rng}.rs` | 新 | `--predict-sim` |
| `apps/cli/src/{main,args}.rs` | 改 | 参数与分派 |
| `apps/macos/src/host/{cloud/mod,model/mod,presenting/mod,mod,init,config/mod}.rs`、`apps/macos/src/imk/controller/{mod,display}.rs` | 改 | 用 Core 插位、报画出时刻、`predict_now` |
| `cloud/crates/qingjian-cloud-bridge/src/session/{cloud,rescore,mod,config}.rs` | 改 | 用 Core 插位、`cloud_at`、`late_frozen = 1` |
| `apps/windows/server/src/dispatch/{composed/mod,rescore/mod,config,message}.rs`、`apps/windows/tsf/src/com/{key/preserved,service/mod,service/processor,service/key_sink}.rs` | 改（单独 PR） | 同 macOS |
| `docs/notes/cloud-request-throttle.md` | 新 | 规则表、出处、模拟模型、前后数字 |
| `docs/user/cloud/index.md`、`docs/user/getting-started/keys.md`、`docs/user/help/data-and-logs.md`、`docs/notes/crate-notes.md`、`cloud/docs/fork-patch.md` | 改 | 文档同步 |

---

## Task 0: 工作树

- [ ] **Step 1: 开工作树、启用钩子**

```bash
git -C /Users/liyuqing/sproot/qingjian fetch origin
git -C /Users/liyuqing/sproot/qingjian worktree add ../qingjian-cloud-throttle -b cloud-throttle/request-gates origin/sujian
export WT=/Users/liyuqing/sproot/qingjian-cloud-throttle
git -C $WT config core.hooksPath .githooks
ls $WT/data/eval/input-log-2026-10-04.jsonl 2>/dev/null || ln -s /Users/liyuqing/sproot/qingjian/data/eval $WT/data/eval
ls $WT/data/generated/dict.tsv 2>/dev/null || echo "缺产品数据：tools/release/data-fetch.sh 或从主检出拷 data/generated（gitignore）"
```

Expected：工作树在 `cloud-throttle/request-gates` 分支；`data/eval/` 与 `data/generated/` 可读（都不进 git）。CLI 默认从当前目录的 `data/generated/` 读词库，下面所有 `qingjian-cli` 命令都在 `$WT` 下跑。

---

## Task 1: 尺子——`Dispatcher` 抽出来、`--predict-sim` 量现状

不改任何行为：把后台线程「查缓存」那一段抽成 `Dispatcher`，CLI 用同一个类型模拟；顺带加 `SkipReason` 与 `Predictor::supersede`（缺省空实现，Engine 这一步还不调）。

**Files:**
- Create: `crates/qingjian-core/src/engine/prediction/skip_reason.rs`
- Modify: `crates/qingjian-core/src/engine/prediction/mod.rs`（mod 声明与 re-export）、`crates/qingjian-core/src/engine/prediction/predictor.rs`、`crates/qingjian-core/src/engine/mod.rs:50-53`、`crates/qingjian-core/src/lib.rs:34-43`
- Create: `crates/qingjian-predict/src/dispatcher/mod.rs`、`crates/qingjian-predict/src/dispatcher/tests.rs`
- Modify: `crates/qingjian-predict/src/{worker.rs,cloud_predictor.rs,lib.rs}`
- Create: `apps/cli/src/predict_sim/{mod.rs,composition.rs,predictor.rs,report.rs,rng.rs}`
- Modify: `apps/cli/src/main.rs`、`apps/cli/src/args.rs`

- [ ] **Step 1: Core 加 `SkipReason`**

新建 `crates/qingjian-core/src/engine/prediction/skip_reason.rs`：

```rust
//! 一次联想为什么没发出去：Core 的门槛（R1 / R2 / R5 / R11）与发送线程的门槛（R3 / R4 / R8 / R9、已作废）共用一套，
//! 日志与 `qingjian-cli --predict-sim` 按它分类计数。

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SkipReason {
    /// R1：字母太少。
    TooShort,

    /// R2：短而完整的全拼，本地首选够用。
    ShortFullPinyin,

    /// R11：按需模式，没按快捷键。
    Manual,

    /// R5：上次的整句补全仍盖得住新敲的字母，直接复用，不发。
    Reused,

    /// 发出前就被更新的输入作废了（继续敲、退格、上屏、清空）。
    Superseded,

    /// R3：退格（比上次发出的短）。
    Deletion,

    /// R4：比上次发出的只多一两个字母。
    TooLittleGrowth,

    /// R8：令牌桶空了。
    RateLimited,

    /// R9：服务商回过 429，锁定期内。
    Locked,
}

impl SkipReason {
    /// 日志与报告里的短名。
    pub fn label(self) -> &'static str {
        match self {
            Self::TooShort => "字母太少",
            Self::ShortFullPinyin => "短全拼",
            Self::Manual => "按需模式",
            Self::Reused => "复用整句",
            Self::Superseded => "已作废",
            Self::Deletion => "退格",
            Self::TooLittleGrowth => "增量不足",
            Self::RateLimited => "限流",
            Self::Locked => "429 锁定",
        }
    }
}
```

`crates/qingjian-core/src/engine/prediction/mod.rs`：在 `mod script;` 后加 `mod skip_reason;`，在 `pub use script::translation_target;` 后加 `pub use skip_reason::SkipReason;`。

`crates/qingjian-core/src/engine/mod.rs` 第 50–53 行的 `pub use prediction::{…}` 里加 `SkipReason`：

```rust
pub use prediction::{
    CloudWord, NoPredictor, Prediction, PredictionKind, PredictionPolicy, PredictionRequest,
    Predictor, SkipReason, SurroundingText,
};
```

`crates/qingjian-core/src/lib.rs` 的 `pub use engine::{…}` 列表里加 `SkipReason`（字母序放在 `RawPreedit` 后；`cargo fmt` 会重排）。

- [ ] **Step 2: `Predictor::supersede` 缺省空实现**

`crates/qingjian-core/src/engine/prediction/predictor.rs`，在 `fn poll(...)` 声明之后、`/// 是否真的会联想` 之前插入：

```rust
    /// Engine 换了序号（继续敲、退格、上屏、门槛没过）：序号小于 `sequence` 的请求还没发出去的就别发了，
    /// 回来了 Engine 也会丢。缺省什么都不做（旧行为：发出去再丢）。
    fn supersede(&mut self, _sequence: u64) {}
```

- [ ] **Step 3: 写 `Dispatcher` 的失败测试**

新建 `crates/qingjian-predict/src/dispatcher/tests.rs`：

```rust
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use qingjian_core::{PredictionKind, PredictionRequest, SkipReason};

use super::{Decision, Dispatcher};
use crate::prompt::Reply;

fn request(sequence: u64, letters: &str) -> PredictionRequest {
    PredictionRequest {
        sequence,
        kind: PredictionKind::Compose,
        before: "前文".into(),
        after: String::new(),
        pinyin: letters.into(),
        letters: letters.into(),
        syllables: 0,
        candidates: vec![format!("候选{sequence}")],
        guess: String::new(),
        max_items: 4,
        want_sentence: true,
        text: String::new(),
        target_language: String::new(),
    }
}

fn dispatcher() -> (Dispatcher, Arc<AtomicU64>) {
    let superseded = Arc::new(AtomicU64::new(0));
    (Dispatcher::new(superseded.clone()), superseded)
}

fn reply() -> Reply {
    Reply {
        words: Vec::new(),
        sentence: Some("所以我想去吃饭".into()),
    }
}

#[test]
fn superseded_requests_are_not_sent() {
    let (mut dispatcher, superseded) = dispatcher();
    superseded.store(5, Ordering::Relaxed);
    let now = Instant::now();
    assert_eq!(
        dispatcher.decide(&request(4, "suoyiwo"), now),
        Decision::Skip(SkipReason::Superseded)
    );
    assert_eq!(dispatcher.decide(&request(5, "suoyiwo"), now), Decision::Send);
}

#[test]
fn identical_requests_hit_the_cache() {
    let (mut dispatcher, _) = dispatcher();
    let now = Instant::now();
    let first = request(1, "suoyiwoxiang");
    assert_eq!(dispatcher.decide(&first, now), Decision::Send);
    dispatcher.note_reply(&first, &reply());
    let mut again = first.clone();
    again.sequence = 2;
    assert_eq!(dispatcher.decide(&again, now), Decision::Cached);
    assert_eq!(dispatcher.cached(&again), Some(&reply()));
}

#[test]
fn empty_replies_are_not_cached() {
    let (mut dispatcher, _) = dispatcher();
    let now = Instant::now();
    let first = request(1, "?mumumu");
    assert_eq!(dispatcher.decide(&first, now), Decision::Send);
    dispatcher.note_reply(&first, &Reply::default());
    assert_eq!(dispatcher.cached(&first), None);
}
```

- [ ] **Step 4: 跑测试确认失败**

Run: `cargo test -p qingjian-predict dispatcher`
Expected: 编译失败，`unresolved import super::{Decision, Dispatcher}` / `file not found for module dispatcher`。

- [ ] **Step 5: 写 `Dispatcher`（第一版：作废 + 缓存）**

新建 `crates/qingjian-predict/src/dispatcher/mod.rs`：

```rust
//! 防抖之后、发网络之前的那一道：作废的不发、缓存命中直接答、退格与小增量不发、限流与 429 锁定期不发。
//! [`crate::worker::Worker`] 与 `qingjian-cli --predict-sim` 共用这一个类型，模拟出来的次数就是真实规则下的次数。

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use qingjian_core::{PredictionRequest, SkipReason};

use crate::cache::PredictionCache;
use crate::prompt::Reply;

/// 缓存条数。
const CACHE_CAPACITY: usize = 64;

/// 一个请求过了防抖之后怎么处理。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    /// 缓存里有，直接用（[`Dispatcher::cached`] 取）。
    Cached,

    /// 真的发出去；发送记录已经记下。
    Send,

    /// 不发。
    Skip(SkipReason),
}

pub struct Dispatcher {
    /// 结果缓存。
    cache: PredictionCache,

    /// Engine 作废到的序号：序号比它小的请求发出去也没人收（[`qingjian_core::Predictor::supersede`]）。
    superseded: Arc<AtomicU64>,
}

impl Dispatcher {
    pub fn new(superseded: Arc<AtomicU64>) -> Self {
        Self {
            cache: PredictionCache::with_capacity(CACHE_CAPACITY),
            superseded,
        }
    }

    /// 决定这个请求发不发。`_now` 给之后的限流与 429 锁定用（Task 8）。
    pub fn decide(&mut self, request: &PredictionRequest, _now: Instant) -> Decision {
        if request.sequence < self.superseded.load(Ordering::Relaxed) {
            return Decision::Skip(SkipReason::Superseded);
        }
        if self.cache.get(&PredictionCache::key(request)).is_some() {
            return Decision::Cached;
        }
        Decision::Send
    }

    pub fn cached(&self, request: &PredictionRequest) -> Option<&Reply> {
        self.cache.get(&PredictionCache::key(request))
    }

    /// 回复到了：非空的进缓存。空回复不进：模型偶尔什么都不给（问字尤其），缓存住就等于这个问题以后永远没答案
    /// （2026-09-07 `?mumumu` 就是这么卡住的）。
    pub fn note_reply(&mut self, request: &PredictionRequest, reply: &Reply) {
        if !reply.is_empty() {
            self.cache
                .insert(PredictionCache::key(request), reply.clone());
        }
    }
}

#[cfg(test)]
mod tests;
```

`crates/qingjian-predict/src/lib.rs`：`mod connection;` 后加 `mod dispatcher;`；`pub use connection::{…};` 后加 `pub use dispatcher::{Decision, Dispatcher};`；`pub use gloss::CloudGlossFiller;` 后加 `pub use prompt::Reply;`（CLI 模拟要往缓存里放占位回复）。

- [ ] **Step 6: 后台线程改用 `Dispatcher`，`CloudPredictor` 接 `supersede`**

`crates/qingjian-predict/src/worker.rs` 整个替换为：

```rust
//! 后台线程：收请求、防抖、过 [`Dispatcher`]（作废 / 缓存 / 增量 / 限流）、发网络请求、回结果。

use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::time::{Duration, Instant};

use qingjian_core::{Prediction, PredictionRequest};

use crate::chat_client::ChatClient;
use crate::dispatcher::{Decision, Dispatcher};
use crate::error::PredictError;
use crate::prompt::Reply;

pub struct Worker {
    /// 请求入口。主线程 drop 掉发送端后线程自然退出。
    requests: Receiver<PredictionRequest>,

    /// 结果出口。
    responses: Sender<Prediction>,

    /// 网络客户端。
    client: ChatClient,

    /// 防抖窗口。
    debounce: Duration,

    /// 发不发的判断与缓存。
    dispatcher: Dispatcher,
}

impl Worker {
    pub fn new(
        requests: Receiver<PredictionRequest>,
        responses: Sender<Prediction>,
        client: ChatClient,
        debounce: Duration,
        dispatcher: Dispatcher,
    ) -> Self {
        Self {
            requests,
            responses,
            client,
            debounce,
            dispatcher,
        }
    }

    /// 阻塞运行直到发送端全部关闭。
    pub fn run(mut self) -> Result<(), PredictError> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        while let Ok(first) = self.requests.recv() {
            let Some(request) = self.debounce(first) else {
                return Ok(());
            };
            match self.dispatcher.decide(&request, Instant::now()) {
                Decision::Skip(reason) => {
                    tracing::debug!(sequence = request.sequence, reason = reason.label(), "联想不发");
                    continue;
                }
                Decision::Cached => {
                    tracing::debug!(sequence = request.sequence, kind = ?request.kind, "联想命中缓存");
                    if let Some(reply) = self.dispatcher.cached(&request).cloned() {
                        self.reply(request.sequence, reply);
                    }
                    continue;
                }
                Decision::Send => {}
            }
            let start = Instant::now();
            match runtime.block_on(self.client.complete(&request)) {
                Ok(reply) => {
                    tracing::info!(
                        sequence = request.sequence,
                        elapsed_ms = start.elapsed().as_millis(),
                        words = reply.words.len(),
                        sentence = reply.sentence.is_some(),
                        "联想完成"
                    );
                    self.dispatcher.note_reply(&request, &reply);
                    self.reply(request.sequence, reply);
                }
                Err(error) => {
                    tracing::warn!(sequence = request.sequence, %error, "联想失败");
                }
            }
        }
        Ok(())
    }

    /// 防抖：在窗口内持续收到新请求就一直等，只保留最后一个。发送端关闭返回 `None`。
    fn debounce(&self, mut latest: PredictionRequest) -> Option<PredictionRequest> {
        loop {
            match self.requests.recv_timeout(self.debounce) {
                Ok(newer) => latest = newer,
                Err(RecvTimeoutError::Timeout) => return Some(latest),
                Err(RecvTimeoutError::Disconnected) => return None,
            }
        }
    }

    fn reply(&self, sequence: u64, reply: Reply) {
        // 接收端没了说明 Predictor 已经被 drop，线程随后也会退出
        let _ = self.responses.send(Prediction {
            sequence,
            words: reply.words,
            sentence: reply.sentence,
        });
    }
}
```

`crates/qingjian-predict/src/cloud_predictor.rs`：

1. 顶部导入换成：

```rust
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::Duration;

use qingjian_core::{Prediction, PredictionPolicy, PredictionRequest, Predictor};

use crate::chat_client::ChatClient;
use crate::config::PredictConfig;
use crate::dispatcher::Dispatcher;
use crate::error::PredictError;
use crate::worker::Worker;
```

2. 结构体末尾加字段：

```rust
    /// Engine 作废到的序号，后台线程防抖后据此丢掉没人要的请求。
    superseded: Arc<AtomicU64>,
```

3. `new()` 里 `let worker = Worker::new(…)` 换成：

```rust
        let superseded = Arc::new(AtomicU64::new(0));
        let worker = Worker::new(
            request_rx,
            response_tx,
            client,
            Duration::from_millis(config.debounce_ms),
            Dispatcher::new(superseded.clone()),
        );
```

返回值 `Ok(Self { policy: config.policy(), requests, responses })` 里加 `superseded,`。

4. `impl Predictor for CloudPredictor` 里 `fn poll` 之后加：

```rust
    fn supersede(&mut self, sequence: u64) {
        self.superseded.fetch_max(sequence, Ordering::Relaxed);
    }
```

- [ ] **Step 7: 跑测试**

Run: `cargo test -p qingjian-predict && cargo test -p qingjian-core`
Expected: 全过；`dispatcher::tests` 三个用例 PASS。

- [ ] **Step 8: CLI `--predict-sim`**

新建 `apps/cli/src/predict_sim/rng.rs`：

```rust
//! 固定种子的伪随机数（xorshift64），让模拟的中途停顿可重现，不引入 rand。

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed.max(1))
    }

    /// 以概率 `p` 返回 `true`。
    pub fn chance(&mut self, p: f64) -> bool {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        ((self.0 >> 11) as f64 / (1u64 << 53) as f64) < p
    }
}
```

新建 `apps/cli/src/predict_sim/predictor.rs`：

```rust
//! 模拟用的 Predictor：只记下防抖窗口里最后一个请求与作废到的序号，不联网。

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use qingjian_core::{Prediction, PredictionPolicy, PredictionRequest, Predictor};

pub struct SimPredictor {
    /// 门槛与上下文窗口，来自配置的 `[predict]`。
    pub policy: PredictionPolicy,

    /// 最后一个提交的请求（防抖只留最后一个）。
    pub pending: Arc<Mutex<Option<PredictionRequest>>>,

    /// 与模拟里的 Dispatcher 共用。
    pub superseded: Arc<AtomicU64>,
}

impl Predictor for SimPredictor {
    fn policy(&self) -> PredictionPolicy {
        self.policy
    }

    fn submit(&mut self, request: PredictionRequest) {
        *self.pending.lock().unwrap() = Some(request);
    }

    fn poll(&mut self) -> Option<Prediction> {
        None
    }

    fn supersede(&mut self, sequence: u64) {
        self.superseded.fetch_max(sequence, Ordering::Relaxed);
    }
}
```

新建 `apps/cli/src/predict_sim/composition.rs`：

```rust
//! 从输入日志里切出一段段组句：一段拼音分几次上屏的算一段，`retype` 事件记到下一段上。

use qingjian_core::{InputLogEntry, InputSource};

/// 一段组句。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Composition {
    /// 整段作用域的字母（全拼，不含 `'`）。
    pub letters: String,

    /// 第一键到第一次上屏的毫秒数。
    pub ms: u64,

    /// 这段之前记下的重打次数。
    pub retypes: usize,
}

impl Composition {
    /// 最后一键到上屏的余量（毫秒）：总时长减去按固定键间隔敲完前面各键的时间。
    pub fn final_pause(&self, key_gap: u64) -> u64 {
        let typing = key_gap * (self.letters.len() as u64).saturating_sub(1);
        self.ms.saturating_sub(typing)
    }
}

/// 切出各段组句；返回 (组句, 不模拟的上屏条数)。英文、双拼 / 形码、原样上屏、云端词这些不模拟。
pub fn collect(entries: impl Iterator<Item = InputLogEntry>) -> (Vec<Composition>, usize) {
    let mut compositions = Vec::new();
    let mut skipped = 0;
    let mut retypes = 0;
    let mut previous: Option<(String, String)> = None;
    for entry in entries {
        let commit = match entry {
            InputLogEntry::Retype { .. } => {
                retypes += 1;
                continue;
            }
            InputLogEntry::Commit(commit) => commit,
            _ => continue,
        };
        let scope = commit.scope.replace('\'', "").to_lowercase();
        let wanted = !commit.english
            && commit.scheme.is_empty()
            && matches!(
                commit.source,
                InputSource::Word | InputSource::Sentence | InputSource::CloudSentence
            )
            && !scope.is_empty()
            && scope.chars().all(|c| c.is_ascii_lowercase());
        if !wanted {
            skipped += 1;
            previous = None;
            continue;
        }
        // 同一段拼音分几次上屏：后几次的作用域是前一次剩下的尾巴
        let continued = previous.as_ref().is_some_and(|(scope_before, keys_before)| {
            scope_before != keys_before
                && scope_before.ends_with(&scope)
                && scope.len() < scope_before.len()
        });
        previous = Some((scope.clone(), commit.keys.replace('\'', "").to_lowercase()));
        if continued {
            continue;
        }
        compositions.push(Composition {
            letters: scope,
            ms: commit.ms,
            retypes,
        });
        retypes = 0;
    }
    (compositions, skipped)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn commit(scope: &str, keys: &str, ms: u64) -> InputLogEntry {
        serde_json::from_str(&format!(
            r#"{{"event":"commit","id":1,"scope":"{scope}","keys":"{keys}","pinyin":"","corrected":false,"text":"字","source":"word","index":0,"top":[],"scheme":"","english":false,"ms":{ms}}}"#
        ))
        .unwrap()
    }

    #[test]
    fn partial_commits_of_one_composition_count_once() {
        let retype: InputLogEntry =
            serde_json::from_str(r#"{"event":"retype","before":"a","after":"b","of":1}"#).unwrap();
        let entries = vec![
            retype,
            commit("suoyiwoxiang", "suoyi", 1500),
            commit("woxiang", "woxiang", 1900),
            commit("nihao", "nihao", 700),
        ];
        let (compositions, skipped) = collect(entries.into_iter());
        assert_eq!(skipped, 0);
        assert_eq!(compositions.len(), 2);
        assert_eq!(compositions[0].letters, "suoyiwoxiang");
        assert_eq!(compositions[0].retypes, 1);
        assert_eq!(compositions[1].letters, "nihao");
        assert_eq!(compositions[1].final_pause(100), 300);
    }
}
```

新建 `apps/cli/src/predict_sim/report.rs`：

```rust
//! `--predict-sim` 的报告。

use std::collections::BTreeMap;
use std::fmt;

use qingjian_core::SkipReason;

#[derive(Debug, Default)]
pub struct Report {
    /// 中途停顿概率。
    pub pause: f64,

    /// 防抖（毫秒）。
    pub debounce_ms: u64,

    /// 模拟了几段组句。
    pub compositions: usize,

    /// 这些组句共几个字母。
    pub letters: usize,

    /// 没模拟的上屏（英文、双拼、原样……）。
    pub skipped_logs: usize,

    /// 壳调了几次 `request_prediction`。
    pub asked: usize,

    /// Core 提交给 Predictor 的次数。
    pub submitted: usize,

    /// Core 没提交的次数。
    pub held_by_core: usize,

    /// Core 各门槛拦下的次数（拦下但不归门槛管的不在这里）。
    pub core_reasons: BTreeMap<SkipReason, usize>,

    /// 防抖之后真的发出去的次数：这就是花钱的请求数。
    pub sent: usize,

    /// 防抖之后命中缓存的次数。
    pub cached: usize,

    /// 防抖之后被 Dispatcher 拦下的次数。
    pub held_by_worker: BTreeMap<SkipReason, usize>,
}

fn reasons(counts: &BTreeMap<SkipReason, usize>) -> String {
    if counts.is_empty() {
        return "—".to_owned();
    }
    counts
        .iter()
        .map(|(reason, count)| format!("{} {count}", reason.label()))
        .collect::<Vec<_>>()
        .join(" / ")
}

impl fmt::Display for Report {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "云联想请求模拟：{} 段组句、{} 个字母（另有 {} 条上屏不模拟）；中途停顿概率 {:.2}、防抖 {} ms",
            self.compositions, self.letters, self.skipped_logs, self.pause, self.debounce_ms
        )?;
        writeln!(f, "  壳调用      {}", self.asked)?;
        writeln!(
            f,
            "  Core 提交   {}；不提交 {}（门槛：{}）",
            self.submitted,
            self.held_by_core,
            reasons(&self.core_reasons)
        )?;
        writeln!(
            f,
            "  防抖之后    命中缓存 {}；不发 {}（{}）",
            self.cached,
            self.held_by_worker.values().sum::<usize>(),
            reasons(&self.held_by_worker)
        )?;
        let per = |count: usize, base: usize| {
            if base == 0 {
                0.0
            } else {
                count as f64 / base as f64
            }
        };
        writeln!(
            f,
            "  实发        {} 次（每段组句 {:.2} 次，每百字母 {:.1} 次）",
            self.sent,
            per(self.sent, self.compositions),
            per(self.sent * 100, self.letters)
        )
    }
}
```

新建 `apps/cli/src/predict_sim/mod.rs`：

```rust
//! 云联想请求量模拟（`--predict-sim`）：按输入日志重放每段组句的逐键输入，用 Engine 的真实门槛与
//! `qingjian-predict` 的 [`Dispatcher`]（作废 / 缓存 / 退格 / 增量 / 限流）数「会发几次、被哪条规则拦下」，不发网络请求。
//!
//! 日志没有逐键时刻，按 `docs/notes/cloud-request-throttle.md`「模拟模型」一节：键间隔 100 ms；最后一键到上屏
//! 的余量 ≥ 防抖算一次停顿；中途停顿按概率 `pause` 撒在第 2 键之后；每条 `retype` 事件算「退一格、停顿、敲回来、停顿」。
//! 上屏时壳作废联想，还在防抖里的那个请求交给 Dispatcher 决定（旧版照发，作废了的不发）。整句复用（R5）要回复内容，模拟不了。

mod composition;
mod predictor;
mod report;
mod rng;

use std::path::Path;
use std::sync::atomic::AtomicU64;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use qingjian_core::{Engine, InputLogEntry, PredictionRequest};
use qingjian_predict::{Decision, Dispatcher, PredictConfig, Reply};

use crate::replay::ReplayError;
use composition::Composition;
use predictor::SimPredictor;
pub use report::Report;
use rng::Rng;

/// 键间隔（毫秒）：日志 ms/键 的 p25，偏保守。
const KEY_GAP_MS: u64 = 100;

pub fn run(
    engine: &mut Engine,
    path: &Path,
    config: &PredictConfig,
    pause: f64,
    seed: u64,
) -> Result<Report, ReplayError> {
    let text = std::fs::read_to_string(path).map_err(|source| ReplayError::Read {
        path: path.to_owned(),
        source,
    })?;
    let entries = text
        .lines()
        .filter_map(|line| serde_json::from_str::<InputLogEntry>(line).ok());
    let (compositions, skipped) = composition::collect(entries);
    let pending = Arc::new(Mutex::new(None));
    let superseded = Arc::new(AtomicU64::new(0));
    engine.set_predictor(Box::new(SimPredictor {
        policy: config.policy(),
        pending: pending.clone(),
        superseded: superseded.clone(),
    }));
    let mut sim = Sim {
        engine,
        pending,
        dispatcher: Dispatcher::new(superseded),
        clock: Instant::now(),
        debounce: config.debounce_ms,
        report: Report {
            pause,
            debounce_ms: config.debounce_ms,
            skipped_logs: skipped,
            ..Report::default()
        },
    };
    let mut rng = Rng::new(seed);
    for composition in &compositions {
        sim.composition(composition, &mut rng, pause);
    }
    Ok(sim.report)
}

struct Sim<'a> {
    engine: &'a mut Engine,

    /// 防抖窗口里最后一个请求（SimPredictor 收的）。
    pending: Arc<Mutex<Option<PredictionRequest>>>,

    dispatcher: Dispatcher,

    /// 模拟时钟。
    clock: Instant,

    debounce: u64,

    report: Report,
}

impl Sim<'_> {
    fn composition(&mut self, composition: &Composition, rng: &mut Rng, pause: f64) {
        let letters = composition.letters.as_str();
        let n = letters.len();
        self.report.compositions += 1;
        self.report.letters += n;
        self.engine.clear();
        for k in 1..=n {
            self.ask(&letters[..k]);
            let gap = if k == n {
                composition.final_pause(KEY_GAP_MS)
            } else if k >= 2 && rng.chance(pause) {
                self.debounce
            } else {
                KEY_GAP_MS
            };
            self.wait(gap);
        }
        for _ in 0..composition.retypes {
            self.ask(&letters[..n - 1]);
            self.wait(self.debounce);
            self.ask(letters);
            self.wait(self.debounce);
        }
        // 上屏：壳作废联想；防抖里还压着的请求照 Dispatcher 的规则处理
        self.engine.cancel_prediction();
        self.wait(self.debounce);
        self.engine.clear();
    }

    /// 壳在缓冲变化后做的事：查候选、发联想请求。
    fn ask(&mut self, input: &str) {
        self.engine.set_input(input);
        let local = self
            .engine
            .query()
            .map(|query| query.candidates.items)
            .unwrap_or_default();
        self.report.asked += 1;
        if self.engine.request_prediction(None, &local).is_some() {
            self.report.submitted += 1;
        } else {
            self.report.held_by_core += 1;
        }
    }

    /// 时间往前走 `ms`；够防抖就把压着的请求交给 Dispatcher。
    fn wait(&mut self, ms: u64) {
        self.clock += Duration::from_millis(ms);
        if ms < self.debounce {
            return;
        }
        let Some(request) = self.pending.lock().unwrap().take() else {
            return;
        };
        match self.dispatcher.decide(&request, self.clock) {
            Decision::Send => {
                self.report.sent += 1;
                // 回复内容模拟不了，只要它进缓存：非空的占位
                let reply = Reply {
                    words: Vec::new(),
                    sentence: Some(String::new()),
                };
                self.dispatcher.note_reply(&request, &reply);
            }
            Decision::Cached => self.report.cached += 1,
            Decision::Skip(reason) => *self.report.held_by_worker.entry(reason).or_default() += 1,
        }
    }
}
```

`apps/cli/src/main.rs`：`mod logging;` 后加 `mod predict_sim;`；`run()` 里 `if let Some(path) = &args.replay {` 之前插入：

```rust
    if let Some(path) = &args.predict_sim {
        let config_path = args
            .config
            .clone()
            .unwrap_or_else(args::default_config_file);
        let predict = Config::load(&config_path)?.predict;
        let report = predict_sim::run(
            &mut engine,
            path,
            &predict,
            args.predict_sim_pause,
            args.predict_sim_seed,
        )?;
        print!("{report}");
        return Ok(());
    }
```

`apps/cli/src/args.rs`：`/// 同音快照：` 那个字段之前插入：

```rust
    /// 云联想请求量模拟：按输入日志逐键重放，用 Engine 与 qingjian-predict 的真实门槛数会发几次请求、被哪条规则拦下，
    /// 不联网、不写文件。门槛读 `--config` 的 `[predict]`
    #[arg(long)]
    pub predict_sim: Option<PathBuf>,

    /// `--predict-sim` 的中途停顿概率（每两个键之间停顿超过防抖的机会）
    #[arg(long, default_value_t = 0.1, requires = "predict_sim")]
    pub predict_sim_pause: f64,

    /// `--predict-sim` 撒停顿用的随机种子（固定种子结果可重现）
    #[arg(long, default_value_t = 1, requires = "predict_sim")]
    pub predict_sim_seed: u64,

```

- [ ] **Step 9: 编译、测试、量基线**

```bash
cd $WT
cargo test -p qingjian-cli predict_sim
cargo build --release -p qingjian-cli
for p in 0 0.1 0.2; do
  target/release/qingjian-cli --config tools/eval/offline.toml --predict-sim data/eval/input-log-2026-10-04.jsonl --predict-sim-pause $p 2>/dev/null
done | tee /tmp/predict-sim-task1.txt
```

Expected：`partial_commits_of_one_composition_count_once` PASS；三段报告，「Core 提交」后门槛一栏是 `—`、防抖之后没有「已作废」（Engine 还不调 `supersede`），「实发」在 750 / 1080 / 1350 上下（我在近似旧规则下量到 750 / 1082 / 1350）。**把三行「实发」原样记下，后面每个任务都和它比。**

- [ ] **Step 10: fmt / clippy / 提交**

```bash
cargo fmt --all && cargo clippy --all-targets -- -D warnings
git add crates/qingjian-core crates/qingjian-predict apps/cli
git commit -m "feat(cli): --predict-sim 按输入日志模拟云联想请求数，发送判断抽成 Dispatcher" -m "后台线程「查缓存」那段抽成 qingjian-predict::Dispatcher，CLI 与后台线程共用，模拟出来的次数就是真实规则的次数。
Predictor 加缺省空实现的 supersede、Core 加 SkipReason，这一步 Engine 还不调，行为不变。
冻结日志基线（p=0/0.1/0.2 实发）：<填 Step 9 的三个数>。"
```

---

## Task 2: 缺省值集中、配置面（无行为变化）

所有新常数进 `defaults.rs` 一处；`[predict]` 新键与 `[shortcut] predict_now` 能解析、模板写清楚；这一步谁都还不读它们（旧测试的假联想器把门槛放回旧值，后面加门槛时它们不受影响）。

**Files:**
- Create: `crates/qingjian-core/src/engine/prediction/defaults.rs`、`crates/qingjian-core/src/engine/prediction/trigger.rs`、`crates/qingjian-core/src/engine/late/mod.rs`、`crates/qingjian-core/src/engine/late/policy.rs`
- Modify: `crates/qingjian-core/src/engine/prediction/{mod.rs,policy.rs}`、`crates/qingjian-core/src/engine/mod.rs`、`crates/qingjian-core/src/lib.rs`、`crates/qingjian-core/src/engine/tests/mod.rs:155-163`
- Modify: `crates/qingjian-predict/src/config.rs`
- Modify: `crates/qingjian-platform/src/config/{key_combo.rs,shortcut.rs,mod.rs}`

- [ ] **Step 1: 写失败测试（配置解析）**

`crates/qingjian-platform/src/config/mod.rs` 的 `#[cfg(test)] mod tests` 里，`fn fuzzy_section_parses()` 之前加：

```rust
    #[test]
    fn prediction_thresholds_and_predict_now_parse() {
        let config: Config = toml::from_str(
            "[predict]\ntrigger = \"manual\"\nmin_letters = 3\nsentence_first = true\n\n[shortcut]\npredict_now = \"control+option+y\"\n",
        )
        .unwrap();
        let policy = config.predict.policy();
        assert_eq!(policy.trigger, qingjian_core::PredictionTrigger::Manual);
        assert_eq!(policy.min_letters, 3);
        assert_eq!(policy.short_full_letters, 6);
        assert!(policy.late.sentence_first);
        assert_eq!(policy.late.frozen, 3);
        assert_eq!(config.shortcut.predict_now.key, 'y');
        let defaults = Config::default();
        assert_eq!(defaults.predict.rate_burst, 0);
        assert_eq!(
            defaults.shortcut.predict_now,
            crate::KeyCombo::PREDICT_NOW_DEFAULT
        );
    }
```

Run: `cargo test -p qingjian-platform prediction_thresholds`
Expected: 编译失败（`PredictionTrigger`、`predict_now`、`policy.late` 不存在）。

- [ ] **Step 2: Core 常数与类型**

新建 `crates/qingjian-core/src/engine/prediction/defaults.rs`：

```rust
//! 云联想请求与晚到插位的缺省常数，集中在这一处改。`[predict]` 配置的缺省值（`qingjian-predict` 的 `PredictConfig`）
//! 与 Core 的 [`super::PredictionPolicy`] / [`crate::engine::LatePolicy`] 都从这里取。
//!
//! 出处与取舍见 `docs/notes/cloud-request-throttle.md`（R1–R11 与晚到插位）；
//! WeType 的值只是起点，青简要用 `qingjian-cli --predict-sim` 与真机重扫。

/// R1：组句拼音少于这么多字母不发（WeType 无前文时 4 个键）。
pub const MIN_LETTERS: usize = 4;

/// R2：不超过这么多字母、且整串能切成完整音节时不发：本地首选够用（本机日志这类组句首选命中 95%）。
pub const SHORT_FULL_LETTERS: usize = 6;

/// R4：同一段组句里，比上次真正发出的请求多这么多字母才再发。
pub const MIN_GROWTH: usize = 3;

/// R8：令牌桶突发容量；0 表示不限流（缺省关）。
pub const RATE_BURST: u32 = 0;

/// R8：令牌桶每隔多少毫秒补一个。
pub const RATE_REFILL_MS: u64 = 5000;

/// R9：服务商回 429 时锁定多少秒（拿不到 `Retry-After`，一律用它）。
pub const LOCK_SECS: u64 = 30;

/// R10：被 429 锁过之后多少秒内防抖抬高。
pub const LOCK_COOLDOWN_SECS: u64 = 600;

/// R10：抬高后的防抖下限（毫秒）。
pub const LOCKED_DEBOUNCE_MS: u64 = 600;

/// 晚到插位：候选窗画出来之后多久内到的云词还能插到第 2 位（WeType 200 ms）。
pub const LATE_GRACE_MS: u32 = 200;

/// 晚到插位：超时或用户动过高亮之后前几位不动，云词从第 `LATE_FROZEN + 1` 位起插（已定：第 4 位起，不丢）。
pub const LATE_FROZEN: usize = 3;

/// 晚到插位：时限内最多插几条云词（WeType 普通云词 1 条）。
pub const LATE_MAX_IN_GRACE: usize = 1;

/// 云整句排第一（#341），缺省关。
pub const SENTENCE_FIRST: bool = false;

/// 云整句刚换到第一位后这么多毫秒内按下的上屏仍给原来的首选：防「正要按空格时首选被换」。
pub const SENTENCE_FIRST_GUARD_MS: u64 = 150;
```

新建 `crates/qingjian-core/src/engine/prediction/trigger.rs`：

```rust
//! 云联想什么时候发请求：组句中按门槛自动发，或只在按快捷键时发（`[predict] trigger`）。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PredictionTrigger {
    /// 组句中按门槛自动发（缺省）。
    #[default]
    Auto,

    /// 只在用户按 `[shortcut] predict_now` 时发；问字模式不受影响（它本身就是用户主动发问）。
    Manual,
}
```

新建 `crates/qingjian-core/src/engine/late/policy.rs`：

```rust
//! 云端结果晚到时的插位策略（B5 的子集：只管云端词与云整句；整句 / 词级神经重排的晚到规则以后并进来）。

use crate::engine::prediction::defaults;

/// 常数是 WeType 的值或已定的缺省（见 [`defaults`]），青简要用真机误选率重扫。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LatePolicy {
    /// 候选窗画出来之后多久内到的结果还能插到第 2 位（毫秒）。
    pub grace_ms: u32,

    /// 过了时限、或用户动过高亮之后，前几位冻结（3：第 4 位起插）。
    pub frozen: usize,

    /// 时限内云词最多插几条。
    pub max_in_grace: usize,

    /// 云整句是这段拼音的纯转换时排到第一位（#341）。
    pub sentence_first: bool,
}

impl Default for LatePolicy {
    fn default() -> Self {
        Self {
            grace_ms: defaults::LATE_GRACE_MS,
            frozen: defaults::LATE_FROZEN,
            max_in_grace: defaults::LATE_MAX_IN_GRACE,
            sentence_first: defaults::SENTENCE_FIRST,
        }
    }
}
```

新建 `crates/qingjian-core/src/engine/late/mod.rs`（Task 10 / 11 往里加）：

```rust
//! 云端结果晚到后怎么并进已显示的候选：时限内插第 2 位，超时或用户动过之后从第 `frozen + 1` 位起插，
//! 用户固定位置的词不动；可选把纯转换的云整句排到第一。规则在 Core，各壳只报「画出来了」并照 `CloudPlacement` 摆。
//! 规则与出处：`docs/notes/cloud-request-throttle.md`。
//! 整句 / 词级神经重排的晚到规则（B5 的另一半）还在各壳里，以后并进这里。

mod policy;

pub use policy::LatePolicy;
```

`crates/qingjian-core/src/engine/prediction/policy.rs` 整个替换为：

```rust
use super::defaults;
use super::trigger::PredictionTrigger;
use crate::engine::late::LatePolicy;

/// 联想策略：观察窗口、条数上限、哪些联想开着。由 [`super::Predictor`] 提供，Engine 据此裁剪上下文、决定发不发。
///
/// 无论壳传来多长的文本，Core 都只发这么多：这是隐私上限的最后一道闸。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PredictionPolicy {
    /// 光标前最多看几个字符。
    pub before: usize,

    /// 光标后最多看几个字符。
    pub after: usize,

    /// 云端词最多插进候选几格；0 表示不要云端词（整句补全照发）。
    pub slots: usize,

    /// 向云端最多要几条（比槽位多要几条，与本地候选重复的去掉后还能填满）。
    pub max_items: usize,

    /// 组句中要不要整句补全。
    pub sentence: bool,

    /// 自动发还是只在按快捷键时发（R11）。
    pub trigger: PredictionTrigger,

    /// 组句拼音少于这么多字母不发（R1）。
    pub min_letters: usize,

    /// 不超过这么多字母、且整串是完整音节时不发（R2）；0 关掉这条。
    pub short_full_letters: usize,

    /// 云端结果晚到时插在哪（含云整句排第一）。
    pub late: LatePolicy,
}

impl Default for PredictionPolicy {
    fn default() -> Self {
        Self {
            before: 64,
            after: 32,
            slots: 2,
            max_items: 4,
            sentence: true,
            trigger: PredictionTrigger::Auto,
            min_letters: defaults::MIN_LETTERS,
            short_full_letters: defaults::SHORT_FULL_LETTERS,
            late: LatePolicy::default(),
        }
    }
}
```

`crates/qingjian-core/src/engine/prediction/mod.rs`：`mod cloud_word;` 后加 `pub mod defaults;`；`mod surrounding_text;` 后加 `mod trigger;`；`pub use surrounding_text::SurroundingText;` 后加 `pub use trigger::PredictionTrigger;`。

`crates/qingjian-core/src/engine/mod.rs`：`mod input_log;` 后加 `mod late;`；把 Task 1 改过的 `pub use prediction::{…}` 换成：

```rust
pub use late::LatePolicy;
pub use prediction::defaults as prediction_defaults;
pub use prediction::{
    CloudWord, NoPredictor, Prediction, PredictionKind, PredictionPolicy, PredictionRequest,
    PredictionTrigger, Predictor, SkipReason, SurroundingText,
};
```

`crates/qingjian-core/src/lib.rs` 的 `pub use engine::{…}` 加 `LatePolicy`、`PredictionTrigger`、`prediction_defaults`（`cargo fmt` 排序）。

`crates/qingjian-core/src/engine/tests/mod.rs` 的 `impl Predictor for EchoPredictor` 里 `fn policy` 换成（旧测试按「2 个字母就发」写的）：

```rust
    fn policy(&self) -> PredictionPolicy {
        // 旧测试按「2 个字母就发」写的：门槛放回旧值，新门槛另在 `cloud_gate.rs` 测
        PredictionPolicy {
            before: 4,
            after: 2,
            slots: 2,
            max_items: 2,
            sentence: self.sentence,
            min_letters: 2,
            short_full_letters: 0,
            ..PredictionPolicy::default()
        }
    }
```

- [ ] **Step 3: `PredictConfig` 新键**

`crates/qingjian-predict/src/config.rs`：

1. 第一行换成 `use qingjian_core::{LatePolicy, PredictionPolicy, PredictionTrigger, prediction_defaults};`
2. 结构体 `reasoning_effort` 字段之后、`}` 之前加：

```rust

    /// `auto` 组句中按门槛自动发（缺省）/ `manual` 只在按 `[shortcut] predict_now` 时发。
    pub trigger: PredictionTrigger,

    /// 组句拼音少于这么多字母不发。
    pub min_letters: usize,

    /// 不超过这么多字母、且整串是完整全拼时不发（本地首选够用）；0 关掉这条。
    pub short_full_letters: usize,

    /// 同一段组句里，比上次真正发出的多这么多字母才再发；0 关掉这条。
    pub min_growth: usize,

    /// 限流：令牌桶突发容量，0 不限流（缺省）。
    pub rate_burst: u32,

    /// 限流：每隔多少毫秒补一个令牌。
    pub rate_refill_ms: u64,

    /// 云整句是这段拼音的纯转换时排第一（`Space` 上屏它）；缺省关。
    pub sentence_first: bool,

    /// 候选窗出来多久内到的云端词插第 2 位（毫秒）；之后到的从第 `late_frozen + 1` 位起插。
    pub late_grace_ms: u32,

    /// 晚到的云端词前面保留几位本地候选不动。
    pub late_frozen: usize,
```

3. `Default` 里 `reasoning_effort: "none".to_owned(),` 之后加：

```rust
            trigger: PredictionTrigger::Auto,
            min_letters: prediction_defaults::MIN_LETTERS,
            short_full_letters: prediction_defaults::SHORT_FULL_LETTERS,
            min_growth: prediction_defaults::MIN_GROWTH,
            rate_burst: prediction_defaults::RATE_BURST,
            rate_refill_ms: prediction_defaults::RATE_REFILL_MS,
            sentence_first: prediction_defaults::SENTENCE_FIRST,
            late_grace_ms: prediction_defaults::LATE_GRACE_MS,
            late_frozen: prediction_defaults::LATE_FROZEN,
```

4. `policy()` 的返回值里 `sentence: self.sentence,` 之后加：

```rust
            trigger: self.trigger,
            min_letters: self.min_letters,
            short_full_letters: self.short_full_letters,
            late: LatePolicy {
                grace_ms: self.late_grace_ms,
                frozen: self.late_frozen,
                sentence_first: self.sentence_first,
                ..LatePolicy::default()
            },
```

- [ ] **Step 4: `[shortcut] predict_now`**

`crates/qingjian-platform/src/config/key_combo.rs`，`TRANSLATE_DEFAULT` 常量之后加：

```rust

    /// 「现在问云端」（`[shortcut] predict_now`）：按需模式下唯一发联想的入口，自动模式下也能立刻发一次。
    pub const PREDICT_NOW_DEFAULT: Self = Self {
        modifiers: Modifiers {
            option: true,
            shift: false,
            control: true,
            command: false,
        },
        key: 'p',
    };
```

`crates/qingjian-platform/src/config/shortcut.rs`：结构体 `delete_candidate` 字段之后加

```rust

    /// 组句中立刻问一次云联想，不管门槛（`[predict] trigger = "manual"` 时唯一的发送入口）。
    pub predict_now: KeyCombo,
```

`Default` 里 `delete_candidate: Modifiers::SHIFT,` 之后加 `predict_now: KeyCombo::PREDICT_NOW_DEFAULT,`。

`crates/qingjian-platform/src/config/mod.rs` 模板：

1. 非 Windows 的 `template_shortcut_keys!` 里 `translate_selection = "control+option+t"` 下一行插入：

```toml
# 组句中立刻问一次云联想（不管字母多少）；[predict] trigger = "manual" 时只有按它才发
predict_now = "control+option+p"
```

2. Windows 的 `template_shortcut_keys!` 里 `translate_selection = "ctrl+alt+t"` 下一行插入：

```toml
# 组句中立刻问一次云联想（不管字母多少）；[predict] trigger = "manual" 时只有按它才发
predict_now = "ctrl+alt+p"
```

3. `[predict]` 一节 `sentence = true` 之后（空行与 `[status_bar]` 之前）插入：

```toml
# 什么时候问云端：auto 组句中自动（下面几条门槛之内）/ manual 只在按 [shortcut] predict_now 时
trigger = "auto"
# 自动模式的门槛：少于 min_letters 个字母不问；不超过 short_full_letters 个字母且都是完整拼音（nihao）不问，本地够用；
# 同一段拼音比上次问的多 min_growth 个字母才再问；退格不问。写 0 关掉对应那条
min_letters = 4
short_full_letters = 6
min_growth = 3
# 限流：最多连发 rate_burst 次，之后每 rate_refill_ms 毫秒补一次；0 不限流。服务商回 429 时总会停 30 秒
rate_burst = 0
rate_refill_ms = 5000
# 云整句是这段拼音的直接转换（不是续写）时排到第一位，Space 上屏它；缺省关（回复通常要 1 秒以上，会换掉已显示的首选）
sentence_first = false
# 候选窗出来 late_grace_ms 毫秒内到的云端词插在第 2 位，之后到的（或已移动过高亮）排在前 late_frozen 个本地候选之后
late_grace_ms = 200
late_frozen = 3
```

- [ ] **Step 5: 跑测试**

```bash
cargo test -p qingjian-platform config
cargo test -p qingjian-core && cargo test -p qingjian-predict
```

Expected：`prediction_thresholds_and_predict_now_parse` 与 `template_parses_to_defaults` PASS（后者守着模板与 `Default` 一致）；core / predict 全过。

- [ ] **Step 6: 模拟数不变**

```bash
cargo build --release -p qingjian-cli && target/release/qingjian-cli --config tools/eval/offline.toml --predict-sim data/eval/input-log-2026-10-04.jsonl 2>/dev/null
```

Expected：与 Task 1 p=0.1 那行「实发」一字不差（新字段没人读）。

- [ ] **Step 7: 提交**

```bash
cargo fmt --all && cargo clippy --all-targets -- -D warnings
git add crates apps
git commit -m "feat(platform): [predict] 加触发方式、门槛、限流、晚到插位与整句排第一的配置，[shortcut] 加 predict_now" -m "新常数集中在 qingjian-core engine/prediction/defaults.rs 一处改；配置能解析、模板写清楚，这一步还没有代码读它们。
缺省（用户定）：晚到云词第 4 位起插不丢、sentence_first 关、令牌桶关、trigger=auto。"
```

---

## Task 3: 作废即不发、组句编号

Engine 每换一次序号就 `supersede`，后台线程防抖后把更早的请求丢掉（现状：上屏在防抖窗口内时，那段组句最后一个请求照样发出去再被丢）。同时给请求编上「第几段组句」，Task 5 的退格 / 增量只在同一段里比。

**Files:**
- Create: `crates/qingjian-core/src/engine/prediction/gate.rs`、`crates/qingjian-core/src/engine/tests/cloud_gate.rs`
- Modify: `crates/qingjian-core/src/engine/prediction/{mod.rs,request.rs}`、`crates/qingjian-core/src/engine/mod.rs`（字段 + 初始化）、`crates/qingjian-core/src/engine/tests/mod.rs:3-19`
- Modify: `crates/qingjian-predict/src/dispatcher/tests.rs`、`crates/qingjian-predict/src/prompt.rs`（测试里的 `request()`）

- [ ] **Step 1: 写失败测试**

新建 `crates/qingjian-core/src/engine/tests/cloud_gate.rs`（后面几个任务往里加用例）：

```rust
//! 云联想的发送门槛（R1 / R2 / R5 / R11）、快捷键「现在问云端」与云整句排第一。

use std::sync::{Arc, Mutex};

use super::*;

/// 用缺省门槛、按序号原样回结果的假联想器；记下提交与作废到的序号。
#[derive(Default)]
struct Recorder {
    submitted: Arc<Mutex<Vec<PredictionRequest>>>,
    superseded: Arc<Mutex<u64>>,
    replies: Arc<Mutex<Vec<Prediction>>>,
    policy: PredictionPolicy,
}

impl Predictor for Recorder {
    fn policy(&self) -> PredictionPolicy {
        self.policy
    }

    fn submit(&mut self, request: PredictionRequest) {
        self.submitted.lock().unwrap().push(request);
    }

    fn poll(&mut self) -> Option<Prediction> {
        self.replies.lock().unwrap().pop()
    }

    fn supersede(&mut self, sequence: u64) {
        *self.superseded.lock().unwrap() = sequence;
    }
}

/// 整句复用要逐字查读音：样例词库里补上「所以我想去吃饭」七个字，「锁」是「所」的同音字。
const GATE_SAMPLE: &str = "所\tsuo\t100\n锁\tsuo\t10\n以\tyi\t100\n我\two\t100\n想\txiang\t100\n去\tqu\t100\n吃\tchi\t100\n饭\tfan\t100\n你好\tni hao\t100\n庄\tzhuang\t100\n";

fn gated(policy: PredictionPolicy) -> (Engine, Recorder) {
    let recorder = Recorder {
        policy,
        ..Recorder::default()
    };
    let probe = Recorder {
        submitted: recorder.submitted.clone(),
        superseded: recorder.superseded.clone(),
        replies: recorder.replies.clone(),
        policy,
    };
    let engine =
        Engine::new(Dictionary::parse(GATE_SAMPLE).unwrap()).with_predictor(Box::new(recorder));
    (engine, probe)
}

fn ask(engine: &mut Engine, input: &str) -> Option<u64> {
    engine.set_input(input);
    engine.request_prediction(None, &[])
}

#[test]
fn every_new_sequence_supersedes_the_older_ones() {
    let (mut engine, probe) = gated(PredictionPolicy::default());
    let sent = ask(&mut engine, "suoyiwox").unwrap();
    assert_eq!(*probe.superseded.lock().unwrap(), sent);
    engine.cancel_prediction();
    assert_eq!(*probe.superseded.lock().unwrap(), sent + 1);
    let next = ask(&mut engine, "suoyiwoxi").unwrap();
    assert_eq!(next, sent + 2);
    // 作废过一次就是下一段组句：发送线程不拿它和上一段比退格与增量
    let submitted = probe.submitted.lock().unwrap();
    assert_eq!(submitted[1].composition, submitted[0].composition + 1);
}
```

`crates/qingjian-core/src/engine/tests/mod.rs` 第 4 行 `mod cloud;` 后加 `mod cloud_gate;`。

Run: `cargo test -p qingjian-core cloud_gate`
Expected: 编译失败（`PredictionRequest` 没有 `composition` 字段）。

- [ ] **Step 2: 请求加 `composition`**

`crates/qingjian-core/src/engine/prediction/request.rs`，结构体最后一个字段 `target_language` 之后加：

```rust

    /// 第几段组句（Engine 每作废一次联想换一个号）：发送线程只在同一段里比退格与增量，不进缓存键。
    pub composition: u64,
```

`crates/qingjian-predict/src/dispatcher/tests.rs` 与 `crates/qingjian-predict/src/prompt.rs` 测试里各有一个 `fn request(…) -> PredictionRequest`，两处都在 `target_language: String::new(),` 之后加一行 `composition: 0,`。

- [ ] **Step 3: `PredictionGate`（第一版：组句编号）**

新建 `crates/qingjian-core/src/engine/prediction/gate.rs`：

```rust
//! Engine 这一侧的联想门槛：R1 字母太少、R2 短而完整的全拼、R11 按需模式、R5 上次的整句补全仍盖得住就复用。
//! 退格与小增量（R3 / R4）要和「真正发出去的」比，Engine 看不到防抖，那两条在 `qingjian-predict` 的发送线程里。

/// 一段组句里联想请求的来往：第几段、上次提交了什么、上次回来的整句、最近一次为什么没发。
#[derive(Debug, Clone, Default)]
pub(crate) struct PredictionGate {
    /// 第几段组句；每次作废（[`Self::reset`]）加一。
    composition: u64,
}

impl PredictionGate {
    pub(crate) fn composition(&self) -> u64 {
        self.composition
    }

    /// 组句作废（清空、上屏、换应用）：之前的来往都不再有意义。
    pub(crate) fn reset(&mut self) {
        self.composition += 1;
    }
}
```

`crates/qingjian-core/src/engine/prediction/mod.rs`：`mod fuzzy;` 后加 `mod gate;`；`pub use fuzzy::{mismatch_count, tolerance};` 后加 `pub(crate) use gate::PredictionGate;`。

`crates/qingjian-core/src/engine/mod.rs`：

1. 紧跟 `pub use prediction::{…};` 之后加 `use prediction::PredictionGate;`
2. 结构体 `prediction_sequence: u64,` 字段之后加：

```rust

    /// 联想门槛的状态：上次提交 / 回来了什么、为什么没发、复用的结果。
    prediction_gate: PredictionGate,
```

3. 构造里 `prediction_sequence: 0,` 之后加 `prediction_gate: PredictionGate::default(),`

- [ ] **Step 4: 换序号即作废**

`crates/qingjian-core/src/engine/prediction/mod.rs`：

1. `request_prediction` 里 `self.prediction_sequence += 1;` 换成 `self.bump_prediction_sequence();`（注释「不发也要换序号」保留）。
2. `request_translation` 里 `self.prediction_sequence += 1;` 换成 `self.bump_prediction_sequence();`。
3. 两处 `PredictionRequest { … target_language: …, }` 构造的最后加 `composition: self.prediction_gate.composition(),`。
4. `cancel_prediction` 换成：

```rust
    /// 作废正在飞的联想（用户清空了拼音、关掉了联想框）。
    pub fn cancel_prediction(&mut self) {
        self.bump_prediction_sequence();
        self.prediction_gate.reset();
    }

    /// 换一个联想序号，并告诉 Predictor 更早的请求不用发了。
    fn bump_prediction_sequence(&mut self) {
        self.prediction_sequence += 1;
        self.predictor.supersede(self.prediction_sequence);
    }
```

- [ ] **Step 5: 跑测试**

```bash
cargo test -p qingjian-core && cargo test -p qingjian-predict
```

Expected：全过，`every_new_sequence_supersedes_the_older_ones` PASS。

- [ ] **Step 6: 模拟**

```bash
cargo build --release -p qingjian-cli
for p in 0 0.1 0.2; do target/release/qingjian-cli --config tools/eval/offline.toml --predict-sim data/eval/input-log-2026-10-04.jsonl --predict-sim-pause $p 2>/dev/null | tail -2; done
```

Expected：「防抖之后」出现「已作废 N」（我量到约 390），「实发」比 Task 1 明显少。记下三个数。

- [ ] **Step 7: 提交**

```bash
cargo fmt --all && cargo clippy --all-targets -- -D warnings
git add crates
git commit -m "feat(core): 联想换序号时告诉发送线程，已作废的请求不再发出去" -m "以前 cancel_prediction 只换序号，后台线程防抖到点照发，回来再丢：上屏在 300ms 内的每段组句都白花一次请求。
请求带上组句编号，发送线程之后按段比较退格与增量。
--predict-sim 实发（p=0/0.1/0.2）：<Task 1 的数> → <这次的数>。"
```

---

## Task 4: R1 字母太少、R2 短而完整的全拼

**Files:**
- Modify: `crates/qingjian-core/src/engine/prediction/{gate.rs,mod.rs}`、`crates/qingjian-core/src/engine/mod.rs:412-413`（`MIN_PREDICTION_LETTERS` 注释）
- Modify: `crates/qingjian-core/src/engine/tests/cloud_gate.rs`、`apps/cli/src/predict_sim/mod.rs`

- [ ] **Step 1: 写失败测试**

`crates/qingjian-core/src/engine/tests/cloud_gate.rs` 末尾加：

```rust

#[test]
fn short_and_short_full_pinyin_are_not_sent() {
    let (mut engine, probe) = gated(PredictionPolicy::default());
    assert_eq!(ask(&mut engine, "nih"), None);
    assert_eq!(engine.prediction_skip(), Some(SkipReason::TooShort));
    for input in ["nihao", "zhuang", "nihaoa"] {
        assert_eq!(ask(&mut engine, input), None, "{input}");
        assert_eq!(engine.prediction_skip(), Some(SkipReason::ShortFullPinyin));
    }
    assert!(ask(&mut engine, "nihaoaw").is_some());
    assert_eq!(engine.prediction_skip(), None);
    assert_eq!(probe.submitted.lock().unwrap().len(), 1);
}

#[test]
fn abbreviations_are_sent_for_the_sentence_only() {
    let (mut engine, probe) = gated(PredictionPolicy::default());
    assert!(ask(&mut engine, "fhyq").is_some());
    let submitted = probe.submitted.lock().unwrap();
    assert_eq!(submitted[0].max_items, 0);
    assert!(submitted[0].want_sentence);
}
```

Run: `cargo test -p qingjian-core cloud_gate`
Expected: 编译失败（没有 `prediction_skip`）。

- [ ] **Step 2: 门槛**

`crates/qingjian-core/src/engine/prediction/gate.rs`：

1. 文件头之后加 `use super::skip_reason::SkipReason;`（空一行再写结构体文档）。
2. 结构体里 `composition` 字段之前加：

```rust
    /// 最近一次 `request_prediction` 没发的原因；发了或不归门槛管（私密、英文……）为 `None`。
    pub(crate) skip: Option<SkipReason>,

```

`crates/qingjian-core/src/engine/prediction/mod.rs` 的 `request_prediction`：

1. 函数体第一行（`if !self.predictor.is_enabled() || self.private {` 之前）加 `self.prediction_gate.skip = None;`
2. 把

```rust
        let letters = pinyin_source.chars().filter(|c| *c != '\'').count();
        if letters < MIN_PREDICTION_LETTERS {
            return None;
        }
```

换成

```rust
        let letters = pinyin_source.replace('\'', "");
        if letters.chars().count() < MIN_PREDICTION_LETTERS {
            return None;
        }
        if !question && let Some(reason) = self.gate_skip(&letters, pinyin_source, &policy) {
            self.prediction_gate.skip = Some(reason);
            return None;
        }
```

3. 构造 `PredictionRequest` 时 `letters: pinyin_source.replace('\'', ""),` 换成 `letters,`。
4. `bump_prediction_sequence` 之后加：

```rust

    /// 最近一次 [`Self::request_prediction`] 被哪条门槛拦下；发了、或因私密 / 英文 / 辅码这类状态不发时为 `None`。
    /// 日志与 `qingjian-cli --predict-sim` 用。
    pub fn prediction_skip(&self) -> Option<SkipReason> {
        self.prediction_gate.skip
    }

    /// R1 / R2：字母太少不发；短而完整的全拼不发（本地首选够用）。
    fn gate_skip(
        &self,
        letters: &str,
        pinyin_source: &str,
        policy: &PredictionPolicy,
    ) -> Option<SkipReason> {
        let count = letters.chars().count();
        if count < policy.min_letters {
            return Some(SkipReason::TooShort);
        }
        let full = matches!(
            segment_longest_prefix(pinyin_source),
            Ok((segmentations, "")) if segmentations
                .iter()
                .any(|s| s.syllables.iter().all(|syllable| syllable.complete))
        );
        (count <= policy.short_full_letters && full).then_some(SkipReason::ShortFullPinyin)
    }
```

`crates/qingjian-core/src/engine/mod.rs` 的 `MIN_PREDICTION_LETTERS` 注释换成：

```rust
/// 拼音短于这个字母数一律不联想（问字、快捷键也不例外）；组句自动联想的门槛更高，见 [`PredictionPolicy::min_letters`]。
const MIN_PREDICTION_LETTERS: usize = 2;
```

- [ ] **Step 3: 跑测试**

Run: `cargo test -p qingjian-core`
Expected: 全过（旧 `cloud.rs` / `english.rs` / `privacy.rs` 的用例用 `EchoPredictor`，门槛是旧值，不受影响）。

- [ ] **Step 4: 模拟按原因分类**

`apps/cli/src/predict_sim/mod.rs` 的 `ask()` 里

```rust
        } else {
            self.report.held_by_core += 1;
        }
```

换成

```rust
        } else {
            self.report.held_by_core += 1;
            if let Some(reason) = self.engine.prediction_skip() {
                *self.report.core_reasons.entry(reason).or_default() += 1;
            }
        }
```

```bash
cargo build --release -p qingjian-cli
for p in 0 0.1 0.2; do target/release/qingjian-cli --config tools/eval/offline.toml --predict-sim data/eval/input-log-2026-10-04.jsonl --predict-sim-pause $p 2>/dev/null; done
```

Expected：「门槛：字母太少 ≈2200 / 短全拼 ≈1300」，实发再降（R2 是最大的一条）。记下三个数。

- [ ] **Step 5: 提交**

```bash
cargo fmt --all && cargo clippy --all-targets -- -D warnings
git add crates apps
git commit -m "feat(core): 组句联想少于 4 个字母、6 个字母内的完整全拼不发" -m "R1/R2（WeType NeedDoRequest：无前文至少 4 键、4–6 键且是合法音节不发；青简扩到多音节全拼）：本机日志里这类组句本地首选命中 95%，云端帮不上。
问字不受限（仍是旧的 2 字母下限）。门槛在 [predict] min_letters / short_full_letters，0 关。
--predict-sim 实发（p=0/0.1/0.2）：<上次> → <这次>。"
```

---

## Task 5: R3 退格不发、R4 小增量不发（发送线程）

**Files:**
- Modify: `crates/qingjian-predict/src/dispatcher/{mod.rs,tests.rs}`、`crates/qingjian-predict/src/cloud_predictor.rs`、`apps/cli/src/predict_sim/mod.rs`

- [ ] **Step 1: 写失败测试**

`crates/qingjian-predict/src/dispatcher/tests.rs`：

1. 导入加 `use crate::config::PredictConfig;`
2. `fn dispatcher()` 换成：

```rust
fn dispatcher(config: &PredictConfig) -> (Dispatcher, Arc<AtomicU64>) {
    let superseded = Arc::new(AtomicU64::new(0));
    (Dispatcher::new(config, superseded.clone()), superseded)
}
```

   已有三个用例里的 `dispatcher()` 都改成 `dispatcher(&PredictConfig::default())`。
3. 末尾加：

```rust

#[test]
fn deletion_and_small_growth_are_not_sent() {
    let (mut dispatcher, _) = dispatcher(&PredictConfig::default());
    let now = Instant::now();
    assert_eq!(dispatcher.decide(&request(1, "suoyiwo"), now), Decision::Send);
    for (sequence, letters) in [(2, "suoyiwox"), (3, "suoyiwoxi")] {
        assert_eq!(
            dispatcher.decide(&request(sequence, letters), now),
            Decision::Skip(SkipReason::TooLittleGrowth)
        );
    }
    assert_eq!(dispatcher.decide(&request(4, "suoyiwoxia"), now), Decision::Send);
    assert_eq!(
        dispatcher.decide(&request(5, "suoyiwoxi"), now),
        Decision::Skip(SkipReason::Deletion)
    );
    // 换了前文、或已经是下一段组句（上屏后重新打），都不比
    let mut other = request(6, "suoy");
    other.before = "别处".into();
    assert_eq!(dispatcher.decide(&other, now), Decision::Send);
    let mut next = request(7, "suoy");
    next.composition = 1;
    assert_eq!(dispatcher.decide(&next, now), Decision::Send);
}
```

Run: `cargo test -p qingjian-predict dispatcher`
Expected: 编译失败（`Dispatcher::new` 只收一个参数）。

- [ ] **Step 2: 实现**

`crates/qingjian-predict/src/dispatcher/mod.rs`：

1. 导入改成：

```rust
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use qingjian_core::{PredictionKind, PredictionRequest, SkipReason};

use crate::cache::PredictionCache;
use crate::config::PredictConfig;
use crate::prompt::Reply;
```

2. `pub enum Decision` 之后加：

```rust

/// 上一次真正发出去的组句请求：退格与增量判断都和它比。
#[derive(Debug, Clone, PartialEq, Eq)]
struct Sent {
    composition: u64,

    before: String,

    after: String,

    letters: String,
}
```

3. `Dispatcher` 结构体 `superseded` 字段之后加：

```rust

    /// 同一段组句里比上次发出的多几个字母才再发（R4）；0 不限。
    min_growth: usize,

    /// 上一次真正发出去的组句请求。
    last_sent: Option<Sent>,
```

4. `new` 与 `decide` 换成：

```rust
    pub fn new(config: &PredictConfig, superseded: Arc<AtomicU64>) -> Self {
        Self {
            cache: PredictionCache::with_capacity(CACHE_CAPACITY),
            superseded,
            min_growth: config.min_growth,
            last_sent: None,
        }
    }

    /// 决定这个请求发不发。返回 [`Decision::Send`] 时已记成「上次发出的」。`_now` 给 Task 8 的限流与 429 锁定用。
    pub fn decide(&mut self, request: &PredictionRequest, _now: Instant) -> Decision {
        if request.sequence < self.superseded.load(Ordering::Relaxed) {
            return Decision::Skip(SkipReason::Superseded);
        }
        if self.cache.get(&PredictionCache::key(request)).is_some() {
            return Decision::Cached;
        }
        if let Some(reason) = self.growth_skip(request) {
            return Decision::Skip(reason);
        }
        if request.kind == PredictionKind::Compose {
            self.last_sent = Some(Sent {
                composition: request.composition,
                before: request.before.clone(),
                after: request.after.clone(),
                letters: request.letters.clone(),
            });
        }
        Decision::Send
    }

    /// R3 / R4：只比同一段组句、同样上下文里的组句请求；问字、翻译不受限。
    fn growth_skip(&self, request: &PredictionRequest) -> Option<SkipReason> {
        let last = self.last_sent.as_ref()?;
        if request.kind != PredictionKind::Compose
            || request.composition != last.composition
            || request.before != last.before
            || request.after != last.after
        {
            return None;
        }
        if request.letters.len() < last.letters.len() {
            return Some(SkipReason::Deletion);
        }
        let grown = request.letters.len() - last.letters.len();
        (request.letters.starts_with(&last.letters) && grown < self.min_growth)
            .then_some(SkipReason::TooLittleGrowth)
    }
```

`crates/qingjian-predict/src/cloud_predictor.rs`：`Dispatcher::new(superseded.clone())` → `Dispatcher::new(config, superseded.clone())`。

`apps/cli/src/predict_sim/mod.rs`：`Dispatcher::new(superseded)` → `Dispatcher::new(config, superseded)`。

- [ ] **Step 3: 跑测试与模拟**

```bash
cargo test -p qingjian-predict
cargo build --release -p qingjian-cli
for p in 0 0.1 0.2; do target/release/qingjian-cli --config tools/eval/offline.toml --predict-sim data/eval/input-log-2026-10-04.jsonl --predict-sim-pause $p 2>/dev/null | tail -2; done
```

Expected：测试全过；「防抖之后」多出「退格 ≈15 / 增量不足」（p 越大增量不足越多：我量到 2 / 39 / 75）。记下三个数。

- [ ] **Step 4: 提交**

```bash
cargo fmt --all && cargo clippy --all-targets -- -D warnings
git add crates apps
git commit -m "feat(predict): 同一段组句里退格、只多一两个字母时不再发联想" -m "R3（WeType DropLast 只作废不发）、R4（多 3 个字母才再问，对应 WeType 回包预填续打结果省下的那几次）。
放在防抖之后的 Dispatcher，与「真正发出去的那次」比；Engine 看不到防抖，在 Core 判会把正常停顿也吞掉。
--predict-sim 实发（p=0/0.1/0.2）：<上次> → <这次>。"
```

---

## Task 6: R6 缓存键去掉易变字段、容量 256

**Files:**
- Modify: `crates/qingjian-predict/src/cache.rs:28-43`、`crates/qingjian-predict/src/dispatcher/{mod.rs,tests.rs}`

- [ ] **Step 1: 写失败测试**

`crates/qingjian-predict/src/dispatcher/tests.rs` 末尾加：

```rust

#[test]
fn cache_key_ignores_local_candidates() {
    let (mut dispatcher, _) = dispatcher(&PredictConfig::default());
    let now = Instant::now();
    let first = request(1, "suoyiwoxiang");
    assert_eq!(dispatcher.decide(&first, now), Decision::Send);
    dispatcher.note_reply(&first, &reply());
    // 候选随学习变了、本地切分变了：仍是同一个请求
    let mut again = request(2, "suoyiwoxiang");
    again.pinyin = "suo'yi'wo'xiang".into();
    assert_ne!(again.candidates, first.candidates);
    assert_eq!(dispatcher.decide(&again, now), Decision::Cached);
    assert_eq!(dispatcher.cached(&again), Some(&reply()));
}
```

Run: `cargo test -p qingjian-predict cache_key_ignores`
Expected: FAIL：期望 `Cached`，实际 `Skip(TooLittleGrowth)`（旧键含候选「候选1」/「候选2」与切分，缓存没命中，落到 R4）。

- [ ] **Step 2: 实现**

`crates/qingjian-predict/src/cache.rs` 的 `key` 换成：

```rust
    /// 序号不参与：同样的上下文与字母就是同一个请求。本地候选（随学习变）与本地切分（由字母定）也不参与，
    /// 不然退格再打回同样的字母永远命中不了（WeType 也是去掉易变字段再当键，`EraseReqDuplicatedInfo`）。
    pub fn key(request: &PredictionRequest) -> String {
        format!(
            "{:?}\u{1}{}\u{1}{}\u{1}{}\u{1}{}\u{1}{}\u{1}{}\u{1}{}",
            request.kind,
            request.before,
            request.after,
            request.letters,
            request.max_items,
            request.want_sentence,
            request.text,
            request.target_language
        )
    }
```

`crates/qingjian-predict/src/dispatcher/mod.rs`：`const CACHE_CAPACITY: usize = 64;` → `256`。

- [ ] **Step 3: 跑测试与模拟、提交**

```bash
cargo test -p qingjian-predict
cargo build --release -p qingjian-cli
for p in 0 0.1 0.2; do target/release/qingjian-cli --config tools/eval/offline.toml --predict-sim data/eval/input-log-2026-10-04.jsonl --predict-sim-pause $p 2>/dev/null | tail -2; done
cargo fmt --all && cargo clippy --all-targets -- -D warnings
git add crates
git commit -m "feat(predict): 联想缓存键去掉本地候选与切分，容量 64 → 256" -m "R6：本地候选随学习变、切分由字母定，留在键里同一输入永远命中不了。缓存只在内存，不落盘（请求带前文）。
--predict-sim 实发（p=0/0.1/0.2）：<上次> → <这次>（模拟里候选不变，这一步主要体现在真机）。"
```

Expected：测试全过；模拟数与 Task 5 基本相同（模拟不学习，候选不变，真机才看得出）。

---

## Task 7: R5 上次的整句补全还盖得住就复用

用户接着敲的字母仍落在上一次整句补全的读音里（`suoyiwoxiangq` → 所以我想去吃饭，再敲 `u`），直接复用，不再问。

**Files:**
- Create: `crates/qingjian-core/src/engine/prediction/cover.rs`
- Modify: `crates/qingjian-core/src/engine/prediction/{gate.rs,mod.rs}`、`crates/qingjian-core/src/engine/tests/cloud_gate.rs`

- [ ] **Step 1: 写失败测试**

`crates/qingjian-core/src/engine/tests/cloud_gate.rs` 末尾加：

```rust

#[test]
fn a_covering_sentence_is_reused_without_asking_again() {
    let (mut engine, probe) = gated(PredictionPolicy::default());
    let first = ask(&mut engine, "suoyiwoxiangq").unwrap();
    probe.replies.lock().unwrap().push(Prediction {
        sequence: first,
        words: Vec::new(),
        sentence: Some("所以我想去吃饭".into()),
    });
    assert_eq!(
        engine.poll_prediction().unwrap().sentence.as_deref(),
        Some("所以我想去吃饭")
    );
    let second = ask(&mut engine, "suoyiwoxiangqu").unwrap();
    assert_eq!(engine.prediction_skip(), Some(SkipReason::Reused));
    let reused = engine.poll_prediction().unwrap();
    assert_eq!(reused.sequence, second);
    assert_eq!(reused.sentence.as_deref(), Some("所以我想去吃饭"));
    assert_eq!(probe.submitted.lock().unwrap().len(), 1);
    // 「去」读 qu，盖不住 m：照常发
    assert!(ask(&mut engine, "suoyiwoxiangm").is_some());
    assert_eq!(engine.prediction_skip(), None);
    assert_eq!(probe.submitted.lock().unwrap().len(), 2);
}
```

Run: `cargo test -p qingjian-core a_covering_sentence`
Expected: FAIL：第二次 `prediction_skip()` 是 `None`（照常发了）。

- [ ] **Step 2: `covers`**

新建 `crates/qingjian-core/src/engine/prediction/cover.rs`（Task 11 会整个换成带 `converts` 的版本）：

```rust
//! 整句补全的读音能不能盖住敲的字母：R5（上次的整句仍适用就复用、不再发）用。

use crate::parser;

/// 整句超过这么多字不判（逐字查读音在按键路径上，组句通常不到 20 字）。
const MAX_CHARS: usize = 32;

/// 最长的音节（`zhuang`）。
const MAX_SYLLABLE: usize = 6;

/// `sentence` 从头逐字读下去能不能盖住 `letters`：每个字吃掉一个它读得出的完整音节，字母吃完即算
/// （最后一个音节允许只敲了开头），句子余下的字是续写。`reads(字, 音节)` 是「这个字有这个读音」。
pub fn covers(sentence: &str, letters: &str, reads: impl Fn(&str, &str) -> bool) -> bool {
    let chars: Vec<String> = sentence.chars().map(String::from).collect();
    if chars.is_empty() || chars.len() > MAX_CHARS || letters.is_empty() || !letters.is_ascii() {
        return false;
    }
    step(&chars, letters, &reads)
}

fn step(chars: &[String], rest: &str, reads: &impl Fn(&str, &str) -> bool) -> bool {
    if rest.is_empty() {
        return true;
    }
    let Some((first, others)) = chars.split_first() else {
        return false;
    };
    if parser::SYLLABLES
        .iter()
        .any(|s| s.len() > rest.len() && s.starts_with(rest) && reads(first, s))
    {
        return true;
    }
    (1..=rest.len().min(MAX_SYLLABLE)).rev().any(|n| {
        let syllable = &rest[..n];
        parser::is_syllable(syllable) && reads(first, syllable) && step(others, &rest[n..], reads)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reads(ch: &str, syllable: &str) -> bool {
        let table = [
            ("所", "suo"),
            ("以", "yi"),
            ("我", "wo"),
            ("想", "xiang"),
            ("去", "qu"),
            ("吃", "chi"),
            ("饭", "fan"),
        ];
        table.iter().any(|(c, s)| *c == ch && *s == syllable)
    }

    #[test]
    fn sentence_covers_its_own_prefix_with_a_partial_last_syllable() {
        assert!(covers("所以我想去吃饭", "suoyiwoxiangq", reads));
        assert!(covers("所以我想去吃饭", "suoyiwoxiangqu", reads));
        assert!(covers("所以我想去吃饭", "suoyiwoxiangquchif", reads));
    }

    #[test]
    fn a_different_letter_breaks_the_cover() {
        assert!(!covers("所以我想去吃饭", "suoyiwoxiangm", reads));
        assert!(!covers("所以我想去吃饭", "suoyiwoxiangquchifanle", reads));
    }
}
```

`crates/qingjian-core/src/engine/prediction/mod.rs`：`mod cloud_word;` 后加 `mod cover;`；`pub use cloud_word::CloudWord;` 后加 `use cover::covers;`。

- [ ] **Step 3: 门槛状态记提交与回复**

`crates/qingjian-core/src/engine/prediction/gate.rs`：

1. 导入改成：

```rust
use super::response::Prediction;
use super::skip_reason::SkipReason;
```

2. 结构体开头（`skip` 字段之前）加：

```rust
    /// 最近一次提交的组句请求：(序号, 字母, 前文)。回复按序号对上了才知道它对应哪段字母。
    submitted: Option<(u64, String, String)>,

    /// 上一次到达的整句补全：(字母, 前文, 整句)。
    replied: Option<(String, String, String)>,

```

   `skip` 字段之后加：

```rust

    /// R5 复用出来的结果，`poll_prediction` 先吐它。
    pub(crate) reused: Option<Prediction>,
```

3. `impl` 里 `composition()` 之后加：

```rust

    pub(crate) fn note_submitted(&mut self, sequence: u64, letters: &str, before: &str) {
        self.submitted = Some((sequence, letters.to_owned(), before.to_owned()));
    }

    /// 回复到了：记下它的整句（没有整句就清掉，免得复用更早的）。
    pub(crate) fn note_reply(&mut self, sequence: u64, sentence: Option<&str>) {
        let Some((submitted, letters, before)) = &self.submitted else {
            return;
        };
        if *submitted != sequence {
            return;
        }
        self.replied = sentence.map(|s| (letters.clone(), before.clone(), s.to_owned()));
    }

    /// 上次的整句，若新字母是它那次字母的延长、前文没变；盖不盖得住由调用方按读音判。
    pub(crate) fn reusable(&self, letters: &str, before: &str) -> Option<&str> {
        let (replied, replied_before, sentence) = self.replied.as_ref()?;
        (before == replied_before && letters.starts_with(replied.as_str()))
            .then_some(sentence.as_str())
    }
```

4. `reset` 换成：

```rust
    /// 组句作废（清空、上屏、换应用）：之前的来往都不再有意义。
    pub(crate) fn reset(&mut self) {
        self.submitted = None;
        self.replied = None;
        self.reused = None;
        self.composition += 1;
    }
```

- [ ] **Step 4: Engine 接线**

`crates/qingjian-core/src/engine/prediction/mod.rs`：

1. `request_prediction` 里 Task 4 加的

```rust
        if !question && let Some(reason) = self.gate_skip(&letters, pinyin_source, &policy) {
            self.prediction_gate.skip = Some(reason);
            return None;
        }
```

换成

```rust
        if !question {
            if let Some(reason) = self.gate_skip(&letters, pinyin_source, &policy) {
                self.prediction_gate.skip = Some(reason);
                return None;
            }
            if let Some(sentence) = self.reusable_sentence(&letters, &before, &policy) {
                self.prediction_gate.skip = Some(SkipReason::Reused);
                self.prediction_gate.reused = Some(Prediction {
                    sequence: self.prediction_sequence,
                    words: Vec::new(),
                    sentence: Some(sentence),
                });
                self.last_prediction_kind = kind;
                self.last_prediction_scope = scope.to_owned();
                return Some(self.prediction_sequence);
            }
        }
```

2. 函数末尾 `self.predictor.submit(request);` 之前加：

```rust
        if kind == PredictionKind::Compose {
            self.prediction_gate.note_submitted(
                request.sequence,
                &request.letters,
                &request.before,
            );
        }
```

3. `bump_prediction_sequence` 末尾加 `self.prediction_gate.reused = None;`
4. `gate_skip` 之后加：

```rust

    /// R5：上次回来的整句补全仍盖得住新敲的字母（逐字读音对得上，末音节可只敲开头），就用它，不再发。
    fn reusable_sentence(
        &self,
        letters: &str,
        before: &str,
        policy: &PredictionPolicy,
    ) -> Option<String> {
        if !policy.sentence {
            return None;
        }
        let sentence = self.prediction_gate.reusable(letters, before)?;
        covers(sentence, letters, |ch, syllable| {
            self.char_reads(ch, syllable)
        })
        .then(|| sentence.to_owned())
    }
```

5. `poll_prediction` 开头

```rust
        while let Some(mut prediction) = self.predictor.poll() {
            if prediction.sequence == self.prediction_sequence {
```

换成

```rust
        while let Some(mut prediction) = self
            .prediction_gate
            .reused
            .take()
            .or_else(|| self.predictor.poll())
        {
            if prediction.sequence == self.prediction_sequence {
                if self.last_prediction_kind == PredictionKind::Compose {
                    self.prediction_gate
                        .note_reply(prediction.sequence, prediction.sentence.as_deref());
                }
```

（`note_reply` 要在繁体转换之前：复用时逐字查的是简体读音。）

- [ ] **Step 5: 跑测试、模拟、提交**

```bash
cargo test -p qingjian-core
cargo build --release -p qingjian-cli && target/release/qingjian-cli --config tools/eval/offline.toml --predict-sim data/eval/input-log-2026-10-04.jsonl 2>/dev/null | tail -2
cargo fmt --all && cargo clippy --all-targets -- -D warnings
git add crates
git commit -m "feat(core): 上次的整句补全仍盖得住新敲的字母时直接复用，不再问云端" -m "R5（WeType 的对应物是服务端回包预填续打结果）：逐字查词库读音，末音节允许只敲开头，不容错——宁可多问一次。
复用时云端词丢掉（它们对应的是更短的字母串）。模拟没有回包内容，这一条的收益只在真机日志里看得到。"
```

Expected：测试全过；模拟数与 Task 6 相同（模拟不回包）。

---

## Task 8: R8 令牌桶（缺省关）、R9 429 锁定、R10 锁后抬高防抖

**Files:**
- Create: `crates/qingjian-predict/src/rate.rs`
- Modify: `crates/qingjian-predict/src/{dispatcher/mod.rs,dispatcher/tests.rs,error.rs,worker.rs,lib.rs}`

- [ ] **Step 1: 写失败测试**

`crates/qingjian-predict/src/dispatcher/tests.rs`：导入第三行改成 `use std::time::{Duration, Instant};`，末尾加：

```rust

#[test]
fn rate_limit_drops_the_seventh_request() {
    let config = PredictConfig {
        rate_burst: 6,
        ..PredictConfig::default()
    };
    let (mut dispatcher, _) = dispatcher(&config);
    let now = Instant::now();
    let words = ["aaaa", "bbbb", "cccc", "dddd", "eeee", "ffff", "gggg"];
    let decisions: Vec<Decision> = words
        .iter()
        .enumerate()
        .map(|(i, letters)| dispatcher.decide(&request(i as u64 + 1, letters), now))
        .collect();
    assert!(decisions[..6].iter().all(|d| *d == Decision::Send));
    assert_eq!(decisions[6], Decision::Skip(SkipReason::RateLimited));
}

#[test]
fn rate_limited_status_locks_and_raises_debounce() {
    let (mut dispatcher, _) = dispatcher(&PredictConfig::default());
    let start = Instant::now();
    dispatcher.lock(start);
    let mut request = request(1, "suoyiwoxiang");
    for seconds in [0, 10, 29] {
        assert_eq!(
            dispatcher.decide(&request, start + Duration::from_secs(seconds)),
            Decision::Skip(SkipReason::Locked)
        );
    }
    request.sequence = 2;
    assert_eq!(
        dispatcher.decide(&request, start + Duration::from_secs(30)),
        Decision::Send
    );
    let base = Duration::from_millis(300);
    assert_eq!(
        dispatcher.debounce(base, start + Duration::from_secs(60)),
        Duration::from_millis(600)
    );
    assert_eq!(dispatcher.debounce(base, start + Duration::from_secs(601)), base);
}
```

Run: `cargo test -p qingjian-predict dispatcher`
Expected: 编译失败（没有 `lock` / `debounce`）。

- [ ] **Step 2: 令牌桶**

新建 `crates/qingjian-predict/src/rate.rs`：

```rust
//! 令牌桶（R8）：最多连发 `burst` 次，之后每 `refill` 补一个。`burst == 0` 是不限流。

use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct RateLimiter {
    /// 桶容量；0 不限流。
    burst: u32,

    /// 补一个令牌的间隔。
    refill: Duration,

    /// 现有令牌，带小数：按经过的时间连续补。
    tokens: f64,

    /// 上次补令牌的时刻；第一次取之前为 `None`（桶是满的）。
    last: Option<Instant>,
}

impl RateLimiter {
    pub fn new(burst: u32, refill: Duration) -> Self {
        Self {
            burst,
            refill,
            tokens: f64::from(burst),
            last: None,
        }
    }

    /// 取一个令牌；没有返回 `false`，不排队——过时的联想没有价值。
    pub fn try_take(&mut self, now: Instant) -> bool {
        if self.burst == 0 {
            return true;
        }
        if let Some(last) = self.last
            && !self.refill.is_zero()
        {
            let gained =
                now.saturating_duration_since(last).as_secs_f64() / self.refill.as_secs_f64();
            self.tokens = (self.tokens + gained).min(f64::from(self.burst));
        }
        self.last = Some(now);
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn burst_then_refill() {
        let start = Instant::now();
        let mut rate = RateLimiter::new(6, Duration::from_secs(5));
        for _ in 0..6 {
            assert!(rate.try_take(start));
        }
        assert!(!rate.try_take(start));
        assert!(rate.try_take(start + Duration::from_secs(5)));
        assert!(!rate.try_take(start + Duration::from_secs(5)));
    }

    #[test]
    fn zero_burst_never_limits() {
        let now = Instant::now();
        let mut rate = RateLimiter::new(0, Duration::from_secs(5));
        assert!((0..100).all(|_| rate.try_take(now)));
    }
}
```

`crates/qingjian-predict/src/lib.rs`：`mod provider;` 后加 `mod rate;`。

- [ ] **Step 3: Dispatcher 接限流与锁定**

`crates/qingjian-predict/src/dispatcher/mod.rs`：

1. 导入改成：

```rust
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use qingjian_core::{PredictionKind, PredictionRequest, SkipReason, prediction_defaults};

use crate::cache::PredictionCache;
use crate::config::PredictConfig;
use crate::prompt::Reply;
use crate::rate::RateLimiter;
```

2. 结构体 `last_sent` 字段之后加：

```rust

    /// 令牌桶（R8）。
    rate: RateLimiter,

    /// 服务商 429 之后锁到什么时候（R9）。
    locked_until: Option<Instant>,

    /// 最近一次被 429 锁的时刻，用来抬高防抖（R10）。
    last_lock: Option<Instant>,
```

3. `new` 里 `last_sent: None,` 之后加：

```rust
            rate: RateLimiter::new(
                config.rate_burst,
                Duration::from_millis(config.rate_refill_ms),
            ),
            locked_until: None,
            last_lock: None,
```

4. `decide` 签名的 `_now` 改回 `now`，文档改成「返回 [`Decision::Send`] 时已记成「上次发出的」并扣了令牌。」；`growth_skip` 判断之后、`if request.kind == PredictionKind::Compose` 之前加：

```rust
        if self.locked_until.is_some_and(|until| now < until) {
            return Decision::Skip(SkipReason::Locked);
        }
        if !self.rate.try_take(now) {
            return Decision::Skip(SkipReason::RateLimited);
        }
```

5. `note_reply` 之后加：

```rust

    /// 服务商回了 429：锁一段时间（拿不到 `Retry-After`，按 [`prediction_defaults::LOCK_SECS`]）。
    pub fn lock(&mut self, now: Instant) {
        let until = now + Duration::from_secs(prediction_defaults::LOCK_SECS);
        self.locked_until = Some(self.locked_until.map_or(until, |old| old.max(until)));
        self.last_lock = Some(now);
    }

    /// 这一轮防抖用多久：被 429 锁过的一段时间内抬到 [`prediction_defaults::LOCKED_DEBOUNCE_MS`]（R10）。
    pub fn debounce(&self, base: Duration, now: Instant) -> Duration {
        let cooling = self.last_lock.is_some_and(|at| {
            now.saturating_duration_since(at)
                < Duration::from_secs(prediction_defaults::LOCK_COOLDOWN_SECS)
        });
        if cooling {
            base.max(Duration::from_millis(prediction_defaults::LOCKED_DEBOUNCE_MS))
        } else {
            base
        }
    }
```

- [ ] **Step 4: 认出 429，后台线程锁定与抬防抖**

`crates/qingjian-predict/src/error.rs` 末尾加：

```rust

impl PredictError {
    /// 服务商回了 429（请求太频繁）。`async-openai` 只在回包是 JSON 错误体时保留状态码，非 JSON 的 429 认不出来。
    pub fn is_rate_limited(&self) -> bool {
        matches!(self, Self::Api(async_openai::error::OpenAIError::ApiError(response)) if response.status_code.as_u16() == 429)
    }
}
```

`crates/qingjian-predict/src/worker.rs`：

1. 字段 `debounce` 的注释改成 `/// 防抖窗口（配置值；429 之后 [`Dispatcher::debounce`] 会临时抬高）。`
2. `Err(error) => { tracing::warn!(…) }` 换成：

```rust
                Err(error) => {
                    if error.is_rate_limited() {
                        self.dispatcher.lock(Instant::now());
                    }
                    tracing::warn!(sequence = request.sequence, %error, "联想失败");
                }
```

3. `fn debounce` 换成：

```rust
    /// 防抖：在窗口内持续收到新请求就一直等，只保留最后一个。发送端关闭返回 `None`。
    fn debounce(&self, mut latest: PredictionRequest) -> Option<PredictionRequest> {
        let window = self.dispatcher.debounce(self.debounce, Instant::now());
        loop {
            match self.requests.recv_timeout(window) {
                Ok(newer) => latest = newer,
                Err(RecvTimeoutError::Timeout) => return Some(latest),
                Err(RecvTimeoutError::Disconnected) => return None,
            }
        }
    }
```

- [ ] **Step 5: 跑测试；限流开着时的模拟（只看，不改缺省）**

```bash
cargo test -p qingjian-predict
cargo build --release -p qingjian-cli
target/release/qingjian-cli --config tools/eval/offline.toml --predict-sim data/eval/input-log-2026-10-04.jsonl 2>/dev/null | tail -2
printf '[predict]\nenabled = false\nrate_burst = 6\n' > /tmp/rate.toml
target/release/qingjian-cli --config /tmp/rate.toml --predict-sim data/eval/input-log-2026-10-04.jsonl 2>/dev/null | tail -2
```

Expected：测试全过；缺省配置实发与 Task 7 相同（令牌桶关）；`rate_burst = 6` 时多出「限流 N」——模拟时钟里组句间没有空闲时间，这个数偏大，只说明开关生效。

- [ ] **Step 6: 提交**

```bash
cargo fmt --all && cargo clippy --all-targets -- -D warnings
git add crates
git commit -m "feat(predict): 可选令牌桶限流（缺省关），服务商回 429 时停 30 秒并抬高防抖" -m "R8 令牌桶 [predict] rate_burst / rate_refill_ms，0 关（用户定缺省关）。
R9/R10 是对服务商限流的被动响应，始终开：async-openai 不给响应头，拿不到 Retry-After，固定 30 秒；锁过之后 10 分钟内防抖至少 600ms。"
```

---

## Task 9: R11 按需模式与「现在问云端」

**Files:**
- Modify: `crates/qingjian-core/src/engine/prediction/{gate.rs,mod.rs}`、`crates/qingjian-core/src/engine/tests/cloud_gate.rs`

- [ ] **Step 1: 写失败测试**

`crates/qingjian-core/src/engine/tests/cloud_gate.rs` 末尾加：

```rust

#[test]
fn manual_trigger_only_sends_on_the_shortcut() {
    let (mut engine, probe) = gated(PredictionPolicy {
        trigger: PredictionTrigger::Manual,
        ..PredictionPolicy::default()
    });
    assert_eq!(ask(&mut engine, "suoyiwoxiang"), None);
    assert_eq!(engine.prediction_skip(), Some(SkipReason::Manual));
    engine.set_input("nih");
    assert!(engine.request_prediction_now(None, &[]).is_some());
    assert_eq!(probe.submitted.lock().unwrap().len(), 1);
    // 快捷键也不越过私密
    engine.set_private(true);
    assert_eq!(engine.request_prediction_now(None, &[]), None);
}
```

Run: `cargo test -p qingjian-core manual_trigger`
Expected: 编译失败（没有 `request_prediction_now`）。

- [ ] **Step 2: 实现**

`crates/qingjian-core/src/engine/prediction/gate.rs`：`skip` 字段之后加

```rust

    /// `request_prediction_now`：这一次绕过 R1 / R2 / R5 / R11。
    pub(crate) forced: bool,
```

`crates/qingjian-core/src/engine/prediction/mod.rs`：

1. `request_prediction` 里 `if !question {`（Task 7 写的）改成 `if !question && !self.prediction_gate.forced {`。
2. `gate_skip` 函数体开头加（文档改成「R11 / R1 / R2：按需模式不发；字母太少不发；短而完整的全拼不发（本地首选够用）。」）：

```rust
        if policy.trigger == PredictionTrigger::Manual {
            return Some(SkipReason::Manual);
        }
```

3. `request_prediction` 函数之后加：

```rust

    /// 快捷键「现在问云端」（`[shortcut] predict_now`）：绕过字母数、短全拼、复用与按需模式（R1 / R2 / R5 / R11），
    /// 私密、辅码、英文这些照旧不发。
    pub fn request_prediction_now(
        &mut self,
        surrounding: Option<SurroundingText>,
        candidates: &[Candidate],
    ) -> Option<u64> {
        self.prediction_gate.forced = true;
        let sequence = self.request_prediction(surrounding, candidates);
        self.prediction_gate.forced = false;
        sequence
    }
```

- [ ] **Step 3: 跑测试、按需模式的模拟、提交**

```bash
cargo test -p qingjian-core
cargo build --release -p qingjian-cli
printf '[predict]\nenabled = false\ntrigger = "manual"\n' > /tmp/manual.toml
target/release/qingjian-cli --config /tmp/manual.toml --predict-sim data/eval/input-log-2026-10-04.jsonl 2>/dev/null
cargo fmt --all && cargo clippy --all-targets -- -D warnings
git add crates
git commit -m "feat(core): 按需模式只在快捷键时问云端，request_prediction_now 绕过门槛" -m "R11（#177 用户诉求）：[predict] trigger = \"manual\"。快捷键入口绕过 R1/R2/R5/R11，不绕过私密、辅码、英文。
问字模式不受按需模式影响（它本身就是主动发问）。"
```

Expected：测试全过；按需模式下「门槛：按需模式 N」，实发 0。

---

## Task 10: `engine/late/`——晚到的云端词插在哪由 Core 定

规则：候选窗画出来 200 ms 内到、用户没动过高亮 → 插第 2 位、最多 1 条；之后到的（或动过高亮）→ 插在前 3 个本地候选之后（第 4 位起，最多 `slots` 条）；高亮已经落在要被挤动的位置上 → 这次不插；用户固定位置（`Custom`）的词不让；问字（没有本地候选）→ 整页给答案。

**Files:**
- Create: `crates/qingjian-core/src/engine/late/{outcome.rs,placement.rs,state.rs,merge/mod.rs,merge/tests.rs}`
- Modify: `crates/qingjian-core/src/engine/late/mod.rs`、`crates/qingjian-core/src/engine/mod.rs`、`crates/qingjian-core/src/engine/prediction/mod.rs`（`bump_prediction_sequence`）、`crates/qingjian-core/src/lib.rs`
- Modify: `crates/qingjian-core/src/candidate/layout/candidate_layout.rs`
- Modify: `crates/qingjian-platform/src/config/mod.rs`（模板 `slots` 注释）

- [ ] **Step 1: 写插位纯函数的失败测试**

新建 `crates/qingjian-core/src/engine/late/merge/tests.rs`：

```rust
use std::time::{Duration, Instant};

use crate::candidate::{Candidate, CandidateKind};

use super::super::outcome::LateOutcome;
use super::super::policy::LatePolicy;
use super::place_words;

fn local(text: &str) -> Candidate {
    Candidate {
        text: text.into(),
        kind: CandidateKind::Chinese,
        syllables: Vec::new(),
        reading: None,
        translation: None,
        aux_code: None,
    }
}

fn cloud(text: &str) -> Candidate {
    Candidate {
        kind: CandidateKind::Cloud,
        ..local(text)
    }
}

fn shown() -> Vec<Candidate> {
    ["a", "b", "c", "d", "e"].into_iter().map(local).collect()
}

fn texts(words: &[Candidate]) -> Vec<&str> {
    words.iter().map(|c| c.text.as_str()).collect()
}

/// 画出来 `after_ms` 毫秒后到达；`None` 是还没画出来。
fn place(
    local: &[Candidate],
    words: &[&str],
    after_ms: Option<u64>,
    touched: Option<usize>,
) -> (usize, Vec<String>, LateOutcome) {
    let start = Instant::now();
    let shown_at = after_ms.map(|_| start);
    let now = start + Duration::from_millis(after_ms.unwrap_or(0));
    let (at, words, outcome) = place_words(
        local,
        words.iter().map(|w| cloud(w)).collect(),
        shown_at,
        touched,
        &LatePolicy::default(),
        2,
        now,
    );
    (
        at,
        texts(&words).into_iter().map(String::from).collect(),
        outcome,
    )
}

#[test]
fn within_grace_one_word_goes_second() {
    assert_eq!(
        place(&shown(), &["X", "Y"], Some(50), None),
        (1, vec!["X".into()], LateOutcome::InGrace)
    );
}

#[test]
fn late_words_go_after_the_frozen_three() {
    assert_eq!(
        place(&shown(), &["X", "Y"], Some(500), None),
        (3, vec!["X".into(), "Y".into()], LateOutcome::AfterFrozen)
    );
}

#[test]
fn touched_counts_as_late_and_never_moves_the_highlighted_word() {
    assert_eq!(
        place(&shown(), &["X"], Some(50), Some(1)),
        (3, vec!["X".into()], LateOutcome::AfterFrozen)
    );
    assert_eq!(
        place(&shown(), &["X"], Some(50), Some(4)),
        (3, Vec::new(), LateOutcome::Skipped)
    );
}

#[test]
fn duplicates_of_local_words_are_dropped() {
    assert_eq!(
        place(&shown(), &["c"], Some(500), None),
        (3, Vec::new(), LateOutcome::Empty)
    );
}

#[test]
fn not_yet_drawn_counts_as_within_grace() {
    assert_eq!(
        place(&shown(), &["X"], None, None),
        (1, vec!["X".into()], LateOutcome::InGrace)
    );
}

#[test]
fn fixed_positions_stay_put() {
    let mut list = vec![local("a"), local("F")];
    list[1].kind = CandidateKind::Custom(2);
    list.extend([local("b"), local("c")]);
    assert_eq!(
        place(&list, &["X"], Some(50), None),
        (2, vec!["X".into()], LateOutcome::InGrace)
    );
}

#[test]
fn question_answers_fill_an_empty_list() {
    assert_eq!(
        place(&[], &["森", "淼", "焱"], Some(500), None),
        (
            0,
            vec!["森".into(), "淼".into(), "焱".into()],
            LateOutcome::InGrace
        )
    );
}
```

这个文件要等 Step 2 的 `merge/mod.rs` 声明 `mod tests;` 才会被编译；Step 2 写完后再跑：

Run: `cargo test -p qingjian-core late`
Expected（Step 2 之后）: `late::merge::tests` 7 个用例 PASS。先把 Step 2 的 `place_words` 写成 `todo!()` 跑一次能看到 7 个 FAIL，再填实现。

- [ ] **Step 2: 类型与纯函数**

新建 `crates/qingjian-core/src/engine/late/outcome.rs`：

```rust
//! 晚到的云端结果落到了哪条插位规则上。

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum LateOutcome {
    /// 没有可插的（都和本地候选重复，或不要云端词）。
    #[default]
    Empty,

    /// 时限内：插在第 2 位，最多 `max_in_grace` 条。
    InGrace,

    /// 超时或用户动过高亮：插在前 `frozen` 个本地候选之后。
    AfterFrozen,

    /// 用户的高亮已经在要被挤动的位置上：这次不插，免得选中的词换了。
    Skipped,
}
```

新建 `crates/qingjian-core/src/engine/late/placement.rs`：

```rust
//! 一次云端结果摆在哪：壳照着把词插进候选、把整句放到第一位。

use crate::candidate::Candidate;

use super::outcome::LateOutcome;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CloudPlacement {
    /// 云端词插在本地候选的第几个之前（0 起，按本地候选数夹住）。
    pub at: usize,

    /// 要插的云端词，已与本地候选去重、按规则截断。
    pub words: Vec<Candidate>,

    /// 排到第一位的云整句（`sentence_first`）；上屏照常调 `Engine::commit`，Core 认得出它。
    pub top: Option<Candidate>,

    /// 留给 `Tab` 的整句补全；排到第一位时为 `None`。
    pub sentence: Option<String>,

    /// 落到了哪条规则上（日志与测试用）。
    pub outcome: LateOutcome,
}

impl CloudPlacement {
    /// 候选列表不用动（整句补全另算）。
    pub fn is_empty(&self) -> bool {
        self.words.is_empty() && self.top.is_none()
    }
}
```

新建 `crates/qingjian-core/src/engine/late/state.rs`（Task 11 加 `top` 字段）：

```rust
//! 当前这一版候选的显示状态：壳报告画出来的时刻，以及排到第一位的云整句。只由壳的报告驱动，Core 不碰 UI。

use std::time::Instant;

#[derive(Debug, Clone, Default)]
pub(crate) struct LateState {
    /// 壳第一次报告这一版候选画出来的时刻；`None` 是还没画（结果比画面先到，按时限内处理）。
    pub(crate) shown_at: Option<Instant>,
}
```

新建 `crates/qingjian-core/src/engine/late/merge/mod.rs`：

```rust
//! 晚到插位的纯函数：给定已显示的本地候选、到达的云端词与显示状态，算出插在哪、插几条。

use std::time::{Duration, Instant};

use crate::candidate::{Candidate, CandidateKind};

use super::outcome::LateOutcome;
use super::policy::LatePolicy;

/// 输入：`local` 是壳当前显示的本地候选（顺序即显示顺序），`words` 是校验过的云端词，
/// `shown_at` 是这版候选画出来的时刻，`touched` 是用户移动过高亮时高亮所在的下标（在整个列表里）。
/// 返回 (插在第几个本地候选之前, 要插的词, 规则)。
pub(crate) fn place_words(
    local: &[Candidate],
    words: Vec<Candidate>,
    shown_at: Option<Instant>,
    touched: Option<usize>,
    policy: &LatePolicy,
    slots: usize,
    now: Instant,
) -> (usize, Vec<Candidate>, LateOutcome) {
    let mut fresh: Vec<Candidate> = Vec::new();
    for mut word in words {
        if local.iter().any(|c| c.text == word.text) || fresh.iter().any(|c| c.text == word.text) {
            continue;
        }
        word.kind = CandidateKind::Cloud;
        fresh.push(word);
    }
    // 问字模式没有本地候选：答案就是整个列表
    if local.is_empty() {
        let outcome = if fresh.is_empty() {
            LateOutcome::Empty
        } else {
            LateOutcome::InGrace
        };
        return (0, fresh, outcome);
    }
    let in_grace = touched.is_none()
        && shown_at.is_none_or(|shown| {
            now.saturating_duration_since(shown)
                <= Duration::from_millis(u64::from(policy.grace_ms))
        });
    let (at, cap, outcome) = if in_grace {
        (1, policy.max_in_grace, LateOutcome::InGrace)
    } else {
        (policy.frozen, slots, LateOutcome::AfterFrozen)
    };
    // 用户固定位置的词（自定义短语）不让：插在最后一个固定位置之后
    let fixed = local
        .iter()
        .filter_map(|c| match c.kind {
            CandidateKind::Custom(position) => Some(position),
            _ => None,
        })
        .max()
        .unwrap_or(0);
    let at = at.max(fixed).min(local.len());
    fresh.truncate(cap.min(slots));
    if fresh.is_empty() {
        return (at, fresh, LateOutcome::Empty);
    }
    if touched.is_some_and(|highlighted| highlighted >= at) {
        return (at, Vec::new(), LateOutcome::Skipped);
    }
    (at, fresh, outcome)
}

#[cfg(test)]
mod tests;
```

- [ ] **Step 3: Engine 接口**

`crates/qingjian-core/src/engine/late/mod.rs` 整个换成（Task 11 再加整句排第一）：

```rust
//! 云端结果晚到后怎么并进已显示的候选：时限内插第 2 位，超时或用户动过之后从第 `frozen + 1` 位起插，
//! 用户固定位置的词不动；可选把纯转换的云整句排到第一。规则在 Core，各壳只报「画出来了」并照 [`CloudPlacement`] 摆。
//! 规则与出处：`docs/notes/cloud-request-throttle.md`。
//! 整句 / 词级神经重排的晚到规则（B5 的另一半）还在各壳里，以后并进这里。

mod merge;
mod outcome;
mod placement;
mod policy;
mod state;

pub use outcome::LateOutcome;
pub use placement::CloudPlacement;
pub use policy::LatePolicy;
pub(crate) use state::LateState;

use std::time::Instant;

use crate::candidate::Candidate;

use super::Engine;
use super::prediction::{CloudWord, Prediction};
use merge::place_words;

impl Engine {
    /// 壳：这一版候选画到屏幕上了。每次重画都可以调，只记第一次；新一轮联想请求时清零。
    pub fn note_candidates_shown(&mut self) {
        if self.late.shown_at.is_none() {
            self.late.shown_at = Some(Instant::now());
        }
    }

    /// 云端结果到了，算出摆在哪：`local` 是壳当前显示的本地候选，`touched` 是用户移动过高亮 / 翻过页时高亮在整个列表里的下标。
    /// 翻译选中文字的结果不走这里（壳直接显示译文）。
    pub fn place_prediction(
        &mut self,
        prediction: Prediction,
        local: &[Candidate],
        touched: Option<usize>,
    ) -> CloudPlacement {
        let policy = self.predictor.policy();
        let now = Instant::now();
        let words = prediction
            .words
            .into_iter()
            .map(CloudWord::into_candidate)
            .collect();
        let (at, words, outcome) = place_words(
            local,
            words,
            self.late.shown_at,
            touched,
            &policy.late,
            policy.slots,
            now,
        );
        let placement = CloudPlacement {
            at,
            words,
            top: None,
            sentence: prediction.sentence,
            outcome,
        };
        tracing::debug!(at = placement.at, words = placement.words.len(), top = placement.top.is_some(), outcome = ?placement.outcome, "云端结果插位");
        placement
    }
}
```

`crates/qingjian-core/src/engine/mod.rs`：

1. `pub use late::LatePolicy;` 换成 `pub use late::{CloudPlacement, LateOutcome, LatePolicy};`，下一行加 `use late::LateState;`
2. 结构体 `prediction_gate` 字段之后加：

```rust

    /// 候选的显示状态与排到第一位的云整句（云端结果晚到时插在哪）。
    late: LateState,
```

3. 构造里 `prediction_gate: PredictionGate::default(),` 之后加 `late: LateState::default(),`

`crates/qingjian-core/src/engine/prediction/mod.rs` 的 `bump_prediction_sequence` 末尾加 `self.late = LateState::default();`（新一轮请求 = 新一版候选，画出时刻与排第一的整句都作废）。

`crates/qingjian-core/src/lib.rs` 的 `pub use engine::{…}` 加 `CloudPlacement`、`LateOutcome`。

- [ ] **Step 4: 写 `CandidateLayout` 的失败测试**

`crates/qingjian-core/src/candidate/layout/candidate_layout.rs` 的 `mod tests` 末尾加：

```rust

    #[test]
    fn placed_cloud_words_go_where_core_says_and_the_sentence_leads() {
        let mut layout = CandidateLayout::new(many(5), 9, 2);
        layout.place_cloud(vec![cloud("云0"), cloud("云1")], 3, Some(cloud("整句")));
        assert_eq!(
            texts(&layout.page(0)),
            ["☁整句", "本0", "本1", "本2", "☁云0", "☁云1", "本3", "本4"]
        );
        assert_eq!(layout.len(), 8);
        assert_eq!(layout.candidate(0).unwrap().text, "整句");
        let mut rebuilt = CandidateLayout::new(many(5), 9, 2);
        rebuilt.carry_cloud(&layout);
        assert_eq!(rebuilt.page(0).len(), 8);
        assert_eq!(rebuilt.candidate(4).unwrap().text, "云0");
    }
```

Run: `cargo test -p qingjian-core placed_cloud_words`
Expected: 编译失败（没有 `place_cloud`）。

- [ ] **Step 5: `CandidateLayout` 照 Core 的位置摆**

`crates/qingjian-core/src/candidate/layout/candidate_layout.rs`：

1. 结构体 `slots` 字段之后加：

```rust

    /// 云端词插在第几格前（Core 的 `Engine::place_prediction` 算的）；`None` 是旧规则：第一页末尾。
    cloud_at: Option<usize>,

    /// 排到第一格的云整句（`[predict] sentence_first`）。装箱：布局常被放进枚举（Windows 的 `Composed`），别让它变胖。
    top: Option<Box<Candidate>>,
```

2. `new` 里 `slots,` 之后加 `cloud_at: None,` 与 `top: None,`
3. `/// 本地格数包含最大固定位置之前的空格。` 之前加：

```rust
    /// 照 Core 算好的位置摆云端结果：词插在第 `at` 格前（按格数夹住），整句（若有）放第一格。
    /// 去重与条数 Core 已经做过（`Engine::place_prediction`）。
    pub fn place_cloud(&mut self, words: Vec<Candidate>, at: usize, top: Option<Candidate>) {
        self.cloud = words;
        self.cloud_at = Some(at);
        self.top = top.map(Box::new);
    }

    /// 本地候选重排后换了一份布局：已到的云端结果照原位置带过来。
    pub fn carry_cloud(&mut self, from: &CandidateLayout) {
        self.cloud = from.cloud.clone();
        self.cloud_at = from.cloud_at;
        self.top = from.top.clone();
    }

```

4. `cells()` 与 `len()` 换成：

```rust
    /// 全部格子按索引顺序排开；空位只属于布局。
    pub fn cells(&self) -> Vec<Cell<'_>> {
        let mut cells = self.local_cells();
        let first = match self.cloud_at {
            Some(at) => at.min(cells.len()),
            None => cells
                .len()
                .min(self.page_size.saturating_sub(self.cloud.len())),
        };
        cells.splice(first..first, self.cloud.iter().map(Cell::Cloud));
        if let Some(top) = &self.top {
            cells.insert(0, Cell::Cloud(top));
        }
        cells
    }

    pub fn len(&self) -> usize {
        self.local_len() + self.cloud.len() + usize::from(self.top.is_some())
    }
```

旧的 `set_cloud` / `capacity` 保留不动（Windows 壳在 Task 17 之前还用它；它会把 `cloud_at` 留在 `None`）。为此 `set_cloud` 开头 `self.cloud.clear();` 之后加 `self.cloud_at = None;` 与 `self.top = None;`。

- [ ] **Step 6: 模板注释**

`crates/qingjian-platform/src/config/mod.rs` 模板 `[predict]` 里

```toml
# 云端词到了补进候选窗口第一页末尾几格（比如 2 就是 8、9 两格），前面的本地候选不动；0 表示不要云端词
slots = 2
```

换成

```toml
# 云端词最多插几个（插在哪见下面的 late_grace_ms / late_frozen）；0 表示不要云端词
slots = 2
```

- [ ] **Step 7: 跑测试、提交**

```bash
cargo test -p qingjian-core && cargo test -p qingjian-platform
cargo fmt --all && cargo clippy --all-targets -- -D warnings
git add crates
git commit -m "feat(core): 晚到的云端词插在哪由 Core 定（engine/late），时限内第 2 位、之后第 4 位起" -m "B5 的云端那一半：place_words 纯函数 + Engine::{note_candidates_shown, place_prediction}，CandidateLayout::{place_cloud, carry_cloud}。
已定缺省：超时到的不丢，从第 4 位起插（WeType 缺省丢；青简的 LLM 回包 >1s，丢了等于没有云联想）。
高亮已经落在要被挤动的位置时这次不插；固定位置的词不让。壳还没接，下一步。"
```

Expected：`late::merge::tests` 7 个、`placed_cloud_words_go_where_core_says_and_the_sentence_leads` 与旧的 `CandidateLayout` 测试全过。

---

## Task 11: `sentence_first`——云整句排第一（#341，缺省关）

条件：开关开、用户没动过高亮、本地首选不是固定位置 / 快捷候选、整句恰好是这段拼音逐字的转换（不是续写）。不受 200 ms 时限限制（LLM 回包通常 > 1 s，按时限这个开关永远不生效，打开它就是愿意首选晚到被换）。刚换上去 150 ms 内的上屏仍给原首选。上屏整句走 `accept_prediction`（来源 `CloudSentence`，切词记个人 n-gram，已有）。

**Files:**
- Modify: `crates/qingjian-core/src/engine/prediction/{cover.rs,mod.rs}`、`crates/qingjian-core/src/engine/late/{mod.rs,state.rs}`、`crates/qingjian-core/src/engine/commit/mod.rs:77-79`
- Create: `crates/qingjian-core/src/engine/late/top.rs`
- Modify: `crates/qingjian-core/src/engine/tests/cloud_gate.rs`

- [ ] **Step 1: 写失败测试**

`crates/qingjian-core/src/engine/tests/cloud_gate.rs` 末尾加：

```rust

fn sentence_first() -> PredictionPolicy {
    let mut policy = PredictionPolicy::default();
    policy.late.sentence_first = true;
    policy
}

/// 发一次请求、回一条整句，返回本地候选与摆放结果。
fn place_sentence(
    engine: &mut Engine,
    probe: &Recorder,
    sentence: &str,
    touched: Option<usize>,
) -> (Vec<Candidate>, CloudPlacement) {
    engine.cancel_prediction();
    let local = {
        engine.set_input("suoyiwoxiangqu");
        engine.query().unwrap().candidates.items
    };
    let sequence = engine.request_prediction(None, &local).unwrap();
    engine.note_candidates_shown();
    probe.replies.lock().unwrap().push(Prediction {
        sequence,
        words: Vec::new(),
        sentence: Some(sentence.into()),
    });
    let prediction = engine.poll_prediction().unwrap();
    let placement = engine.place_prediction(prediction, &local, touched);
    (local, placement)
}

#[test]
fn a_converting_sentence_leads_when_switched_on() {
    let (mut engine, probe) = gated(sentence_first());
    let (local, placement) = place_sentence(&mut engine, &probe, "锁以我想去", None);
    assert_ne!(local[0].text, "锁以我想去");
    assert_eq!(placement.top.as_ref().unwrap().text, "锁以我想去");
    assert_eq!(placement.sentence, None);
    // 刚换上去就按的上屏：给原来的首选
    let top = placement.top.unwrap();
    assert_eq!(engine.commit(&top), local[0].text);
}

#[test]
fn the_leading_sentence_commits_as_a_sentence_after_the_guard() {
    let (mut engine, probe) = gated(sentence_first());
    let (_, placement) = place_sentence(&mut engine, &probe, "锁以我想去", None);
    std::thread::sleep(std::time::Duration::from_millis(
        prediction_defaults::SENTENCE_FIRST_GUARD_MS + 20,
    ));
    assert_eq!(engine.commit(&placement.top.unwrap()), "锁以我想去");
    assert!(engine.composition().is_empty());
}

#[test]
fn continuations_touched_lists_and_the_default_keep_the_sentence_on_tab() {
    let (mut engine, probe) = gated(sentence_first());
    let (_, placement) = place_sentence(&mut engine, &probe, "锁以我想去吃饭", None);
    assert!(placement.top.is_none());
    assert_eq!(placement.sentence.as_deref(), Some("锁以我想去吃饭"));
    let (_, placement) = place_sentence(&mut engine, &probe, "锁以我想去", Some(0));
    assert!(placement.top.is_none());
    let (mut engine, probe) = gated(PredictionPolicy::default());
    let (_, placement) = place_sentence(&mut engine, &probe, "锁以我想去", None);
    assert!(placement.top.is_none());
}
```

（`place_sentence` 开头先 `cancel_prediction`：否则第二次同样的字母会被 R5 复用、吐的是上一条整句。）

Run: `cargo test -p qingjian-core cloud_gate`
Expected: `a_converting_sentence_leads_when_switched_on` 与 `the_leading_sentence_commits_as_a_sentence_after_the_guard` FAIL（`placement.top` 是 `None`），第三个 PASS。

- [ ] **Step 2: `converts`**

`crates/qingjian-core/src/engine/prediction/cover.rs` 整个换成：

```rust
//! 整句补全的读音能不能盖住敲的字母：R5（上次的整句仍适用就复用、不再发）与云整句排第一（纯转换）共用。

use crate::parser;

/// 整句超过这么多字不判（逐字查读音在按键路径上，组句通常不到 20 字）。
const MAX_CHARS: usize = 32;

/// 最长的音节（`zhuang`）。
const MAX_SYLLABLE: usize = 6;

/// `sentence` 从头逐字读下去能不能盖住 `letters`：每个字吃掉一个它读得出的完整音节，字母吃完即算
/// （最后一个音节允许只敲了开头），句子余下的字是续写。`reads(字, 音节)` 是「这个字有这个读音」。
pub fn covers(sentence: &str, letters: &str, reads: impl Fn(&str, &str) -> bool) -> bool {
    walk(sentence, letters, &reads, false)
}

/// `sentence` 是不是 `letters` 的纯转换：逐字一个完整音节、字与字母同时用完（不是续写）。
pub fn converts(sentence: &str, letters: &str, reads: impl Fn(&str, &str) -> bool) -> bool {
    walk(sentence, letters, &reads, true)
}

fn walk(sentence: &str, letters: &str, reads: &impl Fn(&str, &str) -> bool, exact: bool) -> bool {
    let chars: Vec<String> = sentence.chars().map(String::from).collect();
    if chars.is_empty() || chars.len() > MAX_CHARS || letters.is_empty() || !letters.is_ascii() {
        return false;
    }
    step(&chars, letters, reads, exact)
}

fn step(chars: &[String], rest: &str, reads: &impl Fn(&str, &str) -> bool, exact: bool) -> bool {
    if rest.is_empty() {
        return !exact || chars.is_empty();
    }
    let Some((first, others)) = chars.split_first() else {
        return false;
    };
    if !exact
        && parser::SYLLABLES
            .iter()
            .any(|s| s.len() > rest.len() && s.starts_with(rest) && reads(first, s))
    {
        return true;
    }
    (1..=rest.len().min(MAX_SYLLABLE)).rev().any(|n| {
        let syllable = &rest[..n];
        parser::is_syllable(syllable)
            && reads(first, syllable)
            && step(others, &rest[n..], reads, exact)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reads(ch: &str, syllable: &str) -> bool {
        let table = [
            ("所", "suo"),
            ("以", "yi"),
            ("我", "wo"),
            ("想", "xiang"),
            ("去", "qu"),
            ("吃", "chi"),
            ("饭", "fan"),
        ];
        table.iter().any(|(c, s)| *c == ch && *s == syllable)
    }

    #[test]
    fn sentence_covers_its_own_prefix_with_a_partial_last_syllable() {
        assert!(covers("所以我想去吃饭", "suoyiwoxiangq", reads));
        assert!(covers("所以我想去吃饭", "suoyiwoxiangqu", reads));
        assert!(covers("所以我想去吃饭", "suoyiwoxiangquchif", reads));
    }

    #[test]
    fn a_different_letter_breaks_the_cover() {
        assert!(!covers("所以我想去吃饭", "suoyiwoxiangm", reads));
        assert!(!covers("所以我想去吃饭", "suoyiwoxiangquchifanle", reads));
    }

    #[test]
    fn conversion_needs_every_char_and_every_letter() {
        assert!(converts("所以我想去", "suoyiwoxiangqu", reads));
        assert!(!converts("所以我想去吃饭", "suoyiwoxiangqu", reads));
        assert!(!converts("所以我想去", "suoyiwoxiangq", reads));
    }
}
```

`crates/qingjian-core/src/engine/prediction/mod.rs`：`use cover::covers;` 换成 `pub(crate) use cover::{converts, covers};`；`fn char_reads(&self, …)` 改成 `pub(super) fn char_reads(&self, ch: &str, syllable: &str) -> bool`（`late` 要用；是 `engine` 内可见）。

- [ ] **Step 3: 排第一的整句与上屏挂钩**

新建 `crates/qingjian-core/src/engine/late/top.rs`：

```rust
//! 排到第一位的云整句，以及被它挤下去的本地首选：刚换上去的一小会儿里上屏仍给原首选。

use std::time::Instant;

use crate::candidate::Candidate;

#[derive(Debug, Clone)]
pub(crate) struct LateTop {
    /// 排到第一位的整句（显示的文本，繁体模式下已转繁）。
    pub(crate) sentence: String,

    /// 换上去的时刻。
    pub(crate) placed_at: Instant,

    /// 被挤到第二位的本地首选。
    pub(crate) displaced: Candidate,
}
```

`crates/qingjian-core/src/engine/late/state.rs` 换成：

```rust
//! 当前这一版候选的显示状态：壳报告画出来的时刻，以及排到第一位的云整句。只由壳的报告驱动，Core 不碰 UI。

use std::time::Instant;

use super::top::LateTop;

#[derive(Debug, Clone, Default)]
pub(crate) struct LateState {
    /// 壳第一次报告这一版候选画出来的时刻；`None` 是还没画（结果比画面先到，按时限内处理）。
    pub(crate) shown_at: Option<Instant>,

    /// 排到第一位的云整句。
    pub(crate) top: Option<LateTop>,
}
```

`crates/qingjian-core/src/engine/late/mod.rs`：

1. `mod state;` 后加 `mod top;`
2. 导入换成：

```rust
use std::time::{Duration, Instant};

use crate::candidate::{Candidate, CandidateKind};

use super::Engine;
use super::prediction::{CloudWord, Prediction, PredictionKind, converts, defaults};
use merge::place_words;
use top::LateTop;
```

3. `place_prediction` 里 `let placement = CloudPlacement { … };` 改成 `let mut placement = …`，紧接着加：

```rust
        if policy.late.sentence_first
            && touched.is_none()
            && let Some(sentence) = placement.sentence.clone()
            && let Some(first) = local.first()
            && self.sentence_may_lead(&sentence, first)
        {
            placement.top = Some(
                CloudWord {
                    text: sentence.clone(),
                    syllables: Vec::new(),
                    reading: None,
                }
                .into_candidate(),
            );
            placement.sentence = None;
            self.late.top = Some(LateTop {
                sentence,
                placed_at: now,
                displaced: first.clone(),
            });
        }
```

4. `impl Engine` 末尾（`place_prediction` 之后）加：

```rust

    /// 云整句能不能排到第一：组句联想、本地首选不是固定位置 / 快捷候选，且整句恰好是这段拼音逐字的转换（不是续写）。
    /// 本地首选是「用户常用且最近用过」时不让（B5 的 IsProtectLocalFirst）要读学习记录的时间戳，还没做。
    fn sentence_may_lead(&self, sentence: &str, first: &Candidate) -> bool {
        if self.last_prediction_kind != PredictionKind::Compose
            || self.question_mode()
            || matches!(
                first.kind,
                CandidateKind::Custom(_) | CandidateKind::Shortcut
            )
        {
            return false;
        }
        let original = self
            .traditional_map
            .borrow()
            .get(sentence)
            .cloned()
            .unwrap_or_else(|| sentence.to_owned());
        let decoded = self.decode(self.composition.scope());
        let typed = decoded
            .as_ref()
            .map_or(self.composition.scope(), |d| d.pinyin());
        let letters = typed.replace('\'', "");
        converts(&original, &letters, |ch, syllable| {
            self.char_reads(ch, syllable)
        })
    }

    /// `commit` 的挂钩：上屏的是排到第一位的云整句时走整句补全的上屏；刚换上去
    /// [`defaults::SENTENCE_FIRST_GUARD_MS`] 内按下的仍上屏原来的首选（用户是冲着原首选按的）。
    pub(super) fn commit_late_top(&mut self, candidate: &Candidate) -> Option<String> {
        let top = self.late.top.as_ref()?;
        if candidate.kind != CandidateKind::Cloud || candidate.text != top.sentence {
            return None;
        }
        let top = self.late.top.take()?;
        if top.placed_at.elapsed() < Duration::from_millis(defaults::SENTENCE_FIRST_GUARD_MS) {
            tracing::debug!("云整句刚换上第一位，这次上屏原首选");
            return Some(self.commit(&top.displaced));
        }
        Some(self.accept_prediction(&top.sentence))
    }
```

`crates/qingjian-core/src/engine/commit/mod.rs` 的 `commit` 换成（上游文件，只加三行挂钩）：

```rust
    pub fn commit(&mut self, candidate: &Candidate) -> String {
        if let Some(text) = self.commit_late_top(candidate) {
            return text;
        }
        self.commit_with(candidate, InputSource::from(candidate.kind), None)
    }
```

- [ ] **Step 4: 跑测试、提交**

```bash
cargo test -p qingjian-core
cargo fmt --all && cargo clippy --all-targets -- -D warnings
git add crates
git commit -m "feat(core): 可选「云整句排第一」（[predict] sentence_first，缺省关）" -m "#341：整句是这段拼音逐字的转换（不是续写）、用户没动过高亮、首选不是固定位置时排第一，Space 上屏走 accept_prediction。
WeType Mac/PC 缺省同样禁止云词抢首位，所以缺省关；不受 200ms 时限，换上去 150ms 内的上屏仍给原首选。
首选是用户常用词时不让（IsProtectLocalFirst）要学习记录的时间戳，没做，记在 todo。"
```

Expected：`cloud_gate` 8 个用例、`cover` 3 个用例全过；`cargo test -p qingjian-core` 全过。

---

## Task 12: macOS 壳——插位交给 Core、报画出时刻、`predict_now`

现状（`grep -n "set_cloud\|cloud_slots_untouched" apps/macos/src -r`）：`host/cloud/mod.rs:119` 自己判「翻页 / 高亮在末尾几格」再 `set_cloud` 填第一页末尾；`host/model/mod.rs:184` 重排后 `set_cloud` 带回。全部换成 Core。

**Files:**
- Modify: `apps/macos/src/host/cloud/mod.rs:86-197`、`apps/macos/src/host/model/mod.rs:181-185`、`apps/macos/src/host/presenting/mod.rs:84`、`apps/macos/src/host/mod.rs:24-27,128-129`、`apps/macos/src/host/init.rs:154`、`apps/macos/src/host/config/mod.rs:37`
- Modify: `apps/macos/src/imk/controller/display.rs:57-60,92-125`、`apps/macos/src/imk/controller/mod.rs:13,240`

- [ ] **Step 1: 云端结果按 `CloudPlacement` 摆**

`apps/macos/src/host/cloud/mod.rs`：

1. `poll_prediction` 的文档改成 `/// 定时器回调：结果到了就画上去。云端词按 Core 的插位规则摆，整句挂在 preedit 右侧（或排第一）。`
2. `apply_prediction` 里从 `// 云端词补进第一页末尾（前面的本地候选不动）…` 到 `self.render();` 换成：

```rust
        // 插在哪由 Core 定（时限内第 2 位，晚到或动过高亮后第 4 位起，可选整句排第一）；云端词没有译文，先补上
        let touched = self.session.navigated.then_some(self.session.highlighted);
        let local = self.session.layout.local().to_vec();
        let placement = self.engine.place_prediction(prediction, &local, touched);
        if !placement.is_empty() {
            let mut words = qingjian_core::CandidateList {
                items: placement.words,
            };
            self.engine.annotate(&mut words);
            self.session
                .layout
                .place_cloud(words.items, placement.at, placement.top);
        }
        self.sentence = placement.sentence;
        self.render();
```

3. 删掉 `pub(super) fn cloud_slots_untouched(…)` 连同它上面三行文档，删掉文件末尾的 `#[cfg(test)] mod tests { … question_answers_fill_the_page… }`（规则与问字整页的情形已在 Core `late::merge::tests::question_answers_fill_an_empty_list` 测）。

`apps/macos/src/host/mod.rs` 第 25 行 `use qingjian_core::{…}` 里去掉不再用的 `CloudWord`。

- [ ] **Step 2: 重排后带回云端结果、报画出时刻**

`apps/macos/src/host/model/mod.rs` 的 `poll_rescoring` 里

```rust
        let cloud = self.session.layout.cloud().to_vec();
        self.reset_session(preedit, query.candidates.items);
        if !cloud.is_empty() {
            self.session.layout.set_cloud(cloud);
        }
```

换成

```rust
        let previous = self.session.layout.clone();
        self.reset_session(preedit, query.candidates.items);
        self.session.layout.carry_cloud(&previous);
```

`apps/macos/src/host/presenting/mod.rs` 的 `pub fn render(&mut self) {` 下第一行加：

```rust
        // 晚到的云端词插在哪要看候选窗画出来多久了（Core 只记第一次）
        self.engine.note_candidates_shown();
```

`apps/macos/src/imk/controller/display.rs` 第 57 行注释 `// 先发联想再画：发出去就留好云端槽位，画出来的第一帧本地候选就已经在最终位置` 换成 `// 先发联想再画：发联想会清掉上一版的画出时刻，画的时候 Core 再记这一版的`。

- [ ] **Step 3: `predict_now` 快捷键**

`apps/macos/src/host/mod.rs`：`pub translate_keys: KeyCombo,` 之后加

```rust

    /// 「现在问云端」的快捷键（配置 `[shortcut] predict_now`）。
    pub predict_keys: KeyCombo,
```

`apps/macos/src/host/init.rs`：`translate_keys: KeyCombo::TRANSLATE_DEFAULT,` 之后加 `predict_keys: KeyCombo::PREDICT_NOW_DEFAULT,`。

`apps/macos/src/host/config/mod.rs`：`self.translate_keys = config.shortcut.translate_selection;` 之后加 `self.predict_keys = config.shortcut.predict_now;`。

`apps/macos/src/imk/controller/display.rs`：

1. `self.request_prediction(client, &candidates);` → `self.request_prediction(client, &candidates, false);`
2. `request_prediction` 换签名与文档、按 `now` 分叉：

```rust
    /// 发一次联想请求（`now`：快捷键「现在问云端」，不管字母数与按需模式）。Secure Input 里绝不发；没接联想器时是空操作。
    ///
    /// 读上下文要等应用回话，这段时间 IMK 可能把 `deactivateServer:` 之类的回调插进来，
    /// 所以分两次借 Host：先拿策略、放开借用去读、再借回来发请求。
    pub(super) fn request_prediction(
        &self,
        client: TextClient<'_>,
        candidates: &[Candidate],
        now: bool,
    ) {
```

   函数体里

```rust
            match h.engine.request_prediction(surrounding, candidates) {
                Some(_) => h.await_prediction(),
                None => h.cancel_prediction(),
            }
```

   换成

```rust
            let sent = if now {
                h.engine.request_prediction_now(surrounding, candidates)
            } else {
                h.engine.request_prediction(surrounding, candidates)
            };
            match sent {
                Some(_) => h.await_prediction(),
                None => h.cancel_prediction(),
            }
```

`apps/macos/src/imk/controller/mod.rs`：

1. 第 13 行 `use qingjian_platform::Modifiers;` → `use qingjian_platform::{KeyCombo, Modifiers};`
2. 翻译快捷键那段 `return self.translate_selection(client);` 的 `}` 之后、`// 修饰键 + 数字：按配置的两组组合上屏第一 / 第二个译词` 之前插入：

```rust
        // 「现在问云端」（组句中）：不管门槛发一次联想，结果照常由定时器送达
        let predict = host::with(|h| h.predict_keys).unwrap_or(KeyCombo::PREDICT_NOW_DEFAULT);
        if pressed == predict.modifiers
            && typed.as_deref().and_then(|t| t.chars().next()) == Some(predict.key)
            && host::with(|h| !h.engine.composition().is_empty()).unwrap_or(false)
        {
            let candidates =
                host::with(|h| h.session.layout.local().to_vec()).unwrap_or_default();
            self.request_prediction(client, &candidates, true);
            return true;
        }
```

- [ ] **Step 4: 编译与测试**

```bash
cargo clippy -p qingjian-macos --all-targets -- -D warnings
cargo test -p qingjian-macos
```

Expected：无警告；测试全过（删掉的 `cloud_slots_untouched` 测试不再出现）。

- [ ] **Step 5: 真机验证（必做，contributing「外部 PR」）**

```bash
apps/macos/scripts/bundle.sh --install
```

1. 菜单栏切到青简，「偏好设置 → 云服务」开大模型，确认 ☁︎；`~/Library/Application Support/Qingjian/config.toml` 的 `[general] log_level = "debug"`。
2. TextEdit 里依次敲并停顿 1 秒：`nihao`（不应出云端词）、`fhyq`（只出整句）、`suoyiwoxiangqu`（云端词出现在第 4 位起，第 1–3 位本地候选不动；整句挂 preedit 右侧）。
3. 敲 `suoyiwoxiangqu` 后马上按 ↓ 两次再等结果：高亮在第 3 位时云端词插在其后，高亮所在的词不变；按 ↓ 到第 5 位再等：这次不插（日志 `outcome=Skipped`）。
4. 退格一格停顿：`~/Library/Logs/Qingjian/` 最新日志里是 `联想不发 reason="退格"`，没有新的「联想完成」。
5. 敲 `nih` 按 `⌃⌥P`：日志出现「联想完成」（快捷键越过 R1）。
6. 把 `[predict] trigger = "manual"` 存盘：敲 `suoyiwoxiangqu` 停顿不发；按 `⌃⌥P` 才发。改回 `auto`。
7. `[predict] sentence_first = true`：敲 `jintiantianqihenhao` 停顿，云整句若是纯转换就排第一（带云朵），等 1 秒后 `Space` 上屏整句；再试一次结果到的瞬间就按 `Space`，上屏的是原首选。改回 `false`。
8. 问字：`u` 开头 `usangemu`（三个木）照常出答案、整页都是答案。

把系统版本、应用、每步看到的结果记下来，写进 PR「怎么验证的」。

- [ ] **Step 6: 提交**

```bash
cargo fmt --all
git add apps/macos
git commit -m "feat(macos): 云端词插位改由 Core 决定，候选窗画出时报 Core，加 ⌃⌥P「现在问云端」" -m "删掉壳里的 cloud_slots_untouched 与第一页末尾填格：时限内第 2 位、晚到或动过高亮后第 4 位起、高亮在要挤动的位置时不插（Core engine/late）。
重排后用 CandidateLayout::carry_cloud 把云端结果按原位置带回。
真机：<系统版本、应用、Step 5 结果>。"
```

---

## Task 13: 素笺 iOS 桥——插位交给 Core，手机上仍紧跟首选

现状：`cloud/crates/qingjian-cloud-bridge/src/session/cloud.rs` 的 `poll()` 把云端词插到 `CLOUD_POSITION = 1`，`session/rescore.rs` 的 `requery_local` 也按它插回。换成 Core 的 `place_prediction`；手机候选栏窄，桥里把 `late_frozen` 覆盖成 1，晚到的词仍在第 2 格。

**Files:**
- Modify: `cloud/crates/qingjian-cloud-bridge/src/session/{cloud.rs,rescore.rs,mod.rs,config.rs}`

- [ ] **Step 1: `Session` 记住插位**

`cloud/crates/qingjian-cloud-bridge/src/session/mod.rs`：

```rust
    /// 候选栏里的格子，下标与 Swift 那边显示的一致；云端结果回来后按 Core 的插位规则插进来。
    entries: Vec<Entry>,

    /// 云端词插在第几格起（Core 的 `place_prediction` 算的）；重排后重查本地候选时照这个位置插回。
    cloud_at: usize,
```

（替换原来 `entries` 字段与它的文档；构造里 `entries: Vec::new(),` 之后加 `cloud_at: 0,`。）

- [ ] **Step 2: `cloud.rs`**

1. 文件头第一行里「（插在首选之后，不抢空格要上屏的那个；」改成「（插在哪由 Core 的 `place_prediction` 定，不抢空格要上屏的那个；」。
2. 删掉 `pub(super) const CLOUD_POSITION: usize = 1;` 与它的文档。
3. `poll()` 里从 `let mut position = CLOUD_POSITION.min(self.entries.len());` 到函数末尾 `inserted || reloaded` 换成：

```rust
        // 键盘扩展不知道用户有没有滑动候选栏，按没动过算；整句补全在配置里已关（`sentence: false`），排第一也就不会出现
        let local = self.local_candidates();
        let placement = self.engine.place_prediction(prediction, &local, None);
        if placement.is_empty() {
            return reloaded;
        }
        self.cloud_at = placement.at;
        let mut position = placement.at.min(self.entries.len());
        for candidate in placement.words {
            if self.entries.iter().any(|e| e.text() == candidate.text) {
                continue;
            }
            self.entries.insert(position, Entry::Cloud(candidate));
            position += 1;
        }
        if let Some(top) = placement.top {
            self.entries.insert(0, Entry::Cloud(top));
        }
        true
    }

    /// 候选栏里的本地候选（云端插进来的不算）。
    pub(super) fn local_candidates(&self) -> Vec<Candidate> {
        self.entries
            .iter()
            .filter_map(|entry| match entry {
                Entry::Local(candidate) => Some(candidate.clone()),
                _ => None,
            })
            .collect()
    }
```

4. `request_prediction` 换成：

```rust
    pub(super) fn request_prediction(&mut self) {
        let local = self.local_candidates();
        if self
            .engine
            .request_prediction(self.context.clone(), &local)
            .is_none()
        {
            self.engine.cancel_prediction();
        }
        // 候选栏随这次调用的返回刷新，下一帧就画出来；晚到的云端词插在哪按这个时刻算
        self.engine.note_candidates_shown();
    }
```

- [ ] **Step 3: `rescore.rs` 与 `config.rs`**

`cloud/crates/qingjian-cloud-bridge/src/session/rescore.rs`：删掉 `use super::cloud::CLOUD_POSITION;`；`requery_local` 的文档改成 `/// 带着新到的神经分重查一遍：本地候选换成新次序，云端的词按原样插回上次 Core 定的位置。`；`let mut position = CLOUD_POSITION.min(self.entries.len());` → `let mut position = self.cloud_at.min(self.entries.len());`。

`cloud/crates/qingjian-cloud-bridge/src/session/config.rs` 的 `apply_prediction`：文档第二行改成 `/// 自定义接口照配置用。手机候选栏窄，晚到的云端词也插在首选之后（`late_frozen = 1`，与以前一样），见 `cloud.rs`。`，函数体第一行加：

```rust
        let predict = &PredictConfig {
            late_frozen: 1,
            ..predict.clone()
        };
```

- [ ] **Step 4: 编译与测试**

```bash
cd $WT/cloud
cargo fmt --all
cargo clippy -p qingjian-cloud-bridge --all-targets -- -D warnings
cargo test -p qingjian-cloud-bridge
cd $WT
```

Expected：无警告，测试全过（`cloud/Cargo.lock` 不变：没有新依赖）。

- [ ] **Step 5: 真机 / 模拟器验证**

```bash
cd $WT/cloud/ios && scripts/build-bridge.sh && xcodegen generate
```

（新工作树要先补 gitignore 产物，见记忆「iOS worktree 构建」；构建用 Xcode 打开 `cloud/ios` 的工程装到真机或模拟器。）在备忘录里打 `suoyiwoxiangqu` 停 1 秒：云端词出现在第 2 格（首选之后），首选不变；打 `nihao` 不出云端词；Xcode 控制台里有 `联想不发 reason="短全拼"` 之类的 debug 日志（要开 debug 级别）。记下机型、iOS 版本与结果写进 PR。

- [ ] **Step 6: 提交**

```bash
git add cloud/crates/qingjian-cloud-bridge
git commit -m "feat(cloud): iOS 键盘的云端词插位改由 Core 决定，手机上仍紧跟首选" -m "删掉桥里的 CLOUD_POSITION，poll 与重排后的重查都照 Engine::place_prediction 的位置插；late_frozen 覆盖为 1，窄候选栏里晚到的词仍在第 2 格。
真机：<机型、iOS 版本、结果>。"
```

---

## Task 14: 文档同步（同一 PR）

**Files:**
- Create: `docs/notes/cloud-request-throttle.md`
- Modify: `docs/user/cloud/index.md`、`docs/user/getting-started/keys.md:104-114`、`docs/user/help/data-and-logs.md:44-51`、`docs/notes/crate-notes.md:115-121`、`docs/plan/todo.md:94`、`cloud/docs/fork-patch.md`（「## 合并上游时」之前）

- [ ] **Step 1: 核对 Linux 壳确实不用改**

```bash
grep -rn "request_prediction\|poll_prediction\|set_cloud" apps/linux/server/src
grep -n "cloud_slots = 0" apps/linux/server/src/dispatch/mod.rs
```

Expected：第一条没有输出；第二条命中 `dispatch/mod.rs:74`（Linux 第一阶段不接云联想）。这个结论写进下面的笔记。

- [ ] **Step 2: 工程笔记**

新建 `docs/notes/cloud-request-throttle.md`（`<…>` 处填执行时量到的数）：

```markdown
# 云联想请求降频（D2）

上游 #177：只要停顿就调模型，几百字输入 100 多次请求、十几万 token；#341：想要大模型整句排第一。
做法参照 WeType 2.2.3 的静态阅读（`CloudSearchByInputV2`、`CloudInputMerger`），常数按青简的 LLM 成本重定。

## 规则

| # | 规则 | 缺省 | 放在哪 |
|---|---|---|---|
| — | 已作废的请求（继续敲、退格、上屏、清空）防抖后不发 | 开 | `qingjian-predict` `Dispatcher`，Engine 换序号时 `Predictor::supersede` |
| R1 | 组句拼音少于 `min_letters` 个字母不发 | 4 | Core `engine/prediction`（问字不受限，仍是 2） |
| R2 | 不超过 `short_full_letters` 个字母且整串是完整全拼不发 | 6 | Core |
| R3 | 同一段组句里比上次真正发出的短（退格）不发 | 开 | `Dispatcher` |
| R4 | 同一段组句里比上次真正发出的只多不到 `min_growth` 个字母不发 | 3 | `Dispatcher` |
| R5 | 上次的整句补全逐字读音仍盖得住新字母（末音节可只敲开头）就复用 | 开 | Core（`cover::covers`，不容错） |
| R6 | 缓存键去掉本地候选与切分；容量 256；只在内存 | — | `qingjian-predict` `cache.rs` |
| R7 | 在途同请求不重发 | — | 后台线程串行 + 回包先进缓存，已满足 |
| R8 | 令牌桶 `rate_burst` / `rate_refill_ms` | 关（0） | `Dispatcher` |
| R9 | 服务商 429 → 停 30 秒（拿不到 `Retry-After`） | 开 | `Dispatcher::lock` |
| R10 | 被 429 锁过 10 分钟内防抖至少 600 ms | 开 | `Dispatcher::debounce` |
| R11 | `trigger = "manual"` 只在 `[shortcut] predict_now` 时发 | auto | Core（快捷键绕过 R1/R2/R5/R11） |

退格与增量放在发送线程，不在 Core：壳每次缓冲变化都调 `request_prediction`，防抖在后台线程，Engine 看不到哪次真的发了；
在 Core 判会把用户在「被跳过的那一键」上的停顿也吞掉。请求带 `composition`（Engine 每作废一次加一），只在同一段里比。

## 晚到插位（`engine/late/`）

| 情形 | 插在哪 |
|---|---|
| 候选窗画出来 `late_grace_ms`（200）内到、没动过高亮 | 第 2 位，最多 1 条 |
| 之后到、或动过高亮 | 前 `late_frozen`（3）个本地候选之后，最多 `slots` 条 |
| 动过高亮且高亮落在要被挤动的位置 | 这次不插 |
| 用户固定位置的词（自定义短语） | 不让，插在它之后 |
| 问字（没有本地候选） | 整页给答案 |
| `sentence_first`（缺省关）且整句是纯转换、没动过高亮、首选不是固定位置 / 快捷候选 | 整句排第一；换上去 150 ms 内的上屏仍给原首选 |

壳只做两件事：画完调 `Engine::note_candidates_shown`；结果到了调 `Engine::place_prediction(prediction, 本地候选, 动过高亮时的下标)`，
照 `CloudPlacement` 摆（macOS / Windows 用 `CandidateLayout::place_cloud`，重排后 `carry_cloud`）。iOS 桥把 `late_frozen` 覆盖成 1（候选栏窄）。
Linux 壳不接云联想（`dispatch/mod.rs` 把 `cloud_slots` 置 0、不调 `request_prediction`），不改。
整句 / 词级神经重排晚到后的「翻页 / 动过高亮不重排」守卫还在各壳里，没并进 `engine/late/`。

## 模拟模型（`qingjian-cli --predict-sim`）

按输入日志重放每段组句的逐键输入（一段拼音分几次上屏算一段），门槛用 Engine 与 `Dispatcher` 的真代码，不联网。
日志没有逐键时刻：键间隔 100 ms（日志 ms/键 的 p25）；最后一键到上屏的余量 ≥ 防抖算一次停顿；中途停顿按概率 p 撒在第 2 键之后；
每条 `retype` 算「退一格、停顿、敲回来、停顿」；上屏时壳作废联想。回包内容模拟不了，R5 的收益不在数里。

```bash
cargo build --release -p qingjian-cli
target/release/qingjian-cli --config tools/eval/offline.toml --predict-sim data/eval/input-log-2026-10-04.jsonl --predict-sim-pause 0.1
```

## 结果（冻结日志 `input-log-2026-10-04.jsonl`，1210 段组句、6386 个字母）

| 步骤 | p=0 | p=0.1 | p=0.2 |
|---|---|---|---|
| 改前（Task 1） | <> | <> | <> |
| + 作废即不发 | <> | <> | <> |
| + R1 / R2 | <> | <> | <> |
| + R3 / R4 | <> | <> | <> |
| + R6 | <> | <> | <> |
| 降幅 | <> | <> | <> |

本地候选与排序不变：`tools/release/gate-pass.sh` 五个指标与基线逐个相等（<贴输出>）。

## 未决

- 首选是「用户常用且最近用过」时不让云整句抢（WeType IsProtectLocalFirst）：要 Learner 给最后使用时间，现在只保护固定位置与快捷候选。
- `Retry-After`：要换成能拿到响应头的调用（或开 async-openai 的 middleware）才能用服务商给的值。
- 真机两周后用输入日志里 `prediction` 事件之后紧跟的 `commit` 来源，对比云词被选次数（目标：不降超过 10%）。
```

- [ ] **Step 3: 用户文档**

`docs/user/cloud/index.md`：

1. frontmatter 的 `description` 改成 `可选、缺省关闭的云联想：开启方式、什么时候发送、发送的内容与对象、云端候选词与整句补全的显示、按需模式、译词自动补充。`
2. 「## 云端候选词」一节第一段换成：

```markdown
云端给出的词带云朵标记，插在候选里，数字键正常选择，上屏消耗整段拼音：

- 候选窗出来后很快（0.2 秒内）就到的，插在第二位；更晚到的、或者你已经用方向键移动过高亮，排在前三个本地候选之后，前三位不动。
- 你的高亮已经停在会被挤动的位置时，这次不插，免得正要选的词换了位置。自定义短语固定的位置不会被占。
- 最多插几个由「云服务」页的格数决定（0 至 4，`[predict] slots`，0 表示不要云端词，只留整句补全）。
```

   原来的列表里删掉「前面的本地候选位置不变，`Space` 上屏的仍是本地首选。」这一条（改成上面的规则），其余几条保留。
3. 「## 云端候选词」之前插入新的一节：

```markdown
## 什么时候发送

不是每次停顿都问。输入拼音停顿 0.3 秒后，下面几种情况不发送：

- 不到 4 个字母；不超过 6 个字母而且都是完整拼音（如 `nihao`、`zhuang`）——这类本机候选已经够准。
- 退格之后；比上次问过的只多一两个字母（多 3 个字母才再问）。
- 上次给的整句仍能对上接着敲的字母（`suoyiwoxiangq` 给了「所以我想去吃饭」，再敲 `u` 仍对得上），直接沿用。
- 同样的输入问过了：直接用上次的结果。上屏、清空之后还没发出去的请求也不再发。
- 服务商提示请求太频繁时，停 30 秒再问。

`[predict]` 里可以调：`min_letters`、`short_full_letters`、`min_growth`（写 0 关掉对应那条），`rate_burst` / `rate_refill_ms`（每分钟上限，缺省不限）。

## 按需模式

`[predict] trigger = "manual"` 后组句中不再自动问，只在按 `⌃⌥P`（Windows `Ctrl + Alt + P`，`[shortcut] predict_now` 可改）时问一次；
自动模式下按它也能不管上面的条件立刻问一次。问字（`u` 开头）不受按需模式影响。

## 整句排第一

`[predict] sentence_first = true` 后，云端整句如果正好是这段拼音的直接转换（不是往后续写），排到候选第一位，`Space` 上屏它。
缺省关：云端回复通常要一秒以上，打开后已经显示的首选可能在你按空格前被换掉；结果刚换上去的一瞬间按下的 `Space` 仍上屏原来的首选。
你移动过高亮、或首选是固定位置的自定义短语时不换。
```

4. 「## 不联想的情况」一段末尾加一句：`「云服务」页关掉大模型或菜单栏关掉云联想后不再发送；只想要整句补全不要云端词，把格数设为 0。`

`docs/user/getting-started/keys.md` 的「## 云联想」表格里「翻译应用中选中的文字」一行之后加：

```markdown
| 现在问云端（组句中，不管字母多少；按需模式下唯一的发送方式） | `⌃⌥P` | `Ctrl + Alt + P` |
```

   表格下那段改成 `均需先打开云联想，见 [问字与翻译](../cloud/question-and-translate.md) 与 [云联想](../cloud/index.md#按需模式)。两个快捷键都可在配置文件 [shortcut] 修改；Windows 上修改后需切换到其他输入法再切回青简才生效。`（Windows 的 `Ctrl + Alt + P` 在 Task 17 合入之前还不能用：Task 17 不合的话这一格写「—」。）

`docs/user/help/data-and-logs.md`「## 云联想发什么」列表末尾加：`- 不是每次停顿都发送：字母太少、短而完整的拼音、退格、只多一两个字母、上次结果还适用时都不发，见 [云联想](../cloud/index.md#什么时候发送)。`

- [ ] **Step 4: 实现要点、待办、分叉补丁**

`docs/notes/crate-notes.md` 的 `## crates/qingjian-predict` 第一条里「后台线程防抖 / 缓存 / 超时」改成「后台线程防抖 / `Dispatcher`（作废、缓存、退格与增量、令牌桶、429 锁定）/ 超时」；「一次请求给云端词（容错校验后补进候选第一页末尾 `[predict] slots` 格，缺省 2，不预留不占位，前面的本地候选不挪；排布在 Core `CandidateLayout`）」改成「一次请求给云端词（容错校验后由 Core `engine/late` 定插位：画出 200 ms 内第 2 位、之后第 4 位起，最多 `[predict] slots` 格；壳用 `CandidateLayout::place_cloud` 摆）」；该节末尾加一条：

```markdown
- 请求降频（`docs/notes/cloud-request-throttle.md`）：Core 判 R1 字母数 / R2 短全拼 / R5 整句复用 / R11 按需（`engine/prediction/gate.rs`，常数在 `prediction/defaults.rs`），
  `Dispatcher` 判作废 / 退格 / 增量 / 令牌桶 / 429（缓存 256 条，键不含本地候选与切分）。`qingjian-cli --predict-sim <日志>` 用同一套代码数会发几次。
```

`docs/plan/todo.md` 第 94 行「云联想收尾（Phase 6）：密钥进钥匙串（…）、按应用禁用、限流与用量统计」里「限流与用量统计」改成「用量统计（限流已做：令牌桶 + 429 锁定，见 `docs/notes/cloud-request-throttle.md`）」，下面加一行：

```markdown
- [ ] 云整句排第一的首选保护：本地首选是「用户常用且最近用过」时不让换（WeType IsProtectLocalFirst），要 Learner 给词的最后使用时间；整句 / 词级重排的晚到守卫从四个壳收进 Core `engine/late/`
```

`cloud/docs/fork-patch.md`，在「## 合并上游时」之前加一节：

```markdown
### 云联想请求降频与晚到插位（Core + predict + 壳，各平台都生效）

上游 #177（停顿就发）、#341（云整句排第一）。规则与数字见 `docs/notes/cloud-request-throttle.md`。新代码都在新文件，上游文件只加挂钩：

| 文件 | 改动 | 说明 |
|---|---|---|
| `crates/qingjian-core/src/engine/prediction/{defaults,trigger,skip_reason,gate,cover}.rs` | 新文件 | 常数、触发方式、没发原因、门槛状态、整句读音覆盖 |
| `crates/qingjian-core/src/engine/late/` | 新目录 | 晚到插位与云整句排第一 |
| `crates/qingjian-core/src/engine/prediction/mod.rs` | 改约 120 行 | `request_prediction` 接门槛、`request_prediction_now`、`prediction_skip`、换序号即 `supersede`、复用结果；`char_reads` 改 `pub(super)` |
| `crates/qingjian-core/src/engine/prediction/{policy,predictor,request}.rs` | 加字段 / 方法 | `PredictionPolicy` 新门槛与 `late`；`Predictor::supersede` 缺省空；`PredictionRequest::composition` |
| `crates/qingjian-core/src/engine/mod.rs`、`lib.rs` | 加约 10 行 | `mod late;`、两个字段、导出 |
| `crates/qingjian-core/src/engine/commit/mod.rs` | 加 3 行 | `commit` 首行挂 `commit_late_top` |
| `crates/qingjian-core/src/candidate/layout/candidate_layout.rs` | 加约 40 行 | `place_cloud` / `carry_cloud`；`set_cloud` 保留 |
| `crates/qingjian-predict/src/{dispatcher/,rate.rs}` | 新文件 | 发送判断与令牌桶 |
| `crates/qingjian-predict/src/{worker,cloud_predictor,cache,config,error,lib}.rs` | 改 | 用 Dispatcher、supersede、缓存键、配置键、429 识别 |
| `crates/qingjian-platform/src/config/{key_combo,shortcut,mod}.rs` | 加约 40 行 | `[shortcut] predict_now`、`[predict]` 模板 |
| `apps/cli/src/predict_sim/` | 新目录 | `--predict-sim` |
| `apps/macos/src/host/{cloud/mod,model/mod,presenting/mod,mod,init,config/mod}.rs`、`imk/controller/{mod,display}.rs` | 改 | 插位交给 Core、报画出时刻、`⌃⌥P` |

合并上游时：上游若改了 `request_prediction` 的门槛或 `CandidateLayout::set_cloud`，按上游新写法把挂钩重新接上；`cargo test -p qingjian-core cloud_gate late` 与 `cargo test -p qingjian-predict dispatcher` 守着行为。
```

- [ ] **Step 5: 提交**

```bash
git add docs cloud/docs/fork-patch.md
git commit -m "docs: 云联想什么时候发送、按需模式与整句排第一；请求降频的规则与模拟数字" -m "用户页写清触发条件与新配置键（偏好设置不加开关）；工程笔记记规则出处、模拟模型与前后数字；fork-patch 记分叉补丁。"
```

---

## Task 15: 全量验证

- [ ] **Step 1: 格式、静态检查、测试（两个 workspace）**

```bash
cd $WT
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo test --workspace
cd cloud && cargo fmt --all --check && cargo clippy -p qingjian-cloud-bridge --all-targets -- -D warnings && cargo test -p qingjian-cloud-bridge && cd ..
```

Expected：全部通过，无警告。

- [ ] **Step 2: 本地候选与排序不变**

```bash
tools/release/gate-pass.sh
```

Expected：none / fusion 两种配置五个指标（评测首选 / 前三 / 字准、clean2 词首选 / 整句首选）与基线**逐个相等**（差值 `+0.0`），写出 `data/generated/GATE_PASSED`。任何一个不为 0 都是误伤本地路径：停下来查，别调容差。

- [ ] **Step 3: 最终模拟数**

```bash
cargo build --release -p qingjian-cli
for p in 0 0.1 0.2; do target/release/qingjian-cli --config tools/eval/offline.toml --predict-sim data/eval/input-log-2026-10-04.jsonl --predict-sim-pause $p 2>/dev/null; done
```

Expected：实发约 249 / 335 / 395（我在 `/tmp/plan-d2` 量到的），比 Task 1 降 60% 以上。把三个数与降幅填进 `docs/notes/cloud-request-throttle.md` 的结果表，`git commit -m "docs(notes): 云联想请求降频的最终模拟数字"`。

---

## Task 16: 推分支、开 PR（不合并）

- [ ] **Step 1: 推送**

```bash
git push -u origin cloud-throttle/request-gates
```

（`origin` 必须是 `dayuer/qingjian`：先 `git remote get-url origin` 确认；**不往 qingjian-team/qingjian 推。**）

- [ ] **Step 2: 开 PR**

```bash
gh pr create --repo dayuer/qingjian --base sujian --head cloud-throttle/request-gates \
  --title "feat: 云联想请求降频（停顿不再必发）与晚到插位收回 Core" \
  --body "$(cat <<'EOF'
## 做了什么

- 云联想不再「停顿就发」：Core 判字母数 / 短全拼 / 整句复用 / 按需模式，发送线程判已作废 / 退格 / 小增量 / 缓存 / 令牌桶（缺省关）/ 429 锁定（上游 #177）。
- 晚到的云端词插在哪由 Core `engine/late/` 决定：画出 200 ms 内第 2 位，之后第 4 位起，高亮在要挤动的位置时不插；macOS 与 iOS 桥的自写守卫删掉。
- 可选 `[predict] sentence_first`（缺省关）：纯转换的云整句排第一（上游 #341）。
- `[predict] trigger = "manual"` 与 `[shortcut] predict_now`（macOS ⌃⌥P）。Windows 另开 PR。
- `qingjian-cli --predict-sim`：用真实门槛按输入日志数会发几次请求。

## 怎么验证的

- `cargo test --workspace`、`cargo clippy --all-targets -- -D warnings`、`cloud/` 下 bridge 的 clippy 与测试：全过。
- `tools/release/gate-pass.sh`：none / fusion 五个指标与基线逐个相等（本地候选与排序不变）。
- `--predict-sim`（冻结日志 1210 段组句）实发：p=0 <改前>→<改后>，p=0.1 <>→<>，p=0.2 <>→<>。
- macOS 真机：<系统版本>，TextEdit：<Task 12 Step 5 每步结果>。
- iOS：<机型 / iOS 版本>，备忘录：<Task 13 Step 5 结果>。
EOF
)"
```

- [ ] **Step 3: 停在这里**

把 PR 链接与「怎么验证的」交给用户。**不合并**；用户明说「合 #N」或自己点合并才算。工作树等 PR 合了再 `git -C /Users/liyuqing/sproot/qingjian worktree remove ../qingjian-cloud-throttle`。

---

## Task 17: Windows 壳（单独分支与 PR，需要 Windows 真机）

TSF DLL 在 macOS 上编不了，壳行为改动必须真机验，所以从 Task 16 的分支切一条新分支单独做；没有 Windows 机器就只推分支、不开 PR，在主 PR 里注明。

```bash
git -C $WT switch -c cloud-throttle/windows
```

**Files:**
- Modify: `apps/windows/server/src/dispatch/{composed/mod.rs,rescore/mod.rs,config.rs,message.rs}`
- Modify: `apps/windows/tsf/src/com/{key/preserved.rs,service/mod.rs,service/processor.rs,service/key_sink.rs}`

- [ ] **Step 1: Server 插位交给 Core、报画出时刻**

`apps/windows/server/src/dispatch/composed/mod.rs`：

1. 导入换成：

```rust
use qingjian_core::{Candidate, CandidateLayout, CandidateList, Query};
use qingjian_platform::protocol::{
    Frame, KeyEvent, KeyModifiers, PROTOCOL_VERSION, PreeditKind, PreeditSegment,
};
```

2. `recompose` 里

```rust
                if self.engine.prediction_enabled() {
                    self.engine.request_prediction(None, layout.local());
                }
```

   换成

```rust
                if self.engine.prediction_enabled() {
                    self.engine.request_prediction(None, layout.local());
                    // 这一帧随按键结果马上画出来；晚到的云端词插在哪按这个时刻算
                    self.engine.note_candidates_shown();
                }
```

3. `poll_prediction` 文档改成 `/// 拉一次云联想结果：云端词按 Core 的插位并进候选布局，整句补全记下；翻译评审时结果是译文。`，末尾

```rust
        if let Some(Composed::Candidates { layout, .. }) = self.composed.as_mut() {
            let words: Vec<Candidate> = prediction
                .words
                .into_iter()
                .map(CloudWord::into_candidate)
                .collect();
            layout.set_cloud(words);
            self.sentence = prediction.sentence;
        }
    }
```

   换成

```rust
        // 插在哪由 Core 定：时限内第 2 位，晚到或动过高亮后第 4 位起，可选整句排第一
        if let Some(Composed::Candidates { layout, .. }) = self.composed.as_mut() {
            let touched = self.navigated.then_some(self.highlight);
            let local = layout.local().to_vec();
            let placement = self.engine.place_prediction(prediction, &local, touched);
            if !placement.is_empty() {
                layout.place_cloud(placement.words, placement.at, placement.top);
            }
            self.sentence = placement.sentence;
        }
    }

    /// 「现在问云端」快捷键（`[shortcut] predict_now`）：DLL 登记成保留键，命中时按这个组合转发过来。
    pub(super) fn matches_predict_combo(&self, event: &KeyEvent) -> bool {
        let combo = self.config.predict_now;
        event.character == Some(combo.key)
            && event.modifiers.chord() == KeyModifiers::from(combo.modifiers)
    }

    /// 不管门槛立刻问一次云端；结果照常由轮询送达。
    pub(super) fn predict_now(&mut self) {
        if let Some(Composed::Candidates { layout, .. }) = &self.composed {
            let local = layout.local().to_vec();
            self.engine.request_prediction_now(None, &local);
        }
    }
```

`apps/windows/server/src/dispatch/rescore/mod.rs` 的 `requery_rescored` 里

```rust
        let cloud = layout.cloud().to_vec();
        let mut rebuilt = CandidateLayout::new(
            query.candidates.items.clone(),
            self.config.page_size,
            self.config.cloud_slots,
        );
        if !cloud.is_empty() {
            rebuilt.set_cloud(cloud);
        }
```

换成

```rust
        let mut rebuilt = CandidateLayout::new(
            query.candidates.items.clone(),
            self.config.page_size,
            self.config.cloud_slots,
        );
        rebuilt.carry_cloud(layout);
```

`apps/windows/server/src/dispatch/config.rs`：`pub translate_selection: KeyCombo,` 之后加

```rust

    /// 「现在问云端」快捷键（`[shortcut] predict_now`）。
    pub predict_now: KeyCombo,
```

`From<&Config>` 里 `translate_selection: config.shortcut.translate_selection,` 之后加 `predict_now: config.shortcut.predict_now,`。

`apps/windows/server/src/dispatch/message.rs` 的 `handle_key` 里 `let (commit, outcome) = match self.apply_key(&event) {` 之前插入：

```rust
        if !self.engine.composition().is_empty()
            && self.engine.prediction_enabled()
            && self.matches_predict_combo(&event)
        {
            self.predict_now();
            return ServerMessage::KeyResult {
                session,
                outcome: KeyOutcome::Consumed,
                commit: None,
                frame: self.current_frame(),
            };
        }
```

本机（macOS）先验 Server：

```bash
cargo clippy -p qingjian-windows-server --all-targets -- -D warnings
cargo test -p qingjian-windows-server
```

Expected：无警告，测试全过（我在 macOS 上跑过：67 个 dispatch 测试过）。

- [ ] **Step 2: DLL 登记 `predict_now` 保留键**

带 Alt 的组合不经击键 sink（见 `preserved.rs` 文件头），所以照「翻译选中文字」登记成 TSF 保留键。

`apps/windows/tsf/src/com/key/preserved.rs`：文件头第一行改成「「翻译选中文字」「现在问云端」与「Ctrl + Alt + Space 切换中英」三个快捷键登记成 TSF **保留键**」；`GUID_SWITCH_MODE` 之后加：

```rust

/// 「现在问云端」保留键的标识。
pub(crate) const GUID_PREDICT_NOW: GUID = GUID::from_u128(0x7d2e4a90_1b6c_4f3a_9e85_3c4d5e6f7a81);
```

`load_combo` 之后加：

```rust

/// 读 `[shortcut] predict_now`；读不到 / 解析失败用缺省 Ctrl+Alt+P。
pub(crate) fn load_predict_combo() -> KeyCombo {
    let Some(path) = qingjian_platform::dirs::config_path() else {
        return KeyCombo::PREDICT_NOW_DEFAULT;
    };
    match Config::load(&path) {
        Ok(config) => config.shortcut.predict_now,
        Err(error) => {
            log(&format!("读配置取「现在问云端」快捷键失败，用缺省: {error}"));
            KeyCombo::PREDICT_NOW_DEFAULT
        }
    }
}
```

`unregister` 之后加：

```rust

pub(crate) fn register_predict_now(
    keystroke: &ITfKeystrokeMgr,
    tid: u32,
    combo: KeyCombo,
) -> Result<()> {
    let key = preserved_key(combo);
    let description: Vec<u16> = "现在问云端（青简）".encode_utf16().collect();
    unsafe { keystroke.PreserveKey(tid, &GUID_PREDICT_NOW, &key, &description) }
}

pub(crate) fn unregister_predict_now(keystroke: &ITfKeystrokeMgr, combo: KeyCombo) {
    let key = preserved_key(combo);
    let _ = unsafe { keystroke.UnpreserveKey(&GUID_PREDICT_NOW, &key) };
}
```

`apps/windows/tsf/src/com/service/mod.rs`：`translate_combo: Cell<Option<KeyCombo>>,` 字段之后加

```rust

    /// 登记成保留键的「现在问云端」组合；停用时撤掉。
    predict_combo: Cell<Option<KeyCombo>>,
```

构造里 `translate_combo: Cell::new(None),` 之后加 `predict_combo: Cell::new(None),`。

`apps/windows/tsf/src/com/service/processor.rs`：`Activate` 里翻译快捷键的 `match preserved::register(…) { … }` 之后加：

```rust
        let predict = preserved::load_predict_combo();
        match preserved::register_predict_now(&keystroke, tid, predict) {
            Ok(()) => {
                self.predict_combo.set(Some(predict));
                log(&format!("「现在问云端」快捷键已登记为保留键: {predict}"));
            }
            Err(error) => log(&format!("登记「现在问云端」快捷键失败: {error}")),
        }
```

`Deactivate` 里 `if let Some(combo) = self.translate_combo.take() { preserved::unregister(&keystroke, combo); }` 之后加：

```rust
            if let Some(combo) = self.predict_combo.take() {
                preserved::unregister_predict_now(&keystroke, combo);
            }
```

`apps/windows/tsf/src/com/service/key_sink.rs` 的 `OnPreservedKey`：`if guid == preserved::GUID_SWITCH_MODE { … }` 之后加：

```rust
        if guid == preserved::GUID_PREDICT_NOW {
            // 只在组句中转发：不在组句时它不是我们的键
            let Some(combo) = self.predict_combo.get() else {
                return Ok(FALSE);
            };
            if !self.shared.composing() {
                return Ok(FALSE);
            }
            let event = preserved::key_event(combo, self.mode_state.english());
            return Ok(self.forward_key(pic, event).into());
        }
```

- [ ] **Step 3: Windows 机器上编译、真机验证**

在 Windows 机器上（部署见 `apps/windows/README.md`）：

```powershell
cargo build --release -p qingjian-windows-server -p qingjian-tsf
$env:QINGJIAN_UIACCESS=0; cargo test -p qingjian-windows-server
```

记事本里：`suoyiwoxiangqu` 停 1 秒，云端词在第 4 位起；方向键把高亮移到第 5 位后等结果，不插；`nih` 按 `Ctrl + Alt + P` 发出（Server 日志「联想完成」）；不在组句时按 `Ctrl + Alt + P` 交给应用。macOS 上本机只能 `cargo check --target x86_64-pc-windows-gnu`（要先 `rustup target add x86_64-pc-windows-gnu`）。

- [ ] **Step 4: 提交、推送、（真机验过才）开 PR**

```bash
git add apps/windows docs/user/getting-started/keys.md
git commit -m "feat(windows): 云端词插位改由 Core 决定，加 Ctrl+Alt+P「现在问云端」" -m "Server 删掉 set_cloud 填第一页末尾，照 Engine::place_prediction 摆；重排后 carry_cloud。DLL 把 predict_now 登记成保留键（带 Alt 不经击键 sink）。
真机：<Windows 版本、应用、结果>。"
git push -u origin cloud-throttle/windows
gh pr create --repo dayuer/qingjian --base sujian --head cloud-throttle/windows --title "feat(windows): 云联想插位收回 Core、Ctrl+Alt+P 现在问云端" --body "依赖主 PR（云联想请求降频）。怎么验证的：<Windows 版本、应用、步骤与结果>"
```

没有 Windows 真机：只推分支，不开 PR；Task 14 里 `keys.md` 的 Windows 那格写「—」。

---

## Self-Review

**规格覆盖（d2 §3.2 R1–R11、§3.4、§3.5；b5 §3.2–3.3）：**

| 规格项 | 任务 |
|---|---|
| R1 / R2 | Task 4 |
| R3 / R4 | Task 5（改到发送线程，见差异 1） |
| R5 | Task 7 |
| R6 | Task 6 |
| R7 | 不写代码（差异 3） |
| R8 / R9 / R10 | Task 8 |
| R11 + `predict_now` | Task 2（配置）、Task 9（Core）、Task 12（macOS）、Task 17（Windows） |
| 单测清单 1–15 | 1 `TooShort`、2/3/4 `ShortFullPinyin`（Task 4）；5 `abbreviations_are_sent_for_the_sentence_only`；6/7 `deletion_and_small_growth_are_not_sent`（Task 5，用 Dispatcher 测）；8/9 `a_covering_sentence_is_reused_without_asking_again`；10 `manual_trigger_only_sends_on_the_shortcut`；11 `cache_key_ignores_local_candidates`；12/13 `rate_limit_drops_the_seventh_request` + `rate::burst_then_refill`；14 `rate_limited_status_locks_and_raises_debounce`；15 由串行 worker + 缓存满足（差异 3） |
| `--predict-sim` | Task 1，之后每个任务报数 |
| `sentence_first` + 150 ms 保护 | Task 11 |
| B5 `engine/late/` 云端那半 + 单测 1–7 | Task 10（`late::merge::tests`）；8–14（重排）不在本计划（差异 7） |
| 四个壳守卫与插位 | macOS Task 12、iOS Task 13、Windows Task 17、Linux 无需改（Task 14 Step 1） |
| 文档（cloud 页、keys、data-and-logs、crate-notes、fork-patch） | Task 14 |
| 验收：模拟降幅、gate-pass 不变 | Task 15 |

**类型一致性：** `SkipReason`（Task 1）← `PredictionGate::skip`（Task 4）、`Decision::Skip`（Task 1/5/8）；`Dispatcher::new(superseded)`（Task 1）→ `new(config, superseded)`（Task 5，三处调用同改：`cloud_predictor.rs`、`predict_sim/mod.rs`、`dispatcher/tests.rs`）；`decide(&request, now)` 签名 Task 1 起不变（Task 8 前参数名 `_now`）；`PredictionRequest::composition`（Task 3）← `Dispatcher` `Sent`（Task 5）；`CloudPlacement { at, words, top, sentence, outcome }`（Task 10）← `CandidateLayout::place_cloud(words, at, top)`（Task 10）← 壳（Task 12/13/17）；`LateState { shown_at }`（Task 10）+ `top`（Task 11）；`cover::covers`（Task 7）+ `converts`（Task 11，整个文件替换）。

## 未决（交回用户）

1. R9 / R10（429 锁 30 秒、锁后防抖 600 ms）缺省开着——它们是对服务商限流的被动响应，不是用户要关的那个「限流」（令牌桶，缺省关）；要也做成开关就在 Task 8 加 `[predict] honor_429`。
2. 晚到第 4 位起插对素笺 iOS 窄候选栏不合适，计划里 iOS 覆盖成第 2 格（`late_frozen = 1`）；要不要同样改 macOS 的缺省，等真机看误选。
3. 首选保护（IsProtectLocalFirst）与重排晚到守卫没收进 Core（todo 已记）。
