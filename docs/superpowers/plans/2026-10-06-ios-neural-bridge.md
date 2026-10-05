# 素笺 iOS 键盘接入本地神经引擎 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 素笺 iOS 键盘加载含章·通变（P2C）本地模型做整句重打分，常开、内存吃紧自动卸载。

**Architecture:** 桥加四个 C ABI（加载走后台线程 + channel 交接，照抄 `apps/macos/src/host/model/mod.rs` 模式），接 `Engine::set_async_sentence_scorer`（`setup.rs:288`），Core 零改动；iOS 侧 0.25s poll 里查状态 + `os_proc_available_memory` 自保。

**Tech Stack:** `qingjian-cloud-bridge`（Rust FFI）、`qingjian-neural`（CharScorer/P2cScorer/candle）、Swift（KeyboardModel / App Settings）。

**spec:** `docs/superpowers/specs/2026-10-06-ios-neural-bridge-design.md`

---

### Task 1: 桥的模型状态机与 FFI

**Files:**
- Create: `cloud/crates/qingjian-cloud-bridge/src/session/model.rs`（加载线程 + 状态）
- Modify: `cloud/crates/qingjian-cloud-bridge/src/session/mod.rs`（`mod model;` + Session 字段 + 各 FFI 入口的 try_recv 挂钩）
- Modify: `cloud/crates/qingjian-cloud-bridge/src/lib.rs`（四个 `qj_*` 导出）
- Modify: `cloud/crates/qingjian-cloud-bridge/Cargo.toml`（加 `qingjian-neural` path 依赖）
- Test: `cloud/crates/qingjian-cloud-bridge/tests/model.rs`（`QINGJIAN_DATA` 门，惯例同 `tests/session.rs:1`）

- [ ] Step 1: `session/model.rs`：`ModelState { Idle, Loading, Active, Failed }` 与 `Session` 上的
      `load_model(path, p2c) -> bool` / `poll_model()`（try_recv，接到就 `engine.set_async_sentence_scorer(Some(...))`，
      状态转 Active）/ `unload_model()`（`set_async_sentence_scorer(None)` + drop 信道 + 状态回 Idle）/
      `model_state() -> u8`。加载线程做 `CharScorer::load` → （p2c 时）`P2cScorer::new` → 预热
      `score_p2c("ni", ["你"])`（非 p2c 用 `scorer.score("", ["的"])`），失败发 Err 记 tracing。
- [ ] Step 2: `Session::refresh`（或最常走的查询入口）与 `qj_poll`（云轮询）开头调 `poll_model()`。
- [ ] Step 3: `lib.rs` 导出 `qj_load_model` / `qj_model_state` / `qj_unload_model`；`qj_model_memory_mb`
      先返回 0（模型自报尺寸后续补，不阻塞主线）。
- [ ] Step 4: `tests/model.rs`：`QINGJIAN_DATA` 下加载真模型 → 轮询至 state 2 → 卸载 → state 0 → 再加载成功；
      路径不存在 → state 3，会话按键照常。没数据跳过、`QINGJIAN_REQUIRE_DATA=1` 失败。
- [ ] Step 5: `cargo test -p qingjian-cloud-bridge` + `cargo clippy -p qingjian-cloud-bridge --all-targets -- -D warnings`。
- [ ] Step 6: 提交 `feat(cloud): 桥接本地神经模型，加载卸载与状态的 C ABI`。

### Task 2: 构建脚本带模型

**Files:**
- Modify: `cloud/ios/scripts/build-bridge.sh`（拷 `.qjm` 进 `Keyboard/Data/models/`，与 dict.qj 同款 cmp -s 判重）

- [ ] Step 1: 脚本 47–66 行那段后面加 `mkdir -p …/Data/models` + 拷
      `data/models/hanzhang-tongbian/hanzhang-tongbian-small.qjm`（从 `QINGJIAN_DATA` 或仓库 `data/` 取，与现有数据源一致）。
- [ ] Step 2: 跑一遍脚本确认文件落位；Keyboard target 的 `Data/` 已是资源（dict.qj 同路径），`project.yml` 不动。
- [ ] Step 3: 提交 `build(cloud): 键盘扩展随包带通变模型`。

### Task 3: 键盘侧加载与内存自保

**Files:**
- Modify: `cloud/ios/Keyboard/Sources/KeyboardModel.swift`（启动加载 + poll 查状态 + 内存水位自卸）
- Modify: `cloud/ios/Keyboard/Sources/KeyboardViewController.swift`（poll 回调里带上模型状态变化）

- [ ] Step 1: `KeyboardModel`：`loadModelIfNeeded()`（UserDefaults 开关默认开；`Bundle(for:)` 或键盘 bundle 的
      `Data/models/hanzhang-tongbian-small.qjm` 路径传 `qj_load_model(path, true)`）。
- [ ] Step 2: poll 里加 `qj_model_state` 检查，状态变化记 log；`os_proc_available_memory()` < 8MB →
      `qj_unload_model()` + 本会话封印标志（不再自动加载）。
- [ ] Step 3: Swift 侧单测（能纯逻辑测的部分：水位判断、封印逻辑）。
- [ ] Step 4: 提交 `feat(cloud): iOS 键盘启动加载通变，内存吃紧自动卸载`。

### Task 4: App 设置页开关

**Files:**
- Modify: `cloud/ios/App/Settings/KeyboardSettings.swift`（「本地整句模型」开关，缺省开）
- Modify: 设置落盘的那个桥配置（跟随现有设置读写通道）

- [ ] Step 1: 开关读写 + 键盘侧读同一 UserDefaults（App Group 若有，跟随现有惯例）。
- [ ] Step 2: 提交 `feat(cloud): 素笺 App 设置页加本地整句模型开关`。

### Task 5: 验收与文档

- [ ] Step 1: `cd cloud && cargo test`（全量）+ iOS 构建通过。
- [ ] Step 2: 真机：四组内存数字（空 / 引擎 / +通变 / 敲 20 字）记录之；`jishimutiandi` 真机敲出「几十亩田地」。
- [ ] Step 3: 文档：桥的模型接口写进 `cloud/docs/`（或 README 对应小节）；spec/计划归档已在 `docs/superpowers/`。

---

## Self-Review

- Spec 覆盖：FFI 四个 → Task 1；构建 → Task 2；加载/自保 → Task 3；开关 → Task 4；验收 → Task 5。✅
- 无占位；`qj_model_memory_mb` 明确「先返 0 不阻塞」。✅
- 类型一致：`set_async_sentence_scorer(Option<Box<dyn SentenceScorer>>)`（`setup.rs:288`）、
  `P2cScorer::new(CharScorer) -> Option<Self>`、预热调用形状照 mac。✅
