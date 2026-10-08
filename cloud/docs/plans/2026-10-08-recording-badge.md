# 键盘「记录中 / 已暂停」（05 的 2a、2b）实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 键盘工具栏牌子后面显示「● 记录中」（在记输入日志并上传时）；点它暂停 1 小时，提示行位置出「已暂停记录，提示照常。1 小时后恢复」+「一直暂停」；暂停时标记变空心圈「已暂停」，点它立即恢复。

**Architecture:** 「记录」= 输入日志（`input-log.jsonl`，2B 的 `input_log` 开关）在写、在传；记一笔的素材是用户主动存的，不算。暂停只停**记**（会话里把输入日志换成什么都不写的 `NoInputLogger`），本地学习、提示、记一笔都照常；已经记下的照常上传。暂停状态落在 App Group 的 `cloud/recording-pause.json`，键盘进程被杀、重开都还在。全部在桥（`cloud/crates/qingjian-cloud-bridge`）与 iOS 壳里做，**上游 `crates/` 一行不动**（Core 已有 `Engine::set_input_logger`）。

**Tech Stack:** Rust（桥，`cargo test -p qingjian-cloud-bridge`，在 `cloud/` 下跑）、Swift 6 / SwiftUI（键盘扩展）、XCTest / XCUITest。

**与设计稿 05 的 2a / 2b 的有意偏离（PR 描述与 UI 清单约束 7 都要写，下次回写设计稿时带上）：**
1. 稿子牌子写「小美 · 恋爱」：场景已删（约束 4），牌子只有人。
2. 稿子提示条写「切走应用或 1 小时后恢复」：键盘扩展拿不到宿主 App 的身份，只做「1 小时后恢复」。
3. 稿子说「一直暂停就是关掉这个场景的记录」：没有场景了；关「上传输入日志」开关会删服务器数据，太重。改成**一直暂停到用户点「已暂停」恢复**，不碰同意开关。
4. 稿子键位（分词 / 英 / 发送）、`.k-py` 行照约束 7 不照。

**约定：** 一个类型一个文件、文件头注释、注释中文只写约束与原因；颜色经 `ColorUsage`；不写装饰性分隔注释；提交信息照 `docs/contributing.md`（中文、不写署名尾行）。

---

## 文件结构

| 路径 | 职责 |
|---|---|
| `cloud/crates/qingjian-cloud-bridge/src/recording/mod.rs` | `mod` 声明与 re-export |
| `cloud/crates/qingjian-cloud-bridge/src/recording/pause.rs` | `RecordingPause`：暂停文件的读写与到期判断（纯逻辑 + 文件） |
| `cloud/crates/qingjian-cloud-bridge/src/recording/state.rs` | `RecordingState`：0 不显示 / 1 记录中 / 2 暂停到某时 / 3 一直暂停 |
| `cloud/crates/qingjian-cloud-bridge/src/session/recording.rs` | `impl Session`：`recording_state` / `pause_recording` / `resume_recording`，换输入日志 |
| `cloud/crates/qingjian-cloud-bridge/src/session/mod.rs` | 会话打开时按暂停文件决定装不装 `InputLog` |
| `cloud/crates/qingjian-cloud-bridge/src/lib.rs` + `include/qingjian_bridge.h` | 三个 C 接口 |
| `cloud/ios/Keyboard/Sources/RecordingBadge.swift` | 牌子后面的标记 |
| `cloud/ios/Keyboard/Sources/PausedBanner.swift` | 提示行位置的暂停提示条 |
| `cloud/ios/Shared/Memory/RecordingDisplay.swift` | 文案与「出不出」的纯值（可测） |
| `cloud/ios/Keyboard/Sources/KeyboardModel.swift`、`IdleBar.swift`、`KeyboardView.swift` | 接线 |
| `cloud/ios/Tests/RecordingDisplayTests.swift` | Swift 单测 |
| `cloud/docs/plans/2026-10-05-ui-implementation.md` | 05 的 2a / 2b 状态、约束 7 加偏离 |

---

## Task 1：桥——暂停文件

**Files:** Create `src/recording/{mod,pause,state}.rs`；Modify `src/lib.rs`（`mod recording;`）

- [ ] **Step 1：写失败的测试**（`src/recording/pause.rs` 末尾 `#[cfg(test)] mod tests`）

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_file_means_not_paused() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(RecordingPause::load(dir.path()).until(1_000), None);
    }

    #[test]
    fn timed_pause_expires() {
        let dir = tempfile::tempdir().unwrap();
        RecordingPause::pause_for(dir.path(), 3600, 1_000).unwrap();
        assert_eq!(RecordingPause::load(dir.path()).until(1_000), Some(Until::At(4_600)));
        assert_eq!(RecordingPause::load(dir.path()).until(4_600), None, "到点就不算暂停了");
    }

    #[test]
    fn forever_and_resume() {
        let dir = tempfile::tempdir().unwrap();
        RecordingPause::pause_forever(dir.path()).unwrap();
        assert_eq!(RecordingPause::load(dir.path()).until(9_999_999), Some(Until::Forever));
        RecordingPause::resume(dir.path()).unwrap();
        assert_eq!(RecordingPause::load(dir.path()).until(1), None);
    }

    #[test]
    fn broken_file_is_not_paused() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("cloud")).unwrap();
        std::fs::write(dir.path().join("cloud/recording-pause.json"), "{oops").unwrap();
        assert_eq!(RecordingPause::load(dir.path()).until(1), None);
    }
}
```

（`tempfile` 若不在桥的 dev-dependencies 里，看桥现有测试怎么建临时目录，照它的写法；不新增依赖。）

- [ ] **Step 2：跑，预期编译失败。**

```bash
cd cloud && cargo test -p qingjian-cloud-bridge recording
```

- [ ] **Step 3：实现 `pause.rs`**

```rust
//! 「记录中」的暂停状态：`<user_dir>/cloud/recording-pause.json`，`{"until": 秒}` 或 `{"until": null}`（一直暂停）。
//! 落盘是因为键盘进程随时被杀，重开要还记得在暂停；文件坏了按没暂停算（宁可多记也不让标记卡死在「已暂停」）。

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// 暂停到什么时候。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Until {
    At(i64),
    Forever,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct RecordingPause {
    /// `Some(秒)` 定时，`None` 一直暂停；文件不存在时整个结构是缺省（没暂停）。
    until: Option<i64>,

    #[serde(skip)]
    present: bool,
}

impl RecordingPause {
    fn path(user_dir: &Path) -> PathBuf {
        user_dir.join("cloud").join("recording-pause.json")
    }

    pub fn load(user_dir: &Path) -> Self {
        std::fs::read_to_string(Self::path(user_dir))
            .ok()
            .and_then(|text| serde_json::from_str::<Self>(&text).ok())
            .map(|mut pause| {
                pause.present = true;
                pause
            })
            .unwrap_or_default()
    }

    /// 现在（`now` 秒）还在不在暂停。
    pub fn until(&self, now: i64) -> Option<Until> {
        if !self.present {
            return None;
        }
        match self.until {
            None => Some(Until::Forever),
            Some(at) if at > now => Some(Until::At(at)),
            Some(_) => None,
        }
    }

    pub fn pause_for(user_dir: &Path, seconds: i64, now: i64) -> std::io::Result<()> {
        Self::write(user_dir, Some(now + seconds))
    }

    pub fn pause_forever(user_dir: &Path) -> std::io::Result<()> {
        Self::write(user_dir, None)
    }

    pub fn resume(user_dir: &Path) -> std::io::Result<()> {
        match std::fs::remove_file(Self::path(user_dir)) {
            Err(error) if error.kind() != std::io::ErrorKind::NotFound => Err(error),
            _ => Ok(()),
        }
    }

    fn write(user_dir: &Path, until: Option<i64>) -> std::io::Result<()> {
        let path = Self::path(user_dir);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let text = serde_json::to_string(&Self { until, present: true }).map_err(std::io::Error::other)?;
        std::fs::write(path, text)
    }
}
```

（桥里已有原子写文件的辅助函数——`cloud_config.rs` 里那个「`cloud.toml` 与 `memory/` 下的文件都走这里」的——**用它代替 `std::fs::write`**，照它的签名改。）

`state.rs`：

```rust
//! 键盘工具栏「记录中」标记的四种状态，C 接口按 u8 传（见 qingjian_bridge.h）。

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum RecordingState {
    /// 不显示：没登录、没开「上传输入日志」。
    Off = 0,
    Recording = 1,
    PausedTimed = 2,
    PausedForever = 3,
}
```

`mod.rs`：`mod pause; mod state; pub use pause::{RecordingPause, Until}; pub use state::RecordingState;`（文件头一行注释）。

- [ ] **Step 4：跑测试，预期全过；`cargo clippy -p qingjian-cloud-bridge --all-targets -- -D warnings` 干净。**

- [ ] **Step 5：提交** `feat(cloud): 桥加「记录中」的暂停状态文件`

---

## Task 2：桥——会话里换输入日志 + C 接口

**Files:** Create `src/session/recording.rs`；Modify `src/session/mod.rs`、`src/lib.rs`、`include/qingjian_bridge.h`

- [ ] **Step 1：写失败的测试**（`session/recording.rs` 的测试模块；照 `session` 里已有测试建会话的方式——先读它们，用同一个 helper 建一个「登录了、`logs = true`」的会话，和一个 `logs = false` 的）：
  - `logs = false` 的会话：`recording_state(now)` 是 `Off`；`pause_recording` 什么都不做（不写文件）。
  - `logs = true`：初始 `Recording`；`pause_recording(Some(3600), now)` 后 `PausedTimed`，且这期间打字提交后 `input-log.jsonl` **没有新增行**（照会话测试里打字上屏的写法打一句）；`now + 3600` 时 `recording_state` 回到 `Recording` 并把 `InputLog` 装回去，再打一句能看到新行；`pause_recording(None, now)` 后 `PausedForever`；`resume_recording()` 后回 `Recording`。
  - 会话重开：暂停文件在时，新开的会话一开始就是暂停（不装 `InputLog`）。

- [ ] **Step 2：跑，预期失败。**

- [ ] **Step 3：实现**
  - `Session` 加字段 `logs_on: bool`（会话打开时 `cloud.logs` 的值，`session/mod.rs` 现在 `if … && cloud.logs` 那段同时记下）与 `logger_paused: bool`。
  - 会话打开时：`logs_on` 且 `RecordingPause::load(dir).until(now).is_none()` 才 `with_input_logger(InputLog::open(...))`；暂停中就不装，`logger_paused = true`。
  - `session/recording.rs`：

```rust
//! 「记录中」：暂停就把输入日志换成什么都不写的 NoInputLogger，恢复再把 InputLog 装回去。
//! 只停「记」，学习、提示、记一笔照常；已经记下的照常上传（上传器不看这里）。

use qingjian_core::NoInputLogger;
use qingjian_learning::InputLog;

use super::Session;
use crate::recording::{RecordingPause, RecordingState, Until};

impl Session {
    /// 顺带处理到期：定时暂停过了点，就把日志装回去。键盘每次刷新都会问，所以到期最多晚一次刷新。
    pub fn recording_state(&mut self, now: i64) -> RecordingState {
        let Some(dir) = self.user_dir.clone().filter(|_| self.logs_on) else {
            return RecordingState::Off;
        };
        match RecordingPause::load(&dir).until(now) {
            Some(Until::At(_)) => RecordingState::PausedTimed,
            Some(Until::Forever) => RecordingState::PausedForever,
            None => {
                if self.logger_paused {
                    self.engine.set_input_logger(Box::new(InputLog::open(dir.join("input-log.jsonl"))));
                    self.logger_paused = false;
                }
                RecordingState::Recording
            }
        }
    }

    /// `seconds` 为 None 是一直暂停。
    pub fn pause_recording(&mut self, seconds: Option<i64>, now: i64) {
        let Some(dir) = self.user_dir.clone().filter(|_| self.logs_on) else {
            return;
        };
        let written = match seconds {
            Some(seconds) => RecordingPause::pause_for(&dir, seconds, now),
            None => RecordingPause::pause_forever(&dir),
        };
        if let Err(error) = written {
            tracing::warn!(%error, "暂停记录没写上");
            return;
        }
        self.engine.set_input_logger(Box::new(NoInputLogger));
        self.logger_paused = true;
    }

    pub fn resume_recording(&mut self, now: i64) {
        if let Some(dir) = self.user_dir.clone() {
            if let Err(error) = RecordingPause::resume(&dir) {
                tracing::warn!(%error, "恢复记录没删掉暂停文件");
            }
        }
        let _ = self.recording_state(now);
    }
}
```

（`self.engine`、`self.user_dir` 的实际字段名照 `Session` 改；`now` 用 `crate::memory::now_unix()`。）
  - C 接口（`lib.rs`，照 `qj_set_private` 的 `with(session, …)` 写法）：

```c
// 键盘工具栏「记录中」（05 的 2a / 2b）。0 不显示（没登录、没开上传输入日志）、1 记录中、2 暂停（1 小时后自动恢复）、3 一直暂停。
// 问它时顺带处理到期。暂停只停记输入日志，学习、提示、记一笔照常。
uint8_t qj_recording_state(QjSession *session);
// seconds > 0 暂停这么多秒；<= 0 一直暂停（到 qj_recording_resume）。
void qj_recording_pause(QjSession *session, int64_t seconds);
void qj_recording_resume(QjSession *session);
```

- [ ] **Step 4：跑桥的全部测试与 clippy，预期全过。**
- [ ] **Step 5：提交** `feat(cloud): 桥的会话可以暂停与恢复记输入日志`

---

## Task 3：Swift——文案与出不出（纯值）

**Files:** Create `cloud/ios/Shared/Memory/RecordingDisplay.swift`、`cloud/ios/Tests/RecordingDisplayTests.swift`

- [ ] **Step 1：写失败的测试**

```swift
// 「记录中 / 已暂停」标记与暂停提示条：桥给的状态怎么显示、什么时候不出。

import XCTest
@testable import QingjianCloud

final class RecordingDisplayTests: XCTestCase {
    func testBadge() {
        XCTAssertNil(RecordingDisplay.badge(state: 0, privateField: false, fullAccess: true))
        XCTAssertEqual(RecordingDisplay.badge(state: 1, privateField: false, fullAccess: true), .recording)
        XCTAssertEqual(RecordingDisplay.badge(state: 2, privateField: false, fullAccess: true), .paused)
        XCTAssertEqual(RecordingDisplay.badge(state: 3, privateField: false, fullAccess: true), .paused)
        XCTAssertNil(RecordingDisplay.badge(state: 1, privateField: true, fullAccess: true), "密码框里不出")
        XCTAssertNil(RecordingDisplay.badge(state: 1, privateField: false, fullAccess: false), "没完全访问时键盘不联网，不出")
    }

    func testCopy() {
        XCTAssertEqual(RecordingDisplay.Badge.recording.title, "记录中")
        XCTAssertEqual(RecordingDisplay.Badge.paused.title, "已暂停")
        XCTAssertEqual(RecordingDisplay.pausedBanner, "已暂停记录，提示照常。1 小时后恢复")
        XCTAssertEqual(RecordingDisplay.pauseForever, "一直暂停")
        XCTAssertEqual(RecordingDisplay.pausedForeverBanner, "已暂停记录，点「已暂停」恢复")
        XCTAssertEqual(RecordingDisplay.resumedBanner, "已恢复记录")
        XCTAssertEqual(RecordingDisplay.pauseSeconds, 3600)
    }
}
```

- [ ] **Step 2：跑，预期编译失败。**
- [ ] **Step 3：实现**（`enum RecordingDisplay` + 嵌套 `enum Badge { case recording, paused; var title }`——嵌套的小枚举与它的命名空间同文件，同 `ScopeDisplay` 的做法；`badge(state:privateField:fullAccess:)` 照测试；文案常数照测试）。
- [ ] **Step 4：跑，预期全过。**
- [ ] **Step 5：提交** `feat(cloud): 「记录中」标记的文案与显示规则`

---

## Task 4：Swift——标记、提示条、接线

**Files:** Create `cloud/ios/Keyboard/Sources/RecordingBadge.swift`、`PausedBanner.swift`；Modify `KeyboardModel.swift`、`IdleBar.swift`、`KeyboardView.swift`；桥的 Swift 包装（键盘里调 C 接口的那层，照 `qj_set_private` 在 Swift 侧的调用处找）

视觉（照设计稿 05 的 2a / 2b 与 theme.css；稿子里 `.rec` 类没有定义，取下面的值）：
- **RecordingBadge**：牌子右边 6pt；圆点 6pt + 文字 12pt，间距 4；**记录中**：实心圆点 `Theme.accentInk`、字 `Theme.accentInk`；**已暂停**：空心圆（1.2pt 描边 `Theme.ink3`）、字 `Theme.ink3`。高 30、左右 6 的点按区域。`ColorUsage` 加 `recordingBadge`（`.accent`）与 `pausedBadge`（`.ink3`），`NoFullAccessStyleTests` 的 accent 白名单同步加 `recordingBadge`（它代表「在记关于你和这个人的对话」，按约束 1 归人）。
- **PausedBanner**：占提示行的位置（高 34，同 `HintRow`），底 `KeyStyle.keyFill`（同记一笔 toast），左 6pt 灰点（`Theme.ink3`），12.5pt `Theme.ink2` 文案，右边「一直暂停」12.5pt `Theme.accentInk`（只在定时暂停时出），下沿 1pt `Hairline.line`。
- 键盘高度变化跟提示行一样走 `ScopeDisplay.hasHintRow`：加一个 `hasBanner` 参数（与 `hasNoteBar` 并列），对象卡 / 草稿卡 / 冲突屏打开时不出。

行为：
- `KeyboardModel`：`private(set) var recording: UInt8`，在 `refresh()` 与键盘出现时问 `qj_recording_state`；`private(set) var recordingBanner: String?` + 是否带「一直暂停」。
- 点「记录中」→ `qj_recording_pause(3600)`，banner = `pausedBanner`（带「一直暂停」），4 秒后或下一次按键消失。
- 点「一直暂停」→ `qj_recording_pause(0)`，banner 换成 `pausedForeverBanner`（不带按钮），4 秒后消失。
- 点「已暂停」→ `qj_recording_resume()`，banner = `resumedBanner`，2 秒后消失。
- `IdleBar.actions`：`ScopeChip` 后面、`记一笔` 前面插 `RecordingBadge`（`RecordingDisplay.badge(...)` 为 nil 就不插）。对象卡 / 草稿卡的工具栏、改写条、快捷切人、私密输入那几支**不**出标记。

- [ ] **Step 1–3：按上面实现。** 每个新文件头两行注释。
- [ ] **Step 4：单测全绿**（`xcodebuild test … -only-testing:QingjianCloudTests`，命令见 `cloud/docs/plans/2026-10-08-ui-paper-pages.md`「构建与测试命令」）。
- [ ] **Step 5：提交** `feat(cloud): 键盘工具栏加「记录中 / 已暂停」与暂停提示条`

---

## Task 5：走查截图与叠图

- [ ] **Step 1：** 在 `UITests/KeyboardShots.swift` 加 `testRecordingBadge`：要「登录 + 开了上传输入日志」的状态——看 `UITests/SpaceShots.swift` 与 `seed.py` 怎么造开通状态；造不出真登录就在 `seed.py` 加 `--recording` 选项，直接写一份带假令牌、`logs = true` 的 `cloud.toml` 到 App Group（上传会失败，但标记只看配置，够拍）。拍三张：`01-recording`（空闲工具栏带「● 记录中」）、`01-paused-banner`（点了之后，提示条 + 「已暂停」）、`01-paused-forever`（点「一直暂停」之后）。浅色 + 深色。
- [ ] **Step 2：叠图** 对 05 的 2a（工具栏那一条）与 2b（提示条那一条），只比键盘上沿 84pt 那一截（稿子的键位与提示行内容照约束 7 不同）；未对齐位移与残差分开写。
- [ ] **Step 3：** 真机：`scripts/install-device.sh` 装到 iPhone，打几句字后看 `input-log.jsonl` 的行数，点「记录中」暂停后再打几句，行数不变；点「已暂停」恢复后再打，行数增加（App Group 文件可用 Xcode 的 Devices 窗口下载容器查看）。这一步写进 PR「怎么验证的」，做不到就写明没做。

---

## Task 6：文档与 PR

- [ ] UI 清单：05 的 2a / 2b 改「已实现」，写清对应文件；约束 7 加一条写上面四处偏离。
- [ ] `cloud/docs/design.md` 讲上传的那一节后面加一段：「记录中」= 记输入日志并上传；暂停只停记、落盘在 `cloud/recording-pause.json`、一直暂停不碰同意开关。
- [ ] 推送，开 PR 到 `sujian`；把 PR 链接、截图与叠图路径、真机验证结果发回「claude design」会话。
