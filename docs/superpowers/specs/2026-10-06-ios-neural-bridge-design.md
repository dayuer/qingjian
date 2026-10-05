# 素笺 iOS 键盘接入本地神经引擎（2026-10-06）

## 目标

给素笺 App 的 iOS 键盘（`cloud/ios/Keyboard`）接上本地神经整句模型（含章·通变，P2C，44MB）：
Mac 上「词图 + 神经重打分」的效果原样带到手机，**常开、内存吃紧自动卸载**。

不做：知微（53MB 必超限）、量化（f16→int8，另立项）、`apps/ios` 第四壳（spike 的内存问题被本设计覆盖）。

## 约束

- Core 零改动：桥接的还是 `Engine::set_async_sentence_scorer`（Mac 壳同款机制，加载中查询照常出词级候选）。
- 桥的 FFI 是单线程 C ABI：加载放后台线程（预热要编译 Metal 内核，几百毫秒），经 channel 交接，
  每个 `qj_*` 入口 `try_recv` 无锁检查，接上即生效——照抄 `apps/macos/src/host/model/mod.rs` 的成熟模式。
- 键盘扩展内存上限（jetsam 按 `phys_footprint`，社区实测 48–60MB）：词库 / LM 都是 mmap clean 页不计足迹，
  通变的 44MB dirty 是唯一大头，贴上限，真机四组实测定生死（空 / 引擎 / +通变 / 敲 20 字）。
- 内存自保：`os_proc_available_memory` 低于水位（8MB）→ 立即卸载、本会话不再自动加载、记日志。

## 桥的新 FFI（`qingjian-cloud-bridge`）

| 函数 | 语义 |
|---|---|
| `qj_load_model(session, path, p2c) -> bool` | 异步开始加载；已在加载 / 在用时返回 false。加载失败不影响会话 |
| `qj_model_state(session) -> u8` | 0 未加载 / 1 加载中 / 2 在用 / 3 上次失败 |
| `qj_unload_model(session)` | 立即卸掉（内部 `set_async_sentence_scorer(None)`），状态回 0 |
| `qj_model_memory_mb(session) -> f64` | 模型自报的内存占用（调试面板显示用），没加载返回 0 |

加载线程：`CharScorer::load` → P2C 包装（`P2cScorer::new`，缺 `<sep>` 报 Corrupt）→ 预热 `score_p2c("ni", ["你"])`
→ channel 发回 `Box<dyn SentenceScorer>`；Session 在下一个 `qj_*` 调用里接上。

## iOS 侧

- `cloud/ios/scripts/build-bridge.sh`：把 `data/models/hanzhang-tongbian/hanzhang-tongbian-small.qjm`
  拷进 `Keyboard/Data/models/`（Data 目录已是资源，无需动 project.yml）。
- `KeyboardModel`：键盘启动时若设置开着 → `qj_load_model(bundle 路径, p2c: true)`；
  现有 0.25s poll 里查 `qj_model_state`，状态变化时刷新候选（模型上线后重排结果经 poll 自然浮现）；
  每次 poll 检查 `os_proc_available_memory()`，低于 8MB → `qj_unload_model` 并本会话封印。
- 素笺 App 设置页：「本地整句模型」开关（缺省开）+ 状态行（未加载 / 加载中 / 在用 / 失败）。

## 验收

- 桥集成测试（`QINGJIAN_DATA` 门，仓库惯例）：加载→状态 2→卸载→状态 0→再加载；无模型文件时 state 3、会话不受影响。
- iOS 现有测试（XCUITest、单元）不回归。
- 真机四组内存数字写 `docs/notes/`（或 cloud 侧文档），超限不回滚功能——自动卸载兜底，量化另立项。
