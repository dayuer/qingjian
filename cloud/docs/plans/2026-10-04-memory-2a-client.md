# 素笺 2A 本地记忆：客户端实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.
> 本文件是审计会话给出的**任务大纲**：接口、文件、测试与验收已定死。执行前由客户端会话用 writing-plans 把每个任务展开成逐步的代码与命令（写在本文件各任务下），展开后先发审计会话审一遍再动手。
> **完成状态（2026-10-04 收尾，`sujian` 1927f4a 之后）：** Task 1–6 已完成：`ScopedLearner`（`OVERLAY_WEIGHT = 4`）、`MemoryStore`（flock + 每对象 rev）、`HintIndex`、Session 与 C 接口、iOS 键盘、iOS App「键盘记住的事」（2655129 等，读写在后台 actor，失败一律有中文提示，数据保护 completeUntilFirstUserAuthentication）。
> Task 7 只做了文档：本计划已按代码修订；**回放 example（`examples/overlay_replay.rs`）没有提交**，下面 Task 7 的数字是展开时在临时副本里跑的，`OVERLAY_WEIGHT` 仍是 4、未按回放复核；`cloud/docs/design.md`「本地记忆」一节与 `cloud/README.md` 功能清单**未写**。
> 真机：App 与键盘并发写（Debug「连续保存 20 次」）待用户真机结果。接着做时从 Task 7 的回放 example 起。
> **完全访问方案（2026-10-04 真机定）：** 没开完全访问时键盘读不到 App 写进 App Group 的卡片。所以记忆功能（对象、提示行、对象卡、记一笔）要求开完全访问；不开时素笺就是普通输入法，打字照常（满足审核指南 4.4.1：不开完全访问也能用基本功能）。
> **键盘里新建对象（2026-10-04 用户要求，已做）：** 选择面板「新对象」不再只提示去 App，改成在提示行位置打名字、点「好了」直接建（桥 `qj_memory_add_contact`，经 `put_contact` 在锁里写名单、建目录；称呼先按 TA，App 里改）；锁被 App 占着或满 8 个时原因显示在输入条里。原「键盘只读名单、写只在 App」的约定就此放开这一处。
> **Task 8（未开始）：没开完全访问时的说明与引导。**
> - 键盘：没开完全访问时点牌子，显示「开启完全访问后才能用记忆。开了也不联网，卡片只在这台手机上」与「去开启」。键盘扩展打不开系统设置、也不许用 responder chain 打开 App，所以「去开启」只展开路径说明（设置 → 通用 → 键盘 → 键盘 → 素笺 → 允许完全访问），不跳转。
> - App：首页「记住的」与首次引导写明同一段话；能从 App 里用 `UIApplication.openSettingsURLString` 跳到素笺的设置页。
> - 设计稿 05 的 2c、2d 文案由审计会话改，落地时以改后的稿为准；文案进 `ScopeDisplay` / App 的 Wording，单测锁住。
> **真机并发验证（待做）：** `QJ_CONFIG=Debug cloud/ios/scripts/install-device.sh` 装 Debug 包；App 里对象详情最底下「调试 · 连续保存 20 次」（只在 `#if DEBUG`）。
> 步骤：开完全访问 → 复制一段字 → 备忘录里用素笺选「恋爱 · 某人」，出现「记到 某人」先别点 → 回 App 点「连续保存 20 次」→ 立刻回备忘录，每复制一段新字点一次「记到」→ 回 App 等脚注「完成 20 次，失败 N 次」。
> 核对：`idevicesyslog -m 'save #' -m '记一笔'` 看两边笔数，出现一次 `lockBusy=true` 或 `conflicts>0` 即算碰到并发；`xcrun devicectl device copy from --domain-type appGroupDataContainer --domain-identifier group.app.qingjian.cloud --source "Library/Application Support/Qingjian/memory"` 拷回 cards.json，20 张压测卡与键盘记的笔一张不少、rev 单调递增。验完装回 Release。
> **展开状态（2026-10-04）：** 7 个任务已展开。每个任务先保留大纲原文（接口、测试清单、验收），其下是「与大纲的差异」（有的话）与逐步的步骤。核实结果在下面「展开前核实」一节，需要审计会话拍板的在文末「需要审计会话决定的点」。

**Goal:** 免费版的本地记忆：场景、对象、手动记忆卡、分区学习、键盘提示行与对象卡、App「键盘记住的事」，不联网、不登录。

**Architecture:** 全部在 `cloud/` 下，不加上游补丁。桥里新增 `scope/`（包装学习器 `ScopedLearner`）与 `memory/`（存储、提示匹配、C 接口）；iOS 键盘加牌子、选择面板、提示行、对象卡面板；App 加「键盘记住的事」一组页面。

**Tech Stack:** Rust（qingjian-cloud-bridge、qingjian-core 的 `Learner` trait、qingjian-learning 的 `FrequencyLearner`）、Swift / SwiftUI（cloud/ios）、XcodeGen。

**Spec:** `synon-ime` 仓库 `docs/superpowers/specs/2026-10-04-memory-design.md`「2A 本地记忆」（分支 `sujian-memory-docs`）。UI：Claude Design「关系记忆 · 移动端 UI」01、02、05（2c、2g、2i）。

---

## 展开前核实（以代码为准，2026-10-04 在 `sujian` 分支上查的；审阅后修订时同步到 4b60e13：proto 的卡片类型 22086d7、卡片上限常量 4b60e13）

1. **`Context`、`UserNgram` 的导出：** `qingjian_core` 根上没有，但 `pub mod sentence` 里有 `pub use context::Context;`、`pub use user_ngram::UserNgram;`（`crates/qingjian-core/src/sentence/mod.rs:27,37`）。
   桥里写 `use qingjian_core::sentence::{Context, UserNgram};`，与 `qingjian-learning` 的 `frequency_learner/mod.rs:11` 同一写法，**不需要上游补丁**。`Forgotten`、`Learner`、`Candidate`、`CandidateKind` 在根上导出；`Dictionary`、`WordList` 用桥已依赖的 `qingjian_dictionary`。
2. **`Learner` trait 全部方法**（`crates/qingjian-core/src/engine/learning/learner.rs`，共 22 个，只有 `record`、`weight` 没有缺省实现）：

   | # | 签名 | `FrequencyLearner` 的行为 | `ScopedLearner` 转发到 |
   |---|---|---|---|
   | 1 | `fn record(&mut self, candidate: &Candidate)` | `counts[text] += 1` | 恋爱：场景 + 对象；其余：全局 |
   | 2 | `fn weight(&self, text: &str) -> u32` | 读 `counts` | 全局 + k×场景 + k×对象 |
   | 3 | `fn record_choice(&mut self, input: &str, text: &str)` | `choices[input][text] += 1`，超 5 万条减半 | 同 1 |
   | 4 | `fn choice_weight(&self, input: &str, text: &str) -> u32` | 读 `choices` | 同 2 |
   | 5 | `fn record_raw(&mut self, input: &str)` | `record_choice(input, "<raw>")` | 同 1 |
   | 6 | `fn raw_count(&self, input: &str) -> u32` | `choice_weight(input, "<raw>")` | 同 2 |
   | 7 | `fn unrecord(&mut self, text: &str)` | `counts` 减 1，到 0 删 | 同 1 |
   | 8 | `fn unrecord_choice(&mut self, input: &str, text: &str)` | `choices` 减 1 | 同 1 |
   | 9 | `fn unrecord_transition(&mut self, context: Context<'_>, word: &str, times: u32)` | 个人 n-gram 减 | 全局 |
   | 10 | `fn learn_word(&mut self, text: &str, syllables: &[String])` | 加用户词、重建小词库 | 全局 |
   | 11 | `fn user_words(&self) -> Option<&Dictionary>` | 用户词小词库 | 全局 |
   | 12 | `fn learn_english(&mut self, word: &str)` | 个人英文词 +1 | 全局 |
   | 13 | `fn user_english(&self) -> Option<&WordList>` | 个人英文词表 | 全局 |
   | 14 | `fn record_transition(&mut self, context: Context<'_>, word: &str, times: u32)` | 个人 n-gram 加 | 全局 |
   | 15 | `fn user_ngram(&self) -> Option<&UserNgram>` | 非空时给引用 | 全局 |
   | 16 | `fn record_typo(&mut self, typed: &str, intended: &str)` | 敲错表 +1 | 同 1 |
   | 17 | `fn unrecord_typo(&mut self, typed: &str, intended: &str)` | 敲错表 -1 | 同 1 |
   | 18 | `fn typo_count(&self, typed: &str, intended: &str) -> u32` | 读敲错表 | 同 2 |
   | 19 | `fn forget(&mut self, text: &str) -> Forgotten` | 删用户词、计数、各输入串选择、n-gram | 全局（见文末决定点 1） |
   | 20 | `fn forget_english(&mut self, word: &str) -> bool` | 删个人英文词 | 全局 |
   | 21 | `fn merge_remote(&mut self, inbox: &str) -> usize` | `merge_inbox_and_flush`（合并后只刷自己） | 全局 |
   | 22 | `fn flush(&mut self)` | 各表脏了就原子写 | 三层都刷 |

   - **`MutedLearner` 的套法：** `Engine` 的 `learner` 字段就是 `MutedLearner`（`engine/learning/muted.rs`，`pub(in crate::engine)`），`Engine::with_learner(Box<dyn Learner>)` 调 `MutedLearner::replace` 把我们的学习器装进去。私密输入、关学习时它吞写、照常读，**包装层不另做私密判断**（spec 原话）。注意 `MutedLearner` 自己没转发 `merge_remote`，但会话的收件箱走 `Engine::learner_mut()`，它返回的是 `MutedLearner::inner_mut()`（即我们的 `ScopedLearner`），不经过 `MutedLearner`，所以合并不会被吞。
   - **`Engine::learner_mut()` 存在**（`engine/setup.rs:510`）：`pub fn learner_mut(&mut self) -> &mut dyn Learner`，调用时先 `forget_span_cache()`。切换场景后「作废格子缓存」的正确调用就是 `self.engine.learner_mut();`（丢弃返回值）。`Engine::commit` 内部本来就会 `forget_span_cache()`。
   - **它只给 `&mut dyn Learner`，没法向下转型成 `ScopedLearner`**，trait 也没有换层的方法（加了就是上游补丁）。所以叠加层放在 `Arc<Mutex<Overlay>>` 里，`ScopedLearner` 与会话手里的 `ScopeHandle` 共享（见 Task 1「与大纲的差异」）。
   - **`Session` 现在的构造**（`cloud/crates/qingjian-cloud-bridge/src/session/mod.rs:62-107`）：`Session::open(data_dir, user_dir, config, cloud)` 里 `user_dir.map_or_else(FrequencyLearner::default, load_learner)`，`load_learner` 读 `<user_dir>/user.tsv`；`Engine::new(dictionary).with_learner(Box::new(learner))`。`Session` 只持有 `engine: Engine`，学习器在引擎里。Task 4 把这一行换成 `LiveMemory::open(dir)` 返回的 `ScopedLearner`，`user_dir` 为空时仍用 `FrequencyLearner::default()`（只在内存里学，不分区）。
3. **C 接口的会话约定：** 现有接口都显式带会话指针（`char *qj_preedit(QjSession *session)`、`char *qj_commit(QjSession *session, uint32_t index)`，Rust 侧 `*mut Session`，经 `lib.rs` 的 `with()` 折空指针与 panic），没有全局会话。所以 `qj_scope_*` 与键盘用的 `qj_memory_hint/dismiss/cards/note` **带 `QjSession *session`**；`qj_memory_read/write` 像 `qj_settings_*` / `qj_account_*` 那样按路径传。参数名照头文件现有写法叫 `session`（大纲里的 `s` 改成 `session`）。
4. **上屏路径：** 全在 `session/mod.rs` 的 `impl Session`：`commit(&mut self, index) -> Option<String>`（137 行）、`take_raw(&mut self) -> String`（153 行）、`punctuate(&mut self, c) -> String`（160 行）、`note_passthrough(&mut self, c)`（171 行）；`refresh()`（181 行）在没组字时提前 `return`。iOS 的 `typeSymbol` 不经过桥（123 层的标点直接 `output.commit`），表情、剪贴板插入也不经过桥，这几类不会进最近 24 字（够用：提示只看汉字词）。
   最近 24 字的缓冲挂在 `Session` 的新字段 `memory: Option<LiveMemory>` 里（`LiveMemory.recent: RecentText`）。
   **私密输入：** `qj_set_private` → `Session::set_private`（`session/cloud.rs:61`）→ `Engine::set_private`；判断用 `self.engine.is_private()`（`cloud.rs:56,76,89` 都是这个写法）。私密时上屏文字不进缓冲、不匹配、`memory_hint` 恒为 `None`。
5. **iOS 现状：**
   - `KeyboardView`（`Keyboard/Sources/KeyboardView.swift`）是 `VStack { CandidateBar; 键区 / 面板 }`，键区高度 `keyAreaHeight` 固定；**键盘总高度不在 SwiftUI 里，而是 `KeyboardViewController.mountKeyboard()` 钉在 `view.heightAnchor` 上的约束**（`candidateBarHeight + keyAreaHeight`，优先级 `.defaultHigh`），触摸层 `KeyTouchView` 的 `keyArea` 在 `viewDidLayoutSubviews` 里按 `y = candidateBarHeight` 算，⌄ 的范围在 `syncTouchView` 里按 `y = 0` 算。提示行要改这三处（见 Task 5 差异）。
   - `IdleBar` 是没组字时的候选栏：私密锁 / 剪贴板提示 / 润色条 / `actions`（润色 + 发到其他设备）；牌子与「记一笔」放进 `actions` 左侧与右侧，确认条作为新的一个分支。
   - `KeyboardModel` 是 `@MainActor @Observable`，`private(set)` 状态 + `refresh()` / `poll()`；面板由 `panel: KeyboardPanel`（`keys / candidates / emoji`）切，键区换成面板时触摸层自动清空格子（`syncTouchView` 里 `panel == .keys` 才给格子），SwiftUI 面板自己收点击（`onKeyboardTap` / `onKeyboardPress`）。
   - `CandidateBar*`：`CandidateBar` 组字时是横向候选，否则交给 `IdleBar`；提示行不放在它里面，放在 `KeyboardView` 的 VStack 顶上。
   - `Engine.swift` 的会话指针是 `private nonisolated(unsafe) let session`，`take` 与 `withOptionalCString` 也是 `private`；`MemoryBridge.swift` 做成 `extension Engine` 要把这三个放宽到模块内可见。
   - App：`QingjianApp` → `SetupView`（一个 `NavigationStack { Form }`，含「启用键盘」「设置」（键盘设置、账号两个 `NavigationLink`）「试一试」）。Tab 化的最小改法：`SetupView.body` 换成 `TabView`，原来的 `Form` 原样搬进第三个 Tab「我」。
   - `project.yml`：各 target 的 `sources` 是目录（`App`、`Shared`、`Keyboard/Sources`、`Tests`），**xcodegen 按目录自动收录**，新文件不用逐个列；`.xcassets` 放进 `App/` 也会自动当资源。测试 target `QingjianCloudTests` 的 `sources` 是 `Tests` 目录加两个键盘文件，依赖 `QingjianCloud`（`@testable import QingjianCloud`）；现有 `Tests/AccountDecodeTests.swift`、`Tests/AccountStoreTests.swift`。工程里没有共享 scheme，`xcodebuild -scheme QingjianCloud` 用的是 Xcode 自动生成的 scheme（账号计划里已这么用过）。
6. **日期：** 桥的 `Cargo.toml` 没有 `chrono` / `time`（`time` 只作为别的 crate 的传递依赖出现在 `Cargo.lock`）。按要求不引依赖：新类型 `LocalDate`（Unix 秒加 8 小时按天取整，公历换算用 Howard Hinnant 的 days_from_civil / civil_from_days），自带测试。大纲里的 `NaiveDate` 全部换成 `LocalDate`。
7. **`FrequencyLearner` 的布局：** `FrequencyLearner::from_path(path: impl Into<PathBuf>) -> Result<Self, LearningError>`，`path` 是**词频文件** `user.tsv` 本身，其余五张表用 `with_file_name` 放同目录：`user-words.tsv`、`user-ngram.tsv`、`user-choices.tsv`、`user-english.tsv`、`user-typos.tsv`（`frequency_learner/mod.rs:19-34`）。文件不在返回空表；只有真 io 错误才 `Err`。落盘走 `qingjian_core::storage::write_atomic`，**不建父目录**，所以分区层打开前要 `create_dir_all`。
   分区层路径：场景 `memory/scene-<场景>/learning/user.tsv`、对象 `memory/<对象 id>/learning/user.tsv`；`memory/` 在学习数据目录下（iOS 开了完全访问时就是 App Group 的 `Qingjian/`，与 spec 的路径一致），所以**不改 `qj_session_open` 的签名**。学习数据同步（`qingjian-cloud-client` 的 `Snapshot::read_dir`）只认学习数据目录顶层的这六个文件名，不会扫到 `memory/` 下的分区层，与 spec「只同步全局层」一致，不用另做处理。

其他核实到的、影响写法的事实：

- `qingjian-cloud-proto` 已有 `Scene { Daily, Dating, Work }`（`serde(rename_all = "lowercase")`，`Copy + Hash`），但没有 `as_str` / `parse`；桥直接用它，不再定义一个同名枚举。
- 展开期间 proto 又合进了 2C 的卡片类型（22086d7）：`CardKind { Date, Promise, Preference, Recent, Other }`、`CardSource { Cloud, Manual }`，serde 都是小写，与 spec 的 `kind` / `source` 取值一致，`Copy`，`CardSource` 没有 `Default`。桥的 `Card` 直接用这两个枚举，不另定义（2C 下发的卡进同一套本地结构时不用转换）。
- `cloud_config.rs` 的原子写在 `CloudConfig::save` 里（临时名 `<文件>.<pid>.<序号>.tmp`、0600、改名），写锁是私有的 `lock()`。Task 2 把写文件的部分抽成 `pub(crate) fn write_atomic(path, bytes, create_parent)`（记忆文件不建父目录），`lock` 不动；记忆的读-改-写改用 `memory/.lock` 的跨进程文件锁（审计阻断项）。toolchain 是 1.96（`rust-toolchain.toml`），`std::fs::File::try_lock`（1.89 稳定）可直接用，不加 `libc`。
- proto 4b60e13 有 `MAX_CARD_TEXT_CHARS = 200`、`MAX_CARD_KEYWORDS = 8`，手写卡校验直接用。
- 桥现在**没有**「导出符号与头文件逐个核对」的测试，`tests/ffi.rs` 只是按 C 签名声明并调用（能链接就说明导出了）。展开时用 shell 核过一遍：`src/` 里的 `extern "C" fn qj_*` 与头文件里的 `qj_*(` 完全一致。Task 4 把这个核对写成测试 `header_declares_every_export`。
- `Dictionary::from_path` 按文件头魔数认格式，TSV 起名 `dict.qj` 也能读；FFI 测试用仓库里的 `assets/sample/dict.tsv`（含「生日」`sheng ri`），不需要产品数据。
- `Engine::language_model() -> &dyn LanguageModel` 是公开的；`qingjian_core::sentence::segment_text(text, model)` 按语言模型切词（模型一个词都不认识时返回 `None`）。上游的 `Dictionary` 只能按拼音查、不能按文字查，所以「用引擎词库切分」落实为用语言模型切（见 Task 3 差异）。
- `getrandom 0.4.3` 已经作为 `uuid` 的依赖在 `Cargo.lock` 里（iOS 上能编，`uuid` 在用），`cargo add getrandom@0.4` 不引入新包；API 是 `getrandom::fill(&mut [u8]) -> Result<(), getrandom::Error>`。
- 提交钩子在仓库根，`cloud/` 是独立 workspace：每个任务提交前在 `cloud/` 下手动跑 `cargo fmt --all` 与 `cargo clippy -p qingjian-cloud-bridge --all-targets -- -D warnings`。

**顺序与编译状态：** Task 1–3 是纯 Rust，各自可独立提交，每次提交 `cargo test -p qingjian-cloud-bridge` 全绿。Task 4 只**新增** C 函数、不改已有签名，iOS 工程在 Task 4 提交后照样能编（Swift 不用新函数也不报错；`build-bridge.sh` 在 preBuildScript 里会重编桥），所以 Task 4、5 不必连着做。Task 5、6 都要先跑 `scripts/build-bridge.sh` 让 xcframework 带上新头文件。Task 6 依赖 Task 5 建的 `Shared/Memory/` 模型（两个 target 共用）。

---

## 文件结构

```
cloud/crates/qingjian-cloud-bridge/src/
  scope/mod.rs            Scene 枚举、ScopeState（当前场景 + 对象）、路径约定
  scope/scoped_learner.rs ScopedLearner：三层 FrequencyLearner，实现 Learner 全部方法
  scope/tests.rs
  memory/mod.rs           re-export、MEMORY_DIR 等常量
  memory/contact.rs       Contact { id, name, pronoun, scene, created_at }、Pronoun
  memory/card.rs          Card { id, kind, text, keywords, when, source, confirmed, created_at, touched_at }、CardKind、CardSource
  memory/store.rs         MemoryStore：读写 contacts.json / cards.json / state.json，原子写，坏文件改名备份
  memory/hint.rs          HintIndex：建索引、match_text、today_reminders、节流
  memory/ffi.rs           qj_scope_* / qj_memory_* C 接口
  memory/tests.rs
  session/mod.rs          （改）Session 持有 ScopedLearner 与 HintIndex，上屏路径喂给提示
include/qingjian_bridge.h （改）新函数声明
cloud/ios/Keyboard/Sources/
  ScopeChip.swift         工具栏牌子
  ScopePicker.swift       键位区里的场景 / 对象选择面板
  HintRow.swift           候选栏上方提示行
  ContactCardPanel.swift  键盘内对象卡面板
  MemoryBridge.swift      调桥的 qj_scope_* / qj_memory_*
cloud/ios/App/Memory/
  MemoryHomeView.swift    「键盘记住的事」首页
  ContactDetailView.swift 对象详情（按 kind 分组）
  CardEditor.swift        新建 / 编辑卡片
  ContactEditor.swift     新建对象（名字、称呼、已知的事）
  ContactSettingsView.swift 名字、称呼、提示开关、导出、忘掉这个人
  WeekView.swift          「本周」：7 天内的日子与约定
  MemoryStore.swift       App 侧经桥 JSON 整份读写
cloud/ios/Tests/MemoryStoreTests.swift
```

### 展开后的文件结构（以此为准；按「一个类型一个文件」「同词干收进目录」「测试多了按主题分文件」拆开）

```
cloud/crates/qingjian-cloud-bridge/
  Cargo.toml                    （改，Task 2）加 getrandom = "0.4.3"
  src/lib.rs                    （改）mod scope / memory；pub use；with() 改 pub(crate)
  src/cloud_config.rs           （改，Task 2）抽出 pub(crate) write_atomic（带 create_parent）
  src/scope/mod.rs              scene_name / parse_scene / is_contact_id / 路径约定 / load_layer / lock
  src/scope/overlay.rs          Overlay：当前叠加层（场景层 + 对象层）
  src/scope/handle.rs           ScopeHandle：会话换层的把手
  src/scope/scoped_learner.rs   ScopedLearner：实现 Learner 全部 22 个方法
  src/scope/state.rs            ScopeState（state.json）
  src/scope/tests.rs
  src/memory/mod.rs             常量、new_id、now_unix、has_date、re-export
  src/memory/contact.rs         Contact
  src/memory/pronoun.rs         Pronoun
  src/memory/card.rs            Card（种类与来源用 proto 的 CardKind、CardSource）
  src/memory/error.rs           MemoryError（code / message / to_json）
  src/memory/local_date.rs      LocalDate（北京时间日历日）
  src/memory/snapshot.rs        MemorySnapshot（App 整份读写的 JSON，带 revs）
  src/memory/cards_file.rs      CardsFile（cards.json 的外层：rev + cards）
  src/memory/dismissed_file.rs  DismissedFile（dismissed.json）
  src/memory/store.rs           MemoryStore（文件锁、修订号、读-改-写）
  src/memory/recent.rs          RecentText（最近 24 字）            （Task 3）
  src/memory/stopwords.txt      100 个停用词                        （Task 3）
  src/memory/hint/mod.rs        常量、停用词、reminder_text、panel_cards（Task 3）
  src/memory/hint/entry.rs      IndexedCard（索引里的一张卡）        （Task 3）
  src/memory/hint/index.rs      HintIndex                           （Task 3）
  src/memory/hint/item.rs       Hint                                （Task 3）
  src/memory/hint/reason.rs     HintReason                          （Task 3）
  src/memory/ffi.rs             9 个 C 函数（含 qj_reset_context）  （Task 4）
  src/memory/tests/mod.rs       测试用的临时目录、样例对象与卡片
  src/memory/tests/date.rs      LocalDate
  src/memory/tests/store.rs     MemoryStore / MemorySnapshot / 卡片上限
  src/memory/tests/sync.rs      两个进程同时写、读不了不覆盖、忘掉的人不复活、「知道了」记录
  src/memory/tests/hint.rs      HintIndex / RecentText / panel_cards（Task 3）
  src/session/mod.rs            （改，Task 4）memory 字段、上屏路径喂缓冲、refresh 后更新提示
  src/session/cloud.rs          （改，Task 4）poll 里按修改时间重载记忆
  src/session/memory/mod.rs     impl Session：set_scope / scope / memory_hint / dismiss_hint / memory_cards / memory_note …（Task 4）
  src/session/memory/live.rs    LiveMemory：会话里的记忆状态（Task 4）
  include/qingjian_bridge.h     （改，Task 4）
  tests/memory_ffi.rs           （Task 4）
  examples/overlay_replay.rs    叠加权重回放（Task 7）
cloud/ios/
  project.yml                   （改，Task 6）App 显示名「素笺」、AppIcon
  Shared/Memory/MemoryContact.swift  MemoryPronoun.swift  MemoryCard.swift  MemoryScope.swift
  Shared/Memory/MemorySnapshot.swift MemoryHint.swift     MemoryFailure.swift MemoryDate.swift
  Shared/Memory/MemoryID.swift       MemoryFiles.swift    MemoryAvatar.swift   MemoryLimits.swift（Task 5，App 与键盘共用）
  Keyboard/Sources/ScopeChip.swift ScopePicker.swift HintRow.swift ContactCardPanel.swift MemoryBridge.swift（Task 5）
  Keyboard/Sources/Engine.swift KeyboardModel.swift KeyboardView.swift IdleBar.swift KeyboardPanel.swift
  Keyboard/Sources/KeyStyle.swift KeyboardViewController.swift                         （改，Task 5）
  App/Memory/MemoryStore.swift MemoryMerge.swift MemoryHomeView.swift ContactDetailView.swift CardEditor.swift
  App/Memory/ContactEditor.swift ContactSettingsView.swift WeekView.swift CloudIntroView.swift（Task 6）
  App/SetupView.swift            （改，Task 6）
  App/Assets.xcassets/            Contents.json、AppIcon.appiconset/（三张 1024 图 + Contents.json）（Task 6）
  App/Info.plist、Keyboard/Info.plist（xcodegen 按 project.yml 重写，显示名「素笺」，Task 6）
  Tests/MemoryStoreTests.swift   （Task 6）
  README.md                      （改，Task 7）
cloud/docs/design.md、cloud/README.md、cloud/docs/fork-patch.md（改，Task 7）
```

## Task 1：`ScopedLearner`（分区学习）

**Files:** Create `scope/mod.rs`、`scope/scoped_learner.rs`、`scope/tests.rs`；Modify `src/lib.rs`（mod 声明）。

接口：

```rust
pub enum Scene { Daily, Dating, Work }            // as_str: "daily" / "dating" / "work"
pub struct ScopedLearner { global: FrequencyLearner, scene: Option<FrequencyLearner>, contact: Option<FrequencyLearner> }
impl ScopedLearner {
    pub const OVERLAY_WEIGHT: u32 = 4;
    pub fn open(user_dir: &Path, memory_dir: &Path, scene: Scene, contact: Option<&str>) -> Self;
}
impl qingjian_core::Learner for ScopedLearner { /* 全部方法，见下表 */ }
```

| 方法 | 读 / 写到哪层 |
|---|---|
| `weight`、`choice_weight`、`raw_count`、`typo_count` | 全局 + 4×场景 + 4×对象（恋爱场景才有后两层） |
| `record`、`record_choice`、`record_raw`、`record_typo` 及对应 `unrecord*` | 恋爱：场景 + 对象；日常 / 工作：全局 |
| `learn_word`、`user_words`、`learn_english`、`user_english`、`record_transition`、`unrecord_transition`、`user_ngram`、`forget*`、`merge_remote` | 一律全局 |
| `flush` | 三层都刷 |

测试（`scope/tests.rs`，用临时目录，不需要产品数据）：
- `dating_writes_do_not_reach_work`：恋爱场景 `record` 一个候选 10 次，换到工作场景 `weight` 为 0；换回恋爱同一对象 `weight` 为 40。
- `contacts_are_isolated`：对象 A 下记录，对象 B 下 `weight` 只含场景层（4×10），不含 A 的对象层。
- `daily_and_work_share_global`：日常记录，工作场景能读到。
- `forwarding_is_complete`：`merge_remote`、`user_words`、`flush` 在三种场景下都作用于全局（落盘后检查 `user*.tsv`）。
- `flush_writes_all_layers`：恋爱记录后 flush，场景与对象目录下都有 `user.tsv`。

验收：`cargo test -p qingjian-cloud-bridge scope`、clippy 无告警。动手前先核实 `Context`、`UserNgram` 是否由 qingjian_core 公开导出（`record_transition` / `user_ngram` 签名要用）；没导出就在本任务里写明改用的办法，不加上游补丁。

### 与大纲的差异

1. **`Scene` 不新定义**，直接用 `qingjian_cloud_proto::Scene`（提交 b1355c8 已有，serde 名就是 `daily/dating/work`）；proto 里没有 `as_str`，桥里写自由函数 `scope::scene_name(Scene) -> &'static str` 与 `scope::parse_scene(&str) -> Option<Scene>`（不改 proto，免得碰服务端也在用的 crate）。
2. **结构体字段改了：** `ScopedLearner { global: FrequencyLearner, overlay: Arc<Mutex<Overlay>>, weight: u32, memory_dir: PathBuf }`，`Overlay { scene: Option<FrequencyLearner>, contact: Option<FrequencyLearner> }`。
   理由：`Engine::learner_mut()` 只给 `&mut dyn Learner`，没有向下转型的口子，`Learner` 也没有换层的方法；不加上游补丁就只能让会话另拿一个共享的把手。新增 `ScopeHandle`（`handle()` 取得，`switch(scene, contact)` 换层）。锁只有键盘主线程在拿，不会争用。
3. **新增 `ScopedLearner::open_with_weight(..., weight)`**：只给 Task 7 的回放调参用，产品路径 `open` 固定用 `OVERLAY_WEIGHT`。
4. **测试数字按 spec 的公式改：** 恋爱同一对象下记 10 次，读到的是 `0 + 4×10（场景）+ 4×10（对象）= 80`，不是大纲写的 40；换到对象 B 是 `4×10 = 40`（与大纲一致）。测试里写成 `2 * K * 10` 与 `K * 10`（`K = OVERLAY_WEIGHT`），Task 7 若改权重测试不用跟着改。
5. **`ScopeState` 放在 `scope/state.rs`**（一个类型一个文件），只有 `scene`、`contact_id` 两个字段（审计修订：提示开关按人设置，放在 `Contact` 上，见 Task 2）。旧文件里多出的字段忽略。
6. 多出两个文件 `scope/overlay.rs`、`scope/handle.rs`，以及 `scope/mod.rs` 里的 `is_contact_id`（对象 id 当目录名，要挡 `../`）。
7. **审计修订（决定点 1、5 与重要 2）**，与大纲的表不同的三处：
   - **删词连叠加层一起删：** `forget` / `forget_english` 先删全局，再对当前打开的场景层与对象层各删一遍，结果取并（`Forgotten` 两个字段按「或」合起来）。
   - **恋爱场景不记词序列转移：** `record_transition` / `unrecord_transition` 在有叠加层（恋爱场景）时什么都不做，读（`user_ngram`）照常读全局。理由是暧昧的话不该在工作场景的整句里冒出来；代价是恋爱场景的句子不帮整句学习（已知限制，文末「需要同步进 spec 的内容」）。
   - **对象层只在对象目录还在时打开、不建目录：** `Overlay::open` 判断 `memory/<id>/` 存在才开对象层（`learning/` 子目录照旧由 `load_layer` 建）；对象目录只在建对象时创建（Task 2 `put_contact` / `write_snapshot`），忘掉的人不会因为键盘还选着它而被重新建出来。测试里的 `dirs()` 因此先建好 A、B 两个对象目录。

### 步骤

**Files:**
- Create: `cloud/crates/qingjian-cloud-bridge/src/scope/{mod,overlay,handle,scoped_learner,state,tests}.rs`
- Modify: `cloud/crates/qingjian-cloud-bridge/src/lib.rs:6-27`（mod 声明与 re-export）

- [ ] **Step 1: 写失败的测试**

Create `cloud/crates/qingjian-cloud-bridge/src/scope/tests.rs`：

```rust
//! 分区学习：恋爱场景的写不进全局、对象之间互不相通、计数类读三层加权、其余方法读全局、恋爱场景不记转移、
//! 删词连叠加层一起删、落盘三层都刷、对象目录不在就不开对象层（也不重建）。

use std::path::PathBuf;

use qingjian_cloud_proto::Scene;
use qingjian_core::sentence::Context;
use qingjian_core::{Candidate, CandidateKind, Learner};

use super::{
    ScopeState, ScopedLearner, contact_learning_dir, is_contact_id, parse_scene,
    scene_learning_dir, scene_name,
};

const A: &str = "0123456789abcdef0123456789abcdef";
const B: &str = "fedcba9876543210fedcba9876543210";
const K: u32 = ScopedLearner::OVERLAY_WEIGHT;

fn candidate(text: &str) -> Candidate {
    Candidate {
        text: text.to_owned(),
        kind: CandidateKind::Chinese,
        syllables: Vec::new(),
        reading: None,
        translation: None,
        aux_code: None,
    }
}

/// 每个测试一个临时的学习数据目录，记忆目录在它下面的 `memory/`；对象 A、B 的目录先建好（建对象时才有）。
fn dirs(name: &str) -> (PathBuf, PathBuf) {
    let user = std::env::temp_dir().join(format!("qj-scope-{name}-{}", std::process::id()));
    std::fs::remove_dir_all(&user).ok();
    let memory = user.join("memory");
    std::fs::create_dir_all(memory.join(A)).unwrap();
    std::fs::create_dir_all(memory.join(B)).unwrap();
    (user, memory)
}

#[test]
fn dating_writes_do_not_reach_work() {
    let (user, memory) = dirs("dating-work");
    let mut learner = ScopedLearner::open(&user, &memory, Scene::Dating, Some(A));
    let handle = learner.handle();
    for _ in 0..10 {
        learner.record(&candidate("宝贝"));
    }
    // 恋爱场景读「全局 + k×场景 + k×对象」：全局没写，场景层与对象层各 10 次
    assert_eq!(learner.weight("宝贝"), 2 * K * 10);
    handle.switch(Scene::Work, None);
    assert_eq!(learner.weight("宝贝"), 0, "工作场景只读全局");
    handle.switch(Scene::Dating, Some(A));
    assert_eq!(
        learner.weight("宝贝"),
        2 * K * 10,
        "换层时落过盘，换回同一对象读得到"
    );
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn contacts_are_isolated() {
    let (user, memory) = dirs("contacts");
    let mut learner = ScopedLearner::open(&user, &memory, Scene::Dating, Some(A));
    let handle = learner.handle();
    for _ in 0..10 {
        learner.record(&candidate("宝贝"));
    }
    handle.switch(Scene::Dating, Some(B));
    assert_eq!(learner.weight("宝贝"), K * 10, "对象 B 只读到场景层");
    handle.switch(Scene::Dating, None);
    assert_eq!(learner.weight("宝贝"), K * 10, "不指定对象也只有场景层");
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn daily_and_work_share_global() {
    let (user, memory) = dirs("daily-work");
    let mut learner = ScopedLearner::open(&user, &memory, Scene::Daily, None);
    let handle = learner.handle();
    for _ in 0..3 {
        learner.record(&candidate("开会"));
    }
    assert_eq!(learner.weight("开会"), 3);
    handle.switch(Scene::Work, None);
    assert_eq!(learner.weight("开会"), 3);
    handle.switch(Scene::Dating, None);
    assert_eq!(learner.weight("开会"), 3, "恋爱场景也读全局，叠加层是空的");
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn counts_overlay_for_choices_raw_and_typos() {
    let (user, memory) = dirs("counts");
    let mut learner = ScopedLearner::open(&user, &memory, Scene::Dating, Some(A));
    learner.record_choice("bb", "宝贝");
    learner.record_raw("bb");
    learner.record_typo("bv", "bei");
    assert_eq!(learner.choice_weight("bb", "宝贝"), 2 * K);
    assert_eq!(learner.raw_count("bb"), 2 * K);
    assert_eq!(learner.typo_count("bv", "bei"), 2 * K);
    learner.unrecord_choice("bb", "宝贝");
    learner.unrecord_typo("bv", "bei");
    assert_eq!(learner.choice_weight("bb", "宝贝"), 0);
    assert_eq!(learner.typo_count("bv", "bei"), 0);
    learner.record(&candidate("宝贝"));
    learner.unrecord("宝贝");
    assert_eq!(learner.weight("宝贝"), 0);
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn forwarding_is_complete() {
    for scene in [Scene::Daily, Scene::Dating, Scene::Work] {
        let (user, memory) = dirs(&format!("forward-{}", scene_name(scene)));
        let contact = (scene == Scene::Dating).then_some(A);
        let mut learner = ScopedLearner::open(&user, &memory, scene, contact);
        assert_eq!(learner.merge_remote("user\tadd\t开发\t3\n"), 1, "{scene:?}");
        learner.learn_word("青简", &["qing".to_owned(), "jian".to_owned()]);
        assert!(learner.user_words().is_some(), "{scene:?}");
        learner.learn_english("gist");
        assert!(learner.user_english().is_some(), "{scene:?}");
        learner.flush();
        let read = |name: &str| std::fs::read_to_string(user.join(name)).unwrap_or_default();
        assert!(read("user.tsv").contains("开发"), "{scene:?}");
        assert!(read("user-words.tsv").contains("青简"), "{scene:?}");
        assert!(read("user-english.tsv").contains("gist"), "{scene:?}");
        assert!(learner.forget("开发").learning, "{scene:?}");
        assert!(learner.forget_english("gist"), "{scene:?}");
        assert_eq!(learner.weight("开发"), 0, "{scene:?}");
        std::fs::remove_dir_all(&user).ok();
    }
}

#[test]
fn dating_does_not_write_transitions() {
    for scene in [Scene::Daily, Scene::Dating, Scene::Work] {
        let (user, memory) = dirs(&format!("transition-{}", scene_name(scene)));
        let contact = (scene == Scene::Dating).then_some(A);
        let mut learner = ScopedLearner::open(&user, &memory, scene, contact);
        learner.record_transition(Context::START, "你好", 1);
        assert_eq!(
            learner.user_ngram().is_some(),
            scene != Scene::Dating,
            "{scene:?}"
        );
        std::fs::remove_dir_all(&user).ok();
    }
    // 日常记下的转移，恋爱场景照样读得到、也不会被恋爱场景撤掉
    let (user, memory) = dirs("transition-read");
    let mut learner = ScopedLearner::open(&user, &memory, Scene::Daily, None);
    learner.record_transition(Context::START, "你好", 1);
    learner.handle().switch(Scene::Dating, Some(A));
    learner.unrecord_transition(Context::START, "你好", 1);
    assert!(learner.user_ngram().is_some());
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn forget_clears_the_open_overlay_layers() {
    let (user, memory) = dirs("forget");
    let mut learner = ScopedLearner::open(&user, &memory, Scene::Dating, Some(A));
    learner.record(&candidate("宝贝"));
    learner.learn_english("honey");
    assert_eq!(learner.weight("宝贝"), 2 * K);
    let forgotten = learner.forget("宝贝");
    assert!(forgotten.learning);
    assert_eq!(learner.weight("宝贝"), 0, "场景层与对象层一起删");
    assert!(learner.forget_english("honey"));
    assert!(!learner.forget_english("honey"));
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn missing_contact_dir_is_not_recreated() {
    let (user, memory) = dirs("missing");
    let gone = "11111111111111111111111111111111";
    let mut learner = ScopedLearner::open(&user, &memory, Scene::Dating, Some(gone));
    learner.record(&candidate("宝贝"));
    learner.flush();
    assert_eq!(learner.weight("宝贝"), K, "对象目录不在，只有场景层");
    assert!(!memory.join(gone).exists());
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn flush_writes_all_layers() {
    let (user, memory) = dirs("flush");
    let mut learner = ScopedLearner::open(&user, &memory, Scene::Dating, Some(A));
    learner.record(&candidate("宝贝"));
    learner.flush();
    assert!(
        scene_learning_dir(&memory, Scene::Dating)
            .join("user.tsv")
            .is_file()
    );
    assert!(contact_learning_dir(&memory, A).join("user.tsv").is_file());
    assert!(!user.join("user.tsv").exists(), "恋爱场景不写全局");
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn bad_contact_ids_are_ignored_and_scenes_parse() {
    assert!(is_contact_id(A));
    assert!(!is_contact_id("../etc"));
    assert!(!is_contact_id(&A.to_uppercase()));
    assert_eq!(parse_scene("party"), None);
    for scene in [Scene::Daily, Scene::Dating, Scene::Work] {
        assert_eq!(parse_scene(scene_name(scene)), Some(scene));
    }
    let (user, memory) = dirs("bad-id");
    let mut learner = ScopedLearner::open(&user, &memory, Scene::Dating, Some("../x"));
    learner.record(&candidate("宝贝"));
    assert_eq!(
        learner.weight("宝贝"),
        K,
        "不合格的 id 当没选对象，只有场景层"
    );
    assert!(!memory.join("..").join("x").join("learning").exists());
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn scope_state_defaults() {
    let state: ScopeState =
        serde_json::from_str(r#"{"scene":"dating","contact_id":null,"hints":false}"#).unwrap();
    assert_eq!(state.scene, Scene::Dating, "旧文件里多出的开关字段忽略");
    assert_eq!(state.contact_id, None);
    assert_eq!(ScopeState::default().scene, Scene::Daily);
}
```

- [ ] **Step 2: 挂上空模块，跑测试看它失败**

Create `cloud/crates/qingjian-cloud-bridge/src/scope/mod.rs`（临时只挂测试，Step 3 整个替换）：

```rust
//! 场景与对象的分区学习。

#[cfg(test)]
mod tests;
```

Modify `cloud/crates/qingjian-cloud-bridge/src/lib.rs`：第 6–13 行的 `mod` 列表里按字母序加一行 `mod scope;`（在 `mod rewrite;` 与 `mod session;` 之间）。

Run: `cd /Users/liyuqing/sproot/qingjian-mainline/cloud && cargo test -p qingjian-cloud-bridge scope 2>&1 | tail -20`
Expected: 编译失败，`error[E0432]: unresolved imports super::ScopeState, super::ScopedLearner …`。

- [ ] **Step 3: 写 `scope/mod.rs`**

Replace `cloud/crates/qingjian-cloud-bridge/src/scope/mod.rs`：

```rust
//! 场景与对象的分区学习（spec「2A 本地记忆 · 分区学习」）：[`ScopedLearner`] 包三层 `FrequencyLearner`，
//! 会话用 [`ScopeHandle`] 换叠加层，当前场景与对象存在 [`ScopeState`]（`memory/state.json`）。
//! 目录约定：场景层 `memory/scene-<场景>/learning/`，对象层 `memory/<对象 id>/learning/`，文件名与全局层一样是 `user*.tsv`。

mod handle;
mod overlay;
mod scoped_learner;
mod state;

#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, PoisonError};

use qingjian_cloud_proto::Scene;
use qingjian_learning::FrequencyLearner;

use self::overlay::Overlay;

pub use self::handle::ScopeHandle;
pub use self::scoped_learner::ScopedLearner;
pub use self::state::ScopeState;

/// 场景在路径与 JSON 里的名字，与 proto 的 serde 名一致。
pub fn scene_name(scene: Scene) -> &'static str {
    match scene {
        Scene::Daily => "daily",
        Scene::Dating => "dating",
        Scene::Work => "work",
    }
}

pub fn parse_scene(text: &str) -> Option<Scene> {
    [Scene::Daily, Scene::Dating, Scene::Work]
        .into_iter()
        .find(|scene| scene_name(*scene) == text)
}

/// 对象 id 是 16 字节随机数的小写十六进制；它要拿来当目录名，不合格的一律不认（挡 `../` 之类）。
pub fn is_contact_id(id: &str) -> bool {
    id.len() == 32
        && id
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

pub fn scene_learning_dir(memory_dir: &Path, scene: Scene) -> PathBuf {
    memory_dir
        .join(format!("scene-{}", scene_name(scene)))
        .join("learning")
}

pub fn contact_learning_dir(memory_dir: &Path, contact_id: &str) -> PathBuf {
    memory_dir.join(contact_id).join("learning")
}

/// 打开一层：目录不在就建（`FrequencyLearner` 落盘时不建父目录）；读不了（锁屏时的数据保护、权限）
/// 退回只在内存里学，不拿空表覆盖用户文件。
pub(crate) fn load_layer(dir: &Path) -> FrequencyLearner {
    if let Err(error) = std::fs::create_dir_all(dir) {
        tracing::warn!(%error, "学习数据目录建不了，这一层只在内存里学习");
        return FrequencyLearner::default();
    }
    let path = dir.join("user.tsv");
    FrequencyLearner::from_path(&path).unwrap_or_else(|error| {
        tracing::error!(path = %path.display(), %error, "学习数据读取失败，这一层只在内存里学习");
        FrequencyLearner::default()
    })
}

/// 叠加层的锁：只有键盘主线程在拿；中毒了也照用里面的数据。
fn lock(overlay: &Mutex<Overlay>) -> MutexGuard<'_, Overlay> {
    overlay.lock().unwrap_or_else(PoisonError::into_inner)
}
```

- [ ] **Step 4: 写 `scope/overlay.rs` 与 `scope/handle.rs`**

Create `cloud/crates/qingjian-cloud-bridge/src/scope/overlay.rs`：

```rust
//! 当前生效的叠加层：只在恋爱场景有；场景层必有，对象层看选没选对象。

use std::path::Path;

use qingjian_cloud_proto::Scene;
use qingjian_core::Learner;
use qingjian_learning::FrequencyLearner;

use super::{contact_learning_dir, is_contact_id, load_layer, scene_learning_dir};

#[derive(Debug, Default)]
pub struct Overlay {
    scene: Option<FrequencyLearner>,

    contact: Option<FrequencyLearner>,
}

impl Overlay {
    /// 日常与工作没有叠加层；恋爱场景开场景层，给了合格的对象 id、且对象目录还在时再开对象层。
    /// 对象目录只由建对象时创建，这里不建：忘掉的人不会因为键盘还选着它而被重新建出来。
    pub fn open(memory_dir: &Path, scene: Scene, contact: Option<&str>) -> Self {
        if scene != Scene::Dating {
            return Self::default();
        }
        let contact = contact
            .filter(|id| is_contact_id(id) && memory_dir.join(id).is_dir())
            .map(|id| load_layer(&contact_learning_dir(memory_dir, id)));
        Self {
            scene: Some(load_layer(&scene_learning_dir(memory_dir, scene))),
            contact,
        }
    }

    /// 有叠加层（恋爱场景）时，写只进叠加层。
    pub fn active(&self) -> bool {
        self.scene.is_some()
    }

    /// 各叠加层的计数乘 `weight` 再相加。
    pub fn count(&self, weight: u32, read: impl Fn(&FrequencyLearner) -> u32) -> u32 {
        [&self.scene, &self.contact]
            .into_iter()
            .flatten()
            .fold(0, |sum, layer| {
                sum.saturating_add(weight.saturating_mul(read(layer)))
            })
    }

    pub fn write(&mut self, mut f: impl FnMut(&mut FrequencyLearner)) {
        for layer in [&mut self.scene, &mut self.contact].into_iter().flatten() {
            f(layer);
        }
    }

    pub fn flush(&mut self) {
        self.write(|layer| layer.flush());
    }
}
```

Create `cloud/crates/qingjian-cloud-bridge/src/scope/handle.rs`：

```rust
//! 会话手里的换层把手。引擎拿走了 `Box<dyn Learner>`，`Engine::learner_mut()` 只给 `&mut dyn Learner`、没法向下转型，
//! 所以叠加层放在与 [`super::ScopedLearner`] 共享的 `Arc<Mutex<_>>` 里，由这里换。

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use qingjian_cloud_proto::Scene;

use super::{Overlay, lock};

#[derive(Debug, Clone)]
pub struct ScopeHandle {
    overlay: Arc<Mutex<Overlay>>,

    memory_dir: PathBuf,
}

impl ScopeHandle {
    pub(super) fn new(overlay: Arc<Mutex<Overlay>>, memory_dir: PathBuf) -> Self {
        Self {
            overlay,
            memory_dir,
        }
    }

    /// 换到 `scene` / `contact`：旧叠加层先落盘再从磁盘开新的，换回同一对象时读得到刚记的。
    /// 换完调用方要调一次 `Engine::learner_mut()`，作废格子缓存里按旧叠加层排的候选。
    pub fn switch(&self, scene: Scene, contact: Option<&str>) {
        let mut overlay = lock(&self.overlay);
        overlay.flush();
        *overlay = Overlay::open(&self.memory_dir, scene, contact);
    }
}
```

- [ ] **Step 5: 写 `scope/scoped_learner.rs` 与 `scope/state.rs`**

Create `cloud/crates/qingjian-cloud-bridge/src/scope/scoped_learner.rs`：

```rust
//! 分区学习器：全局层（学习数据目录的 `user.tsv`）之外，恋爱场景再叠场景层与对象层。
//! 计数类读三层加权求和、写只进叠加层；用户词、个人 n-gram、英文词表要返回引用，没法现场叠加，一律读全局。
//! 恋爱场景不记词序列转移（个人 n-gram）：暧昧的话不该在工作场景的整句里冒出来。删词连当前打开的叠加层一起删。
//! 包装层必须逐个转发 `Learner` 的全部方法，漏一个就会被 trait 的缺省实现悄悄吞掉。私密输入由外面的 `MutedLearner` 挡写。

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use qingjian_cloud_proto::Scene;
use qingjian_core::sentence::{Context, UserNgram};
use qingjian_core::{Candidate, Forgotten, Learner};
use qingjian_dictionary::{Dictionary, WordList};
use qingjian_learning::FrequencyLearner;

use super::{Overlay, ScopeHandle, load_layer, lock};

pub struct ScopedLearner {
    global: FrequencyLearner,

    overlay: Arc<Mutex<Overlay>>,

    /// 叠加层的倍数：产品里是 [`Self::OVERLAY_WEIGHT`]，回放调参时经 [`Self::open_with_weight`] 换。
    weight: u32,

    memory_dir: PathBuf,
}

impl ScopedLearner {
    /// 叠加层计数的倍数（spec 定 4，`examples/overlay_replay.rs` 回放调）。
    pub const OVERLAY_WEIGHT: u32 = 4;

    pub fn open(user_dir: &Path, memory_dir: &Path, scene: Scene, contact: Option<&str>) -> Self {
        Self::open_with_weight(user_dir, memory_dir, scene, contact, Self::OVERLAY_WEIGHT)
    }

    /// 同 [`Self::open`]，叠加倍数自定。只给回放调参用，不进产品配置。
    pub fn open_with_weight(
        user_dir: &Path,
        memory_dir: &Path,
        scene: Scene,
        contact: Option<&str>,
        weight: u32,
    ) -> Self {
        Self {
            global: load_layer(user_dir),
            overlay: Arc::new(Mutex::new(Overlay::open(memory_dir, scene, contact))),
            weight,
            memory_dir: memory_dir.to_path_buf(),
        }
    }

    /// 会话换场景、换对象用的把手。
    pub fn handle(&self) -> ScopeHandle {
        ScopeHandle::new(Arc::clone(&self.overlay), self.memory_dir.clone())
    }

    fn count(&self, read: impl Fn(&FrequencyLearner) -> u32) -> u32 {
        read(&self.global).saturating_add(lock(&self.overlay).count(self.weight, &read))
    }

    /// 恋爱场景写进叠加层（不写全局），日常与工作写全局。
    fn write(&mut self, mut f: impl FnMut(&mut FrequencyLearner)) {
        let mut overlay = lock(&self.overlay);
        if overlay.active() {
            overlay.write(&mut f);
        } else {
            f(&mut self.global);
        }
    }
}

impl Learner for ScopedLearner {
    fn record(&mut self, candidate: &Candidate) {
        self.write(|layer| layer.record(candidate));
    }

    fn weight(&self, text: &str) -> u32 {
        self.count(|layer| layer.weight(text))
    }

    fn record_choice(&mut self, input: &str, text: &str) {
        self.write(|layer| layer.record_choice(input, text));
    }

    fn choice_weight(&self, input: &str, text: &str) -> u32 {
        self.count(|layer| layer.choice_weight(input, text))
    }

    fn record_raw(&mut self, input: &str) {
        self.write(|layer| layer.record_raw(input));
    }

    fn raw_count(&self, input: &str) -> u32 {
        self.count(|layer| layer.raw_count(input))
    }

    fn unrecord(&mut self, text: &str) {
        self.write(|layer| layer.unrecord(text));
    }

    fn unrecord_choice(&mut self, input: &str, text: &str) {
        self.write(|layer| layer.unrecord_choice(input, text));
    }

    fn unrecord_transition(&mut self, context: Context<'_>, word: &str, times: u32) {
        if !lock(&self.overlay).active() {
            self.global.unrecord_transition(context, word, times);
        }
    }

    fn learn_word(&mut self, text: &str, syllables: &[String]) {
        self.global.learn_word(text, syllables);
    }

    fn user_words(&self) -> Option<&Dictionary> {
        self.global.user_words()
    }

    fn learn_english(&mut self, word: &str) {
        self.global.learn_english(word);
    }

    fn user_english(&self) -> Option<&WordList> {
        self.global.user_english()
    }

    fn record_transition(&mut self, context: Context<'_>, word: &str, times: u32) {
        if !lock(&self.overlay).active() {
            self.global.record_transition(context, word, times);
        }
    }

    fn user_ngram(&self) -> Option<&UserNgram> {
        self.global.user_ngram()
    }

    fn record_typo(&mut self, typed: &str, intended: &str) {
        self.write(|layer| layer.record_typo(typed, intended));
    }

    fn unrecord_typo(&mut self, typed: &str, intended: &str) {
        self.write(|layer| layer.unrecord_typo(typed, intended));
    }

    fn typo_count(&self, typed: &str, intended: &str) -> u32 {
        self.count(|layer| layer.typo_count(typed, intended))
    }

    fn forget(&mut self, text: &str) -> Forgotten {
        let mut forgotten = self.global.forget(text);
        lock(&self.overlay).write(|layer| {
            let more = layer.forget(text);
            forgotten.user_word |= more.user_word;
            forgotten.learning |= more.learning;
        });
        forgotten
    }

    fn forget_english(&mut self, word: &str) -> bool {
        let mut found = self.global.forget_english(word);
        lock(&self.overlay).write(|layer| found |= layer.forget_english(word));
        found
    }

    fn merge_remote(&mut self, inbox: &str) -> usize {
        self.global.merge_remote(inbox)
    }

    fn flush(&mut self) {
        self.global.flush();
        lock(&self.overlay).flush();
    }
}
```

Create `cloud/crates/qingjian-cloud-bridge/src/scope/state.rs`：

```rust
//! `memory/state.json`：键盘当前的场景与对象（键盘写，App 不改）。提示开关在各个对象上（`Contact`），不在这里。

use qingjian_cloud_proto::Scene;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ScopeState {
    pub scene: Scene,

    /// 当前对象；只在恋爱场景有。
    pub contact_id: Option<String>,
}

impl Default for ScopeState {
    fn default() -> Self {
        Self {
            scene: Scene::Daily,
            contact_id: None,
        }
    }
}
```

- [ ] **Step 6: lib.rs 导出**

Modify `cloud/crates/qingjian-cloud-bridge/src/lib.rs`：在 `pub use self::rewrite::{RewriteState, Rewriter};`（第 25 行）之后加：

```rust
pub use self::scope::{
    ScopeHandle, ScopeState, ScopedLearner, contact_learning_dir, is_contact_id, parse_scene,
    scene_learning_dir, scene_name,
};
```

（`parse_scene` 等在 Task 4 之前只有测试用，不导出会报 dead_code；导出后 Task 7 的 example 也要用 `ScopedLearner`、`ScopeHandle`。）

- [ ] **Step 7: 跑测试看它通过**

Run: `cd /Users/liyuqing/sproot/qingjian-mainline/cloud && cargo test -p qingjian-cloud-bridge scope 2>&1 | tail -15`
Expected: `test result: ok. 11 passed; 0 failed`（`scope::tests::` 下 11 个：大纲的 5 个，加计数类、恋爱不记转移、删词连叠加层、对象目录不重建、坏 id、`ScopeState` 六个）。

- [ ] **Step 8: 全量测试、格式与 clippy**

Run: `cd /Users/liyuqing/sproot/qingjian-mainline/cloud && cargo fmt --all && cargo test -p qingjian-cloud-bridge 2>&1 | grep "test result" && cargo clippy -p qingjian-cloud-bridge --all-targets -- -D warnings 2>&1 | tail -3`
Expected: 每个测试二进制都是 `test result: ok`；clippy 末行 `Finished`，没有 `warning` / `error`。

- [ ] **Step 9: 提交**

```bash
cd /Users/liyuqing/sproot/qingjian-mainline
git add cloud/crates/qingjian-cloud-bridge/src/scope cloud/crates/qingjian-cloud-bridge/src/lib.rs
git commit -m "feat(cloud): 桥加分区学习 ScopedLearner

恋爱场景在全局之外叠场景层与对象层：计数类读三层加权（k=4），写只进叠加层；用户词、n-gram、英文词表要返回引用，一律读全局。
恋爱场景不记词序列转移（暧昧的话不进工作场景的整句）；删词连当前打开的叠加层一起删；对象目录不在就不开对象层、也不重建。
Engine::learner_mut 只给 &mut dyn Learner，不加上游补丁就没法换层，叠加层放进 Arc<Mutex<_>>，会话拿 ScopeHandle 换。
逐个转发 Learner 的 22 个方法，flush 三层都刷。

```

## Task 2：记忆存储 `MemoryStore`

**Files:** Create `memory/{mod,contact,card,store,tests}.rs`。

接口：

```rust
pub struct MemoryStore { root: PathBuf }   // <user_dir>/memory
impl MemoryStore {
    pub fn open(user_dir: &Path) -> Self;
    pub fn contacts(&self) -> Vec<Contact>;
    pub fn put_contact(&self, c: Contact) -> Result<(), MemoryError>;       // 同场景超过 8 个返回 MemoryError::ContactLimit
    pub fn forget_contact(&self, id: &str) -> Result<(), MemoryError>;      // 删 memory/<id>/ 整个目录并从 contacts.json 去掉
    pub fn cards(&self, contact_id: &str) -> Vec<Card>;
    pub fn put_cards(&self, contact_id: &str, cards: &[Card]) -> Result<(), MemoryError>;
    pub fn state(&self) -> ScopeState;  pub fn put_state(&self, s: &ScopeState) -> Result<(), MemoryError>;
}
```

- JSON 用 serde，字段名与 spec 一致；`id` 由 `getrandom` 生成 16 字节十六进制。
- 原子写（同目录临时文件 + `rename`，临时名带 pid 与序号，复用 `cloud_config.rs` 的写法与进程内写锁）。
- 坏文件：解析失败时改名 `<文件>.broken-<unix秒>`，返回空并记 `tracing::warn!`。
- iOS 数据保护在 Swift 侧给目录设 `.completeUntilFirstUserAuthentication`（Task 6），Rust 不管：桥建的子目录与原子写的临时文件继承所在目录的保护级别。

测试：往返；8 个上限（恋爱第 9 个报 `ContactLimit`，日常不计）；`forget_contact` 后目录不存在；坏 JSON 被改名且返回空；并发 put（两个线程各写 50 次）后文件可解析。

### 与大纲的差异

1. **按「一个类型一个文件」拆：** `Pronoun` 单独 `memory/pronoun.rs`；**`CardKind`、`CardSource` 不在桥里定义，直接用 `qingjian_cloud_proto` 的**（22086d7 刚合进，serde 名与 spec 一致），`memory/card.rs` 里只有 `Card`，「日子与约定才有日期」写成 `memory/mod.rs` 的自由函数 `has_date(CardKind)`（外部枚举上加不了方法）；`Card.source` 因此没有 `#[serde(default)]`（proto 的 `CardSource` 没有 `Default`，桥写出的文件总带这个字段）；另有 `memory/error.rs`（`MemoryError`）、`memory/local_date.rs`（`LocalDate`，`when` 的校验要用，Task 3 的提醒也用）、`memory/snapshot.rs`（`MemorySnapshot`）。测试超过 200 行，按主题放 `memory/tests/{mod,date,store}.rs`（Task 3 再加 `hint.rs`）。
2. **`MemoryStore` 的接口按审计修订扩了一圈**（大纲的七个方法都在，`put_state` 换成了 `update_scope`）：
   - `snapshot() -> Result<MemorySnapshot, _>`、`write_snapshot(&MemorySnapshot)`：Task 4 的 `qj_memory_read/write` 用；
   - `try_contacts` / `try_cards` / `try_state`：返回 `Result` 的读法，读-改-写一律用它们（大纲的 `contacts()` / `cards()` / `state()` 留着，只给显示用，读不了给空）；
   - `add_note(contact_id, text, now)`：「记一笔」的读-改-写放进存储（锁里按**磁盘上的名单**判断这个人还在、读卡片失败就不写）；
   - `update_scope(scene, contact_id)`：键盘切场景，锁里重读 `state.json` 只改场景与对象；
   - `dismissed(today, known)` / `put_dismissed(..)`：「知道了」落盘到 `memory/dismissed.json`（决定点 4）；
   - `stamp(contact_id)`（键盘按修改时间重载用）、`root()`。
3. **两个进程同时写（审计阻断项）：**
   - **跨进程文件锁：** 每个操作（读也算）都拿 `memory/.lock` 的 flock：toolchain 是 1.96（`rust-toolchain.toml`），`std::fs::File::try_lock` 自 1.89 稳定，不用加 `libc`。`try_lock` 失败每 5 毫秒重试，缺省最多等 2 秒（App），超时返回 `MemoryError::LockTimeout`（不套在 `Io` 里，上层好区分），测试里不会死锁。flock 锁的是打开的文件，同一进程里两个 `MemoryStore` 也互斥，所以双进程测试用两个线程各持一个实例模拟。不再借用 `cloud.toml` 的进程内写锁。
   - **修订号：** `cards.json` 改成 `{"rev": n, "cards": [...]}`（`memory/cards_file.rs` 的 `CardsFile`），每写一次加一；早期的光秃秃数组读成修订号 0。快照带 `revs`（对象 id → 读时的修订号）。`write_snapshot` 在锁里先逐个比：磁盘比快照新就**整份不写**，返回 `MemoryError::Conflict`（C 接口 `code = "conflict"`）；没冲突时只重写卡片有变化的对象。
   - `write_snapshot` 不采纳快照里的 `state`（键盘写的为准），当前对象被删就置空。App 收到 `conflict` 的合并规则在 Task 6（`MemoryMerge`）。
4. **`MemorySnapshot.broken`**：读快照时卡片文件坏了、已改名备份的对象 id 列表，App 据此提示「这个人的记忆文件损坏，已备份」（spec「2A 的错误与边界」第一条，大纲没写接口）。
3. **`MemorySnapshot.broken`**：读快照时卡片文件坏了、已改名备份的对象 id 列表，App 据此提示「这个人的记忆文件损坏，已备份」（spec「2A 的错误与边界」第一条，大纲没写接口）。
5. **读失败分两种（审计重要 1）：** 解析失败才改名备份、按空处理；io 错误（锁屏时数据保护挡住、权限）不改名，读-改-写的操作直接返回 `MemoryError::Io`、什么都不写。`snapshot()` 有一个文件读不了就整份报错（App 不能拿半份数据去写回）。测试用 chmod 000 模拟读不了。
6. **`new_id()` 返回 `Result<String, MemoryError>`**：`getrandom::fill` 理论上会失败，不 panic。依赖用 `cargo add getrandom@0.4`（`Cargo.lock` 里已有 0.4.3，`uuid` 在用，不引新包）。
7. `cloud_config.rs` 抽出 `pub(crate) fn write_atomic(path, bytes, create_parent)`：大纲说的「复用写法」落实为两边调同一个函数（记忆文件因此也是 0600）。**记忆文件一律 `create_parent = false`（审计重要 2）**：父目录不在就报错，对象目录只在 `put_contact` / `write_snapshot`（建对象）时显式创建，忘掉的人的目录不会被写卡片重新建出来；`cloud.toml` 照旧 `true`。`cloud_config` 的进程内写锁不动。
8. 8 个上限只数恋爱场景（`scene == Dating`）：spec「对象只在恋爱场景」，大纲测试也写「日常不计」。
9. **`Contact` 加 `hint_on`、`remind_on`**（决定点 3，`#[serde(default = "true_default")]`，旧文件按开）；`ScopeState` 不再有提示开关。
10. **`LocalDate::next_anniversary`**（决定点 2）：日子类按年重复用，2 月 29 日在平年算 2 月 28 日。
11. 新文件：`memory/cards_file.rs`、`memory/dismissed_file.rs`、`memory/tests/sync.rs`；`memory/mod.rs` 多一个自由函数 `sanitized_scope`（会话与存储共用的「对象不在名单上就退回不指定」）。
12. **手写卡的上限（审计会话定的卡片契约，与 2C 一致）：** 文字非空、最多 `MAX_CARD_TEXT_CHARS = 200` 字（按 `chars().count()`，中文、emoji 一个算一个），关键词最多 `MAX_CARD_KEYWORDS = 8` 个、每个去掉首尾空白后 2–8 字（spec 对云端卡关键词的校验，手写卡一并按它）；`put_cards`、`add_note`、`write_snapshot` 写入前都校验，不合格返回 `MemoryError::Invalid`（C 接口 `code = "invalid"`）。两个常量用 `qingjian_cloud_proto` 的（提交 4b60e13 加的）——**Task 2 开工前确认 `cloud/crates/qingjian-cloud-proto/src/lib.rs` 里有 `MAX_CARD_TEXT_CHARS` 与 `MAX_CARD_KEYWORDS`，没有就先 `git pull` 到含 4b60e13 的 `sujian`**；关键词的 2 与 8 是桥里的 `MIN_KEYWORD_CHARS` / `MAX_KEYWORD_CHARS`（proto 里没有）。
13. **`Card` 多 `faded`、`seq`、`updated_at` 三个字段**（`#[serde(default)]`，与 proto 的 `MemoryCard` 同名同语义：`seq` 每个用户单独递增，`updated_at` 是 Unix 毫秒），2A 手写卡恒为 `false` / 0 / 0，App 整份写回时原样带着，2C 下发的卡进同一套本地结构时不丢字段。
14. **键盘侧等锁最多 200 毫秒（审计会话追加）：** 锁超时是 `MemoryStore` 的构造参数：`MemoryStore::open(user_dir)` 缺省 `DEFAULT_LOCK_TIMEOUT`（2 秒，App 用），`MemoryStore::open_with_lock_timeout(user_dir, Duration)`，键盘会话用 `KEYBOARD_LOCK_TIMEOUT`（200 毫秒）；超时返回独立的 `MemoryError::LockTimeout`（`code = "lock_timeout"`）。**键盘侧「拿不到锁就进内存待办、下次 refresh 重试」属于 Task 4（Session 接入）**，这里只提供可配置超时与错误；测试 `keyboard_lock_wait_times_out_quickly`（`tests/sync.rs`）：一个线程持锁，200 毫秒超时的实例写操作在 150–500 毫秒内返回 `LockTimeout`，锁放了之后同一实例的写成功。

### 步骤

**Files:**
- Modify: `cloud/crates/qingjian-cloud-bridge/Cargo.toml`（`cargo add`）
- Modify: `cloud/crates/qingjian-cloud-bridge/src/cloud_config.rs:146-169`（`save` 后面加 `write_atomic`）
- Create: `cloud/crates/qingjian-cloud-bridge/src/memory/{mod,card,cards_file,contact,dismissed_file,pronoun,error,local_date,snapshot,store}.rs`
- Create: `cloud/crates/qingjian-cloud-bridge/src/memory/tests/{mod,date,store,sync}.rs`
- Modify: `cloud/crates/qingjian-cloud-bridge/src/lib.rs`

- [ ] **Step 1: 加依赖**

Run: `cd /Users/liyuqing/sproot/qingjian-mainline/cloud && cargo add getrandom@0.4 -p qingjian-cloud-bridge && grep -n getrandom crates/qingjian-cloud-bridge/Cargo.toml`
Expected: 输出 `getrandom = "0.4.3"`（或 `"0.4"`）一行；`git diff --stat Cargo.lock` 只多了桥对 `getrandom 0.4.3` 的一条依赖边，没有新包。

- [ ] **Step 2: 抽出原子写**

Modify `cloud/crates/qingjian-cloud-bridge/src/cloud_config.rs`：把第 146–163 行的 `save` 整个换成：

```rust
    /// 整份写回（这份文件只有这几项，不用保留注释）。里面有令牌，写法见 [`write_atomic`]。
    pub fn save(&self, path: &Path) -> Result<(), String> {
        let text = toml::to_string(self).map_err(|e| e.to_string())?;
        write_atomic(path, text.as_bytes(), true).map_err(|e| e.to_string())
    }
```

在第 171–175 行的 `fn lock()` 后面（`parse_failure_note` 之前）加：

```rust
/// 同目录写 `.tmp`（名字带进程号与序号，Unix 上 0600）再改名：读的一方（键盘）随时在读，不能读到写了一半的文件。
/// `cloud.toml` 与 `memory/` 下的文件都走这里。`create_parent` 为假时父目录不在就报错：记忆的对象目录只在建对象时创建，
/// 写卡片时不能把已经忘掉的人的目录重新建出来。
pub(crate) fn write_atomic(path: &Path, bytes: &[u8], create_parent: bool) -> std::io::Result<()> {
    if create_parent && let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    let serial = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    name.push(format!(".{}.{serial}.tmp", std::process::id()));
    let temp = path.with_file_name(name);
    write_private(&temp, bytes)
        .and_then(|()| std::fs::rename(&temp, path))
        .inspect_err(|_| {
            std::fs::remove_file(&temp).ok();
        })
}
```

Run: `cd /Users/liyuqing/sproot/qingjian-mainline/cloud && cargo test -p qingjian-cloud-bridge --test cloud_config 2>&1 | grep "test result"`
Expected: `test result: ok.`（行为没变，原有测试照过）。

- [ ] **Step 3: 写失败的测试**

Create `cloud/crates/qingjian-cloud-bridge/src/memory/tests/mod.rs`：

```rust
//! 本地记忆的单元测试，按主题分文件；这里放共用的临时目录与样例对象、卡片。

mod date;
mod store;
mod sync;

use std::path::PathBuf;

use qingjian_cloud_proto::{CardKind, CardSource, Scene};

use crate::memory::{Card, Contact, Pronoun};

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("qj-memory-{name}-{}", std::process::id()));
    std::fs::remove_dir_all(&dir).ok();
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// 第 `n` 个样例 id（32 位十六进制）。
fn id(n: u32) -> String {
    format!("{n:032x}")
}

fn contact(n: u32, scene: Scene) -> Contact {
    Contact {
        id: id(n),
        name: format!("人{n}"),
        pronoun: Pronoun::Ta,
        scene,
        created_at: 1_791_043_200,
        hint_on: true,
        remind_on: true,
    }
}

/// 卡片 id 取 `id(1000 + n)`，与对象 id 错开。
fn card(
    n: u32,
    kind: CardKind,
    text: &str,
    keywords: &[&str],
    when: Option<&str>,
    touched_at: i64,
) -> Card {
    Card {
        id: id(1000 + n),
        kind,
        text: text.to_owned(),
        keywords: keywords.iter().map(|k| (*k).to_owned()).collect(),
        when: when.map(str::to_owned),
        source: CardSource::Manual,
        confirmed: true,
        faded: false,
        seq: 0,
        updated_at: 0,
        created_at: 1_791_043_200,
        touched_at,
    }
}
```

Create `cloud/crates/qingjian-cloud-bridge/src/memory/tests/date.rs`：

```rust
//! 北京时间的日历日：Unix 秒加 8 小时、`YYYY-MM-DD` 的解析与往返、跨月跨年的天数。

use crate::memory::LocalDate;

#[test]
fn unix_seconds_use_beijing_time() {
    assert_eq!(LocalDate::from_unix(0).to_string(), "1970-01-01");
    assert_eq!(
        LocalDate::from_unix(16 * 3600 - 1).to_string(),
        "1970-01-01"
    );
    assert_eq!(LocalDate::from_unix(16 * 3600).to_string(), "1970-01-02");
    // 2026-10-04 00:00 北京时间 = 2026-10-03T16:00Z
    assert_eq!(
        LocalDate::from_unix(1_791_043_200).to_string(),
        "2026-10-04"
    );
    assert_eq!(
        LocalDate::from_unix(1_791_043_199).to_string(),
        "2026-10-03"
    );
}

#[test]
fn parses_only_full_dates() {
    assert!(LocalDate::parse("2024-02-29").is_some());
    assert!(LocalDate::parse("2023-02-29").is_none());
    assert!(LocalDate::parse("2026-13-01").is_none());
    assert!(LocalDate::parse("2026-1-05").is_none());
    assert!(LocalDate::parse("2026/01/05").is_none());
    assert!(LocalDate::parse("+026-01-05").is_none());
    assert!(LocalDate::parse("").is_none());
}

#[test]
fn counts_days_across_months_and_years() {
    let day = |text: &str| LocalDate::parse(text).unwrap();
    assert_eq!(day("2025-12-31").days_until(day("2026-01-01")), 1);
    assert_eq!(day("2026-03-01").days_until(day("2026-02-28")), -1);
    assert_eq!(day("2024-02-28").add_days(1).to_string(), "2024-02-29");
    assert_eq!(day("2026-10-04").add_days(3).to_string(), "2026-10-07");
}

#[test]
fn round_trips_through_text() {
    let epoch = LocalDate::from_unix(0);
    for offset in -700_000..700_000 {
        if offset % 997 != 0 {
            continue;
        }
        let date = epoch.add_days(offset);
        assert_eq!(LocalDate::parse(&date.to_string()), Some(date), "{date}");
    }
}

#[test]
fn anniversaries_repeat_every_year() {
    let day = |text: &str| LocalDate::parse(text).unwrap();
    let next = |when: &str, today: &str| day(when).next_anniversary(day(today)).to_string();
    assert_eq!(next("1998-10-05", "2026-10-04"), "2026-10-05");
    assert_eq!(next("1998-10-04", "2026-10-04"), "2026-10-04", "当天算今年");
    assert_eq!(next("1998-10-03", "2026-10-04"), "2027-10-03", "过了看明年");
    assert_eq!(
        next("2020-01-02", "2026-12-30"),
        "2027-01-02",
        "12 月 30 日看 1 月 2 日"
    );
    assert_eq!(
        next("2024-02-29", "2026-02-27"),
        "2026-02-28",
        "平年按 2 月 28 日"
    );
    assert_eq!(next("2024-02-29", "2026-03-01"), "2027-02-28");
    assert_eq!(next("2024-02-29", "2028-02-28"), "2028-02-29", "闰年照常");
}
```

（±700 000 天约是公元 54 年到 3886 年，年份都是四位，`to_string` 与 `parse` 能往返。）

Create `cloud/crates/qingjian-cloud-bridge/src/memory/tests/store.rs`：

```rust
//! 记忆存储：往返、恋爱场景 8 个上限、忘掉一个人、坏文件改名、并发写、App 整份写回的规则、旧格式的 cards.json。
//! 两个进程同时写、读失败不覆盖、忘掉的人不复活在 `sync.rs`。

use qingjian_cloud_proto::{CardKind, Scene};

use super::{card, contact, id, temp_dir};
use crate::memory::{MemoryError, MemorySnapshot, MemoryStore};
use crate::scope::ScopeState;

#[test]
fn round_trips_contacts_cards_and_state() {
    let user = temp_dir("round-trip");
    let store = MemoryStore::open(&user);
    assert!(store.contacts().is_empty());
    assert_eq!(store.state(), ScopeState::default());

    store.put_contact(contact(1, Scene::Dating)).unwrap();
    assert_eq!(store.contacts(), vec![contact(1, Scene::Dating)]);
    let mut renamed = contact(1, Scene::Dating);
    renamed.name = "小美".to_owned();
    store.put_contact(renamed.clone()).unwrap();
    assert_eq!(store.contacts(), vec![renamed], "同 id 是改，不是加");

    let cards = vec![card(
        1,
        CardKind::Date,
        "生日",
        &["生日"],
        Some("2026-10-05"),
        1,
    )];
    store.put_cards(&id(1), &cards).unwrap();
    assert_eq!(store.cards(&id(1)), cards);
    assert!(store.cards("../x").is_empty());

    let state = store.update_scope(Scene::Dating, Some(&id(1))).unwrap();
    assert_eq!(state.contact_id, Some(id(1)));
    assert_eq!(store.state(), state);
    let state = store.update_scope(Scene::Work, Some(&id(1))).unwrap();
    assert_eq!(state.contact_id, None, "非恋爱场景不带对象");
    let state = store.update_scope(Scene::Dating, Some(&id(9))).unwrap();
    assert_eq!(state.contact_id, None, "名单上没有的对象当不指定");
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn ninth_dating_contact_hits_the_limit() {
    let user = temp_dir("limit");
    let store = MemoryStore::open(&user);
    for n in 0..8 {
        store.put_contact(contact(n, Scene::Dating)).unwrap();
    }
    assert!(matches!(
        store.put_contact(contact(8, Scene::Dating)),
        Err(MemoryError::ContactLimit)
    ));
    store.put_contact(contact(9, Scene::Daily)).unwrap();
    store.put_contact(contact(0, Scene::Dating)).unwrap();
    assert_eq!(store.contacts().len(), 9, "日常不计数，改已有的不算新增");
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn forget_contact_removes_the_directory() {
    let user = temp_dir("forget");
    let store = MemoryStore::open(&user);
    store.put_contact(contact(1, Scene::Dating)).unwrap();
    store
        .put_cards(&id(1), &[card(1, CardKind::Other, "喜欢猫", &[], None, 1)])
        .unwrap();
    let dir = user.join("memory").join(id(1));
    assert!(dir.is_dir());
    store.forget_contact(&id(1)).unwrap();
    assert!(!dir.exists());
    assert!(store.contacts().is_empty());
    assert!(matches!(
        store.forget_contact("../x"),
        Err(MemoryError::Invalid(_))
    ));
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn broken_files_are_renamed_and_read_as_empty() {
    let user = temp_dir("broken");
    let store = MemoryStore::open(&user);
    let memory = user.join("memory");
    std::fs::create_dir_all(&memory).unwrap();
    std::fs::write(memory.join("contacts.json"), "{not json").unwrap();
    assert!(store.contacts().is_empty());
    let names: Vec<String> = std::fs::read_dir(&memory)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert!(
        names.iter().any(|n| n.starts_with("contacts.json.broken-")),
        "{names:?}"
    );
    assert!(!memory.join("contacts.json").exists());

    store.put_contact(contact(1, Scene::Dating)).unwrap();
    std::fs::create_dir_all(memory.join(id(1))).unwrap();
    std::fs::write(memory.join(id(1)).join("cards.json"), "[{").unwrap();
    let snapshot = store.snapshot().unwrap();
    assert_eq!(snapshot.broken, vec![id(1)]);
    assert!(snapshot.cards[&id(1)].is_empty());
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn concurrent_writes_leave_a_parseable_file() {
    let user = temp_dir("concurrent");
    let store = MemoryStore::open(&user);
    store.put_contact(contact(1, Scene::Dating)).unwrap();
    let threads: Vec<_> = (0..2u32)
        .map(|t| {
            let store = store.clone();
            std::thread::spawn(move || {
                for i in 0..50u32 {
                    let cards = vec![card(
                        t * 100 + i,
                        CardKind::Other,
                        "x",
                        &[],
                        None,
                        i64::from(i),
                    )];
                    store.put_cards(&id(1), &cards).unwrap();
                }
            })
        })
        .collect();
    for thread in threads {
        thread.join().unwrap();
    }
    let dir = user.join("memory").join(id(1));
    assert_eq!(store.try_cards(&id(1)).unwrap().len(), 1);
    assert_eq!(
        store.snapshot().unwrap().revs[&id(1)],
        100,
        "每写一次修订号加一"
    );
    let leftovers: Vec<String> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|name| name.ends_with(".tmp"))
        .collect();
    assert!(leftovers.is_empty(), "{leftovers:?}");
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn snapshot_write_replaces_all_but_the_current_scene() {
    let user = temp_dir("snapshot");
    let store = MemoryStore::open(&user);
    store.put_contact(contact(1, Scene::Dating)).unwrap();
    store.put_contact(contact(2, Scene::Dating)).unwrap();
    store
        .put_cards(&id(2), &[card(2, CardKind::Other, "x", &[], None, 1)])
        .unwrap();
    store.update_scope(Scene::Dating, Some(&id(2))).unwrap();

    let mut snapshot = store.snapshot().unwrap();
    snapshot.contacts.retain(|c| c.id == id(1));
    snapshot.cards.remove(&id(2));
    snapshot.cards.insert(
        id(1),
        vec![card(5, CardKind::Preference, "喜欢草莓", &[], None, 2)],
    );
    snapshot.state = ScopeState {
        scene: Scene::Work,
        contact_id: None,
    };
    store.write_snapshot(&snapshot).unwrap();

    assert_eq!(store.contacts(), vec![contact(1, Scene::Dating)]);
    assert!(
        !user.join("memory").join(id(2)).exists(),
        "名单上没了的人连目录一起删"
    );
    assert_eq!(store.cards(&id(1)).len(), 1);
    let state = store.state();
    assert_eq!(state.scene, Scene::Dating, "场景以键盘写的为准");
    assert_eq!(state.contact_id, None, "当前对象被删就退回不指定");
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn snapshot_write_only_rewrites_changed_contacts() {
    let user = temp_dir("changed");
    let store = MemoryStore::open(&user);
    store.put_contact(contact(1, Scene::Dating)).unwrap();
    store.put_contact(contact(2, Scene::Dating)).unwrap();
    store
        .put_cards(&id(1), &[card(1, CardKind::Other, "a", &[], None, 1)])
        .unwrap();
    store
        .put_cards(&id(2), &[card(2, CardKind::Other, "b", &[], None, 1)])
        .unwrap();
    let mut snapshot = store.snapshot().unwrap();
    snapshot.cards.get_mut(&id(1)).unwrap()[0].text = "a2".to_owned();
    store.write_snapshot(&snapshot).unwrap();
    let after = store.snapshot().unwrap();
    assert_eq!(after.revs[&id(1)], 2);
    assert_eq!(after.revs[&id(2)], 1, "没变的对象不重写");
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn legacy_card_arrays_read_as_revision_zero() {
    let user = temp_dir("legacy");
    let store = MemoryStore::open(&user);
    store.put_contact(contact(1, Scene::Dating)).unwrap();
    let legacy = vec![card(1, CardKind::Other, "旧格式", &[], None, 1)];
    std::fs::write(
        user.join("memory").join(id(1)).join("cards.json"),
        serde_json::to_string(&legacy).unwrap(),
    )
    .unwrap();
    let snapshot = store.snapshot().unwrap();
    assert_eq!(snapshot.cards[&id(1)], legacy);
    assert_eq!(snapshot.revs[&id(1)], 0);
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn snapshot_write_validates() {
    let user = temp_dir("validate");
    let store = MemoryStore::open(&user);
    let ok = MemorySnapshot {
        contacts: vec![contact(1, Scene::Dating)],
        ..MemorySnapshot::default()
    };

    let mut unknown = ok.clone();
    unknown.cards.insert(id(9), Vec::new());
    assert!(matches!(
        store.write_snapshot(&unknown),
        Err(MemoryError::Invalid(_))
    ));

    let mut bad_date = ok.clone();
    bad_date.cards.insert(
        id(1),
        vec![card(1, CardKind::Date, "生日", &[], Some("2026-13-01"), 0)],
    );
    assert!(matches!(
        store.write_snapshot(&bad_date),
        Err(MemoryError::Invalid(_))
    ));

    let mut bad_id = ok.clone();
    bad_id.contacts[0].id = "../x".to_owned();
    assert!(matches!(
        store.write_snapshot(&bad_id),
        Err(MemoryError::Invalid(_))
    ));

    let nine = MemorySnapshot {
        contacts: (0..9).map(|n| contact(n, Scene::Dating)).collect(),
        ..MemorySnapshot::default()
    };
    assert!(matches!(
        store.write_snapshot(&nine),
        Err(MemoryError::ContactLimit)
    ));

    store.write_snapshot(&ok).unwrap();
    assert_eq!(store.contacts().len(), 1);
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn errors_have_codes_for_swift() {
    let json: serde_json::Value =
        serde_json::from_str(&MemoryError::ContactLimit.to_json()).unwrap();
    assert_eq!(json["code"], "contact_limit");
    assert_eq!(json["message"], "恋爱场景最多 8 个人");
    assert_eq!(MemoryError::Invalid("x").code(), "invalid");
    assert_eq!(MemoryError::Conflict.code(), "conflict");
    assert_eq!(MemoryError::Io(std::io::Error::other("x")).code(), "io");
    assert_eq!(crate::memory::new_id().unwrap().len(), 32);
}

#[test]
fn card_limits_count_characters() {
    let user = temp_dir("limits");
    let store = MemoryStore::open(&user);
    store.put_contact(contact(1, Scene::Dating)).unwrap();
    let text_ok = |text: &str| {
        store
            .put_cards(&id(1), &[card(1, CardKind::Other, text, &[], None, 1)])
            .is_ok()
    };
    assert!(text_ok(&"字".repeat(200)));
    assert!(!text_ok(&"字".repeat(201)));
    assert!(text_ok(&"😀".repeat(200)), "emoji 一个算一个字");
    assert!(!text_ok(&"😀".repeat(201)));
    assert!(text_ok(&format!("{}{}", "a".repeat(100), "中".repeat(100))));

    let keywords_ok = |keywords: &[&str]| {
        store
            .put_cards(&id(1), &[card(1, CardKind::Other, "x", keywords, None, 1)])
            .is_ok()
    };
    let eight: Vec<String> = (0..8).map(|n| format!("关键词{n}")).collect();
    let eight: Vec<&str> = eight.iter().map(String::as_str).collect();
    assert!(keywords_ok(&eight));
    let mut nine = eight.clone();
    nine.push("第九个");
    assert!(!keywords_ok(&nine), "最多 8 个");
    assert!(!keywords_ok(&["海"]), "至少 2 字");
    assert!(keywords_ok(&["八个字的关键词呀"]));
    assert!(!keywords_ok(&["九个字的关键词呀呀"]), "至多 8 字");
    assert!(!keywords_ok(&["  "]));

    assert!(matches!(
        store.add_note(&id(1), &"记".repeat(201), 2),
        Err(MemoryError::Invalid(_))
    ));
    let too_long = MemorySnapshot {
        contacts: vec![contact(1, Scene::Dating)],
        cards: [(
            id(1),
            vec![card(2, CardKind::Other, &"字".repeat(201), &[], None, 1)],
        )]
        .into_iter()
        .collect(),
        ..MemorySnapshot::default()
    };
    assert!(matches!(
        store.write_snapshot(&too_long),
        Err(MemoryError::Invalid(_))
    ));
    std::fs::remove_dir_all(&user).ok();
}
```

Create `cloud/crates/qingjian-cloud-bridge/src/memory/tests/sync.rs`：

```rust
//! App 与键盘两个进程同时写：文件锁、修订号冲突与重读合并、读不了时不覆盖、忘掉的人不复活、「知道了」的记录。
//! 键盘侧只等 200 毫秒的锁：别的进程占着时返回 `LockTimeout`，不卡 2 秒。
//! 「两个进程」用两个线程各持一个 `MemoryStore` 模拟：flock 锁的是打开的文件，同一进程里两个实例也互斥。

use std::collections::{HashMap, HashSet};
use std::fs::OpenOptions;
use std::os::unix::fs::PermissionsExt;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use qingjian_cloud_proto::{CardKind, Scene};

use super::{card, contact, id, temp_dir};
use crate::memory::{LocalDate, MemoryError, MemorySnapshot, MemoryStore};

/// 测试里的 App：在快照上把第一张卡改成 `text`。
fn edit_first_card(snapshot: &mut MemorySnapshot, text: &str) {
    if let Some(first) = snapshot
        .cards
        .get_mut(&id(1))
        .and_then(|cards| cards.first_mut())
    {
        first.text = text.to_owned();
    }
}

#[test]
fn stale_snapshot_conflicts_and_merges() {
    let user = temp_dir("conflict");
    let store = MemoryStore::open(&user);
    store.put_contact(contact(1, Scene::Dating)).unwrap();
    store
        .put_cards(&id(1), &[card(1, CardKind::Other, "原来的", &[], None, 1)])
        .unwrap();
    let mut stale = store.snapshot().unwrap();
    store.add_note(&id(1), "键盘记的", 2).unwrap();
    edit_first_card(&mut stale, "App 改的");
    assert!(matches!(
        store.write_snapshot(&stale),
        Err(MemoryError::Conflict)
    ));
    let mut fresh = store.snapshot().unwrap();
    edit_first_card(&mut fresh, "App 改的");
    store.write_snapshot(&fresh).unwrap();
    let texts: Vec<String> = store
        .cards(&id(1))
        .into_iter()
        .map(|card| card.text)
        .collect();
    assert_eq!(texts, vec!["App 改的", "键盘记的"]);
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn two_processes_lose_no_notes() {
    let user = temp_dir("two-processes");
    MemoryStore::open(&user)
        .put_contact(contact(1, Scene::Dating))
        .unwrap();
    MemoryStore::open(&user)
        .put_cards(&id(1), &[card(1, CardKind::Other, "第一张", &[], None, 1)])
        .unwrap();
    let keyboard_dir = user.clone();
    let keyboard = std::thread::spawn(move || {
        let store = MemoryStore::open(&keyboard_dir);
        for i in 0..40 {
            store.add_note(&id(1), &format!("记一笔 {i}"), i).unwrap();
        }
    });
    let app_dir = user.clone();
    let app = std::thread::spawn(move || {
        let store = MemoryStore::open(&app_dir);
        let mut conflicts = 0;
        for i in 0..40 {
            let mut snapshot = store.snapshot().unwrap();
            for _ in 0..100 {
                edit_first_card(&mut snapshot, &format!("App 改 {i}"));
                match store.write_snapshot(&snapshot) {
                    Ok(()) => break,
                    Err(MemoryError::Conflict) => {
                        conflicts += 1;
                        snapshot = store.snapshot().unwrap();
                    }
                    Err(error) => panic!("{error}"),
                }
            }
        }
        conflicts
    });
    keyboard.join().unwrap();
    let conflicts = app.join().unwrap();
    let texts: HashSet<String> = MemoryStore::open(&user)
        .cards(&id(1))
        .into_iter()
        .map(|card| card.text)
        .collect();
    for i in 0..40 {
        assert!(texts.contains(&format!("记一笔 {i}")), "第 {i} 条丢了");
    }
    assert!(texts.contains("App 改 39"));
    eprintln!("写回冲突 {conflicts} 次，都重读合并成功");
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn unreadable_files_abort_writes() {
    let user = temp_dir("unreadable");
    let store = MemoryStore::open(&user);
    store.put_contact(contact(1, Scene::Dating)).unwrap();
    store
        .put_cards(&id(1), &[card(1, CardKind::Other, "真文件", &[], None, 1)])
        .unwrap();
    let memory = user.join("memory");
    let cards = memory.join(id(1)).join("cards.json");
    let before = std::fs::read_to_string(&cards).unwrap();
    let deny = |path: &std::path::Path, mode: u32| {
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).unwrap();
    };

    deny(&cards, 0o000);
    assert!(matches!(
        store.add_note(&id(1), "新卡", 2),
        Err(MemoryError::Io(_))
    ));
    assert!(store.try_cards(&id(1)).is_err());
    assert!(store.snapshot().is_err(), "整份读不全就不给 App");
    deny(&cards, 0o600);
    assert_eq!(
        std::fs::read_to_string(&cards).unwrap(),
        before,
        "没被空表覆盖"
    );

    let contacts = memory.join("contacts.json");
    deny(&contacts, 0o000);
    assert!(matches!(
        store.put_contact(contact(2, Scene::Dating)),
        Err(MemoryError::Io(_))
    ));
    assert!(matches!(
        store.update_scope(Scene::Dating, Some(&id(1))),
        Err(MemoryError::Io(_))
    ));
    assert!(store.contacts().is_empty(), "只读的接口读不了给空");
    deny(&contacts, 0o600);
    assert_eq!(store.contacts().len(), 1);
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn forgotten_contact_is_not_recreated() {
    let user = temp_dir("forgotten");
    let store = MemoryStore::open(&user);
    store.put_contact(contact(1, Scene::Dating)).unwrap();
    store.add_note(&id(1), "喜欢猫", 1).unwrap();
    store.forget_contact(&id(1)).unwrap();
    let dir = user.join("memory").join(id(1));
    assert!(matches!(
        store.add_note(&id(1), "又记一笔", 2),
        Err(MemoryError::Invalid(_))
    ));
    assert!(matches!(
        store.put_cards(&id(1), &[]),
        Err(MemoryError::Invalid(_))
    ));
    assert!(!dir.exists(), "目录没被重新建出来");
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn dismissed_records_drop_old_and_unknown_cards() {
    let user = temp_dir("dismissed");
    let store = MemoryStore::open(&user);
    let today = LocalDate::parse("2026-10-04").unwrap();
    let mut records = HashMap::new();
    records.insert(id(1001), today);
    records.insert(id(1002), today.add_days(-31));
    records.insert(id(1003), today);
    store.put_dismissed(&records).unwrap();
    let known: HashSet<String> = [id(1001), id(1002)].into_iter().collect();
    let loaded = store.dismissed(today, &known);
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[&id(1001)], today);
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn keyboard_lock_wait_times_out_quickly() {
    let user = temp_dir("lock-timeout");
    let keyboard = MemoryStore::open_with_lock_timeout(&user, Duration::from_millis(200));
    keyboard.put_contact(contact(1, Scene::Dating)).unwrap();

    let lock_path = user.join("memory").join(".lock");
    let (held_tx, held_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel::<()>();
    let holder = std::thread::spawn(move || {
        let file = OpenOptions::new().write(true).open(lock_path).unwrap();
        file.lock().unwrap();
        held_tx.send(()).unwrap();
        release_rx.recv_timeout(Duration::from_secs(10)).ok();
    });
    held_rx.recv_timeout(Duration::from_secs(5)).unwrap();

    let started = Instant::now();
    let result = keyboard.add_note(&id(1), "拿不到锁", 1);
    let waited = started.elapsed();
    assert!(
        matches!(result, Err(MemoryError::LockTimeout)),
        "{result:?}"
    );
    assert!(waited >= Duration::from_millis(150), "{waited:?}");
    assert!(waited < Duration::from_millis(500), "{waited:?}");
    assert_eq!(MemoryError::LockTimeout.code(), "lock_timeout");
    eprintln!("200ms 超时实测 {waited:?}");

    release_tx.send(()).unwrap();
    holder.join().unwrap();
    keyboard.add_note(&id(1), "锁放了就能写", 2).unwrap();
    assert_eq!(keyboard.cards(&id(1)).len(), 1);

    assert_eq!(
        MemoryStore::open(&user).lock_timeout(),
        Duration::from_secs(2),
        "缺省 2 秒"
    );
    std::fs::remove_dir_all(&user).ok();
}
```

- [ ] **Step 4: 挂空模块，跑测试看它失败**

Create `cloud/crates/qingjian-cloud-bridge/src/memory/mod.rs`（临时，Step 5 整个替换）：

```rust
//! 本地记忆。

#[cfg(test)]
mod tests;
```

Modify `src/lib.rs`：`mod` 列表里在 `mod error;` 与 `mod rewrite;` 之间加 `mod memory;`。

Run: `cd /Users/liyuqing/sproot/qingjian-mainline/cloud && cargo test -p qingjian-cloud-bridge memory 2>&1 | tail -5`
Expected: 编译失败，`error[E0432]: unresolved import crate::memory::Card` 等。

- [ ] **Step 5: 写 `memory/mod.rs` 与几个数据类型**

Replace `cloud/crates/qingjian-cloud-bridge/src/memory/mod.rs`：

```rust
//! 本地记忆（spec「2A 本地记忆」）：对象、记忆卡、当前场景的存储，打字时的提示，以及 C 接口。
//! 数据在学习数据目录的 `memory/` 下（iOS 开了完全访问时是 App Group 的 `Qingjian/memory/`）；
//! App 整份读写，键盘只读（「记一笔」「知道了」与当前场景除外）；两个进程的读-改-写都在 `memory/.lock` 的文件锁里。
//! 卡片的种类与来源用 proto 的 `CardKind`、`CardSource`。

mod card;
mod cards_file;
mod contact;
mod dismissed_file;
mod error;
mod local_date;
mod pronoun;
mod snapshot;
mod store;

#[cfg(test)]
mod tests;

use std::time::{SystemTime, UNIX_EPOCH};

use qingjian_cloud_proto::{CardKind, Scene};

use self::cards_file::CardsFile;
use self::dismissed_file::DismissedFile;
use crate::scope::ScopeState;

pub use self::card::Card;
pub use self::contact::Contact;
pub use self::error::MemoryError;
pub use self::local_date::LocalDate;
pub use self::pronoun::Pronoun;
pub use self::snapshot::MemorySnapshot;
pub use self::store::{DEFAULT_LOCK_TIMEOUT, KEYBOARD_LOCK_TIMEOUT, MemoryStore};

/// 学习数据目录下放记忆的子目录。
pub const MEMORY_DIR: &str = "memory";

/// 恋爱场景最多几个对象。
pub const MAX_CONTACTS: usize = 8;

/// 对象与卡片的 id：16 字节随机数的小写十六进制（32 位，不含名字）。
pub fn new_id() -> Result<String, MemoryError> {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes).map_err(|error| MemoryError::Io(std::io::Error::other(error)))?;
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

/// 现在的 Unix 秒；系统时钟早于 1970 时当 0。
pub fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| {
            i64::try_from(elapsed.as_secs()).unwrap_or(i64::MAX)
        })
}

/// 只有日子与约定的 `when` 有意义（提醒、面板排序、校验都按它）。
pub fn has_date(kind: CardKind) -> bool {
    matches!(kind, CardKind::Date | CardKind::Promise)
}

/// 当前对象不在名单上（被删了）、不是恋爱场景的人，或当前不在恋爱场景：退回不指定。
pub(crate) fn sanitized_scope(mut state: ScopeState, contacts: &[Contact]) -> ScopeState {
    let known = state.contact_id.as_deref().is_some_and(|id| {
        contacts
            .iter()
            .any(|c| c.id == id && c.scene == Scene::Dating)
    });
    if state.scene != Scene::Dating || !known {
        state.contact_id = None;
    }
    state
}
```

Create `cloud/crates/qingjian-cloud-bridge/src/memory/pronoun.rs`：

```rust
//! 提醒文案里怎么称呼对象。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Pronoun {
    #[default]
    Ta,

    /// 他。
    TaM,

    /// 她。
    TaF,

    /// 直接用名字。
    Name,
}

impl Pronoun {
    /// 文案里的称呼；`Name` 时用 `name`。
    pub fn label(self, name: &str) -> String {
        match self {
            Self::Ta => "TA".to_owned(),
            Self::TaM => "他".to_owned(),
            Self::TaF => "她".to_owned(),
            Self::Name => name.to_owned(),
        }
    }
}
```

Create `cloud/crates/qingjian-cloud-bridge/src/memory/contact.rs`：

```rust
//! `contacts.json` 的一项：一个对象。名字只在这里，目录名用随机 id。两个提示开关按人设置，旧文件里没有时按开。

use qingjian_cloud_proto::Scene;
use serde::{Deserialize, Serialize};

use super::Pronoun;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Contact {
    pub id: String,

    /// 名字或代号。
    pub name: String,

    #[serde(default)]
    pub pronoun: Pronoun,

    pub scene: Scene,

    /// 建这个对象时的 Unix 秒（「认识 n 天」从这里算）。
    pub created_at: i64,

    /// 打字时按这个人的卡片给提示。
    #[serde(default = "true_default")]
    pub hint_on: bool,

    /// 这个人的日子与约定快到时提醒。
    #[serde(default = "true_default")]
    pub remind_on: bool,
}

fn true_default() -> bool {
    true
}
```

Create `cloud/crates/qingjian-cloud-bridge/src/memory/card.rs`：

```rust
//! `<对象 id>/cards.json` 的一项：一张记忆卡。种类与来源用 proto 的 `CardKind`、`CardSource`，与 2C 云端下发的卡（proto 的 `MemoryCard`）
//! 同一套 JSON 名。手写卡的上限（文字 200 字、关键词 8 个且每个 2–8 字）与 proto 的常量一致，写入前由 `MemoryStore` 校验。

use qingjian_cloud_proto::{CardKind, CardSource};
use serde::{Deserialize, Serialize};

use super::{MemoryError, new_id};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Card {
    pub id: String,

    pub kind: CardKind,

    pub text: String,

    /// 用户写的匹配词，最多 8 个、每个 2–8 字；提示还会从 `text` 里切词。
    #[serde(default)]
    pub keywords: Vec<String>,

    /// `YYYY-MM-DD`（北京时间，与 proto 的 `MemoryCard.when` 同格式），只对日子与约定有意义。
    #[serde(default)]
    pub when: Option<String>,

    /// 手写（2A）或云端整理（2C）。
    pub source: CardSource,

    /// 手写的恒为真；云端整理的（2C）等用户确认。2C 起用户改了云端卡的文字时服务端会隐含置真，本地跟着置真（2C 再做）。
    #[serde(default)]
    pub confirmed: bool,

    /// 已淡出：过期不再提示，「全部」里还看得到（与 proto 一致；2A 手写卡恒为假）。
    #[serde(default)]
    pub faded: bool,

    /// 服务端给的序号，每个用户单独递增，拉取靠它补（与 proto 一致）；2A 手写卡没上过云，为 0。
    #[serde(default)]
    pub seq: i64,

    /// 服务端记的最后修改时间，Unix **毫秒**（与 proto 一致）；2A 手写卡没上过云，为 0。本地改动时间看 `touched_at`。
    #[serde(default)]
    pub updated_at: i64,

    /// 建卡时间，Unix 秒。
    pub created_at: i64,

    /// 本地最后一次改这张卡的时间，Unix 秒；App 写回冲突时两边都改了以它新者为准。
    pub touched_at: i64,
}

impl Card {
    /// 键盘「记一笔」：一张手写的 `other` 卡。
    pub fn note(text: &str, now: i64) -> Result<Self, MemoryError> {
        Ok(Self {
            id: new_id()?,
            kind: CardKind::Other,
            text: text.to_owned(),
            keywords: Vec::new(),
            when: None,
            source: CardSource::Manual,
            confirmed: true,
            faded: false,
            seq: 0,
            updated_at: 0,
            created_at: now,
            touched_at: now,
        })
    }
}
```

Create `cloud/crates/qingjian-cloud-bridge/src/memory/error.rs`：

```rust
//! 记忆读写失败的种类。C 接口把它折成 `{"code","message"}`：code 给 Swift 分支，message 是给用户看的中文。

use thiserror::Error;

use super::MAX_CONTACTS;

#[derive(Debug, Error)]
pub enum MemoryError {
    /// 恋爱场景的对象超过上限。
    #[error("too many contacts in the dating scene")]
    ContactLimit,

    /// 数据不合格；里面是给用户看的原因。
    #[error("invalid memory data")]
    Invalid(&'static str),

    /// App 拿来写回的快照比磁盘上的旧（这期间键盘「记一笔」改过）：重读、合并后再写。
    #[error("memory changed since it was read")]
    Conflict,

    /// 等 `memory/.lock` 超时（另一个进程占着）；键盘等得短，拿不到就进内存待办、下次再试。
    #[error("memory lock timed out")]
    LockTimeout,

    #[error("memory file io: {0}")]
    Io(#[from] std::io::Error),
}

impl MemoryError {
    /// `contact_limit` / `invalid` / `conflict` / `lock_timeout` / `io`，与头文件里写的一致。
    pub fn code(&self) -> &'static str {
        match self {
            Self::ContactLimit => "contact_limit",
            Self::Invalid(_) => "invalid",
            Self::Conflict => "conflict",
            Self::LockTimeout => "lock_timeout",
            Self::Io(_) => "io",
        }
    }

    pub fn message(&self) -> String {
        match self {
            Self::ContactLimit => format!("恋爱场景最多 {MAX_CONTACTS} 个人"),
            Self::Invalid(reason) => (*reason).to_owned(),
            Self::Conflict => "记忆刚在键盘里改过，已重新读取".to_owned(),
            Self::LockTimeout => "记忆正被另一处使用，稍后再试".to_owned(),
            Self::Io(_) => "记忆文件读写不了（开机后还没解锁过时读不到），请解锁后重试".to_owned(),
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::json!({"code": self.code(), "message": self.message()}).to_string()
    }
}
```

Create `cloud/crates/qingjian-cloud-bridge/src/memory/local_date.rs`：

```rust
//! 北京时间（UTC+8，没有夏令时）的日历日，算「还有几天」用。不为这点事引日期库：
//! Unix 秒加 8 小时按天取整；公历换算用 Howard Hinnant 的 days_from_civil / civil_from_days。

use std::fmt;

use super::now_unix;

const SECS_PER_DAY: i64 = 86_400;

const BEIJING_OFFSET_SECS: i64 = 8 * 3600;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LocalDate {
    /// 1970-01-01 起的天数。
    days: i64,
}

impl LocalDate {
    pub fn from_unix(secs: i64) -> Self {
        Self {
            days: (secs + BEIJING_OFFSET_SECS).div_euclid(SECS_PER_DAY),
        }
    }

    pub fn today() -> Self {
        Self::from_unix(now_unix())
    }

    pub fn from_ymd(year: i64, month: u32, day: u32) -> Option<Self> {
        if !(1..=12).contains(&month) || day == 0 || day > days_in_month(year, month) {
            return None;
        }
        Some(Self {
            days: days_from_civil(year, month, day),
        })
    }

    /// 只认 `YYYY-MM-DD`（`2026-1-5`、`2026/01/05` 都不认）。
    pub fn parse(text: &str) -> Option<Self> {
        let bytes = text.as_bytes();
        let shaped = bytes.len() == 10
            && bytes.iter().enumerate().all(|(i, b)| {
                if i == 4 || i == 7 {
                    *b == b'-'
                } else {
                    b.is_ascii_digit()
                }
            });
        if !shaped {
            return None;
        }
        let year = text[0..4].parse().ok()?;
        let month = text[5..7].parse().ok()?;
        let day = text[8..10].parse().ok()?;
        Self::from_ymd(year, month, day)
    }

    /// 从 `self` 到 `other` 还有几天，过去的是负数。
    pub fn days_until(self, other: Self) -> i64 {
        other.days - self.days
    }

    pub fn add_days(self, days: i64) -> Self {
        Self {
            days: self.days + days,
        }
    }

    pub fn ymd(self) -> (i64, u32, u32) {
        civil_from_days(self.days)
    }

    /// 按年重复的日子（生日、纪念日）：`self` 的月日落在 `today` 当天或之后最近的一次；
    /// 2 月 29 日在平年算 2 月 28 日。
    pub fn next_anniversary(self, today: Self) -> Self {
        let (_, month, day) = self.ymd();
        let (year, _, _) = today.ymd();
        let this_year = Self::anniversary_in(year, month, day);
        if this_year >= today {
            this_year
        } else {
            Self::anniversary_in(year + 1, month, day)
        }
    }

    fn anniversary_in(year: i64, month: u32, day: u32) -> Self {
        Self {
            days: days_from_civil(year, month, day.min(days_in_month(year, month))),
        }
    }
}

impl fmt::Display for LocalDate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (year, month, day) = self.ymd();
        write!(f, "{year:04}-{month:02}-{day:02}")
    }
}

fn is_leap(year: i64) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

fn days_in_month(year: i64, month: u32) -> u32 {
    match month {
        2 if is_leap(year) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let yoe = year.rem_euclid(400);
    let month = i64::from(month);
    let day = i64::from(day);
    let doy = (153 * (if month > 2 { month - 3 } else { month + 9 }) + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month as u32, day as u32)
}
```

Create `cloud/crates/qingjian-cloud-bridge/src/memory/snapshot.rs`：

```rust
//! App 整份读写的 JSON：`{"contacts":[…],"cards":{id:[…]},"revs":{id:n},"state":{…},"broken":[id…]}`。
//! `revs` 是读的时候各对象 `cards.json` 的修订号，写回时拿来判断这期间键盘有没有改过。

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::{Card, Contact};
use crate::scope::ScopeState;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct MemorySnapshot {
    pub contacts: Vec<Contact>,

    /// 对象 id → 卡片。
    pub cards: BTreeMap<String, Vec<Card>>,

    /// 对象 id → 读时 `cards.json` 的修订号；没有的按 0。
    pub revs: BTreeMap<String, u64>,

    /// 键盘当前的场景与对象，只给 App 显示；写回时忽略。
    pub state: ScopeState,

    /// 这次读时卡片文件坏了、已改名备份的对象（App 据此提示）；写回时忽略。
    pub broken: Vec<String>,
}
```

Create `cloud/crates/qingjian-cloud-bridge/src/memory/cards_file.rs`：

```rust
//! `<对象 id>/cards.json` 的外层：修订号加卡片。每写一次修订号加一，App 整份写回时靠它发现键盘这期间改过。
//! 早期写的是光秃秃的卡片数组，读时当修订号 0（见 `store.rs` 的 `parse_cards`）。

use serde::{Deserialize, Serialize};

use super::Card;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CardsFile {
    pub rev: u64,

    pub cards: Vec<Card>,
}
```

Create `cloud/crates/qingjian-cloud-bridge/src/memory/dismissed_file.rs`：

```rust
//! `memory/dismissed.json`：键盘「知道了」的记录，只有键盘写，App 不碰。卡片 id → 点的那天（`YYYY-MM-DD`，北京时间）；
//! 当天不再提示，第二天自然失效，读时顺带清掉 30 天前的与已经不在的卡。

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct DismissedFile {
    pub cards: BTreeMap<String, String>,
}
```

- [ ] **Step 6: 写 `memory/store.rs`**

Create `cloud/crates/qingjian-cloud-bridge/src/memory/store.rs`：

```rust
//! `memory/` 下的文件：`contacts.json`、`state.json`、`dismissed.json`、`<对象 id>/cards.json`。
//! App 与键盘是两个进程，都会读-改-写，所以每个操作都在 `memory/.lock` 的文件锁（flock）里完成，读也在锁里；
//! 写走 `cloud_config::write_atomic`（同目录临时文件加改名），而且不建父目录：对象目录只在建对象时创建。
//! 解析不了的文件改名为 `<文件>.broken-<unix 秒>` 再按空处理；读不了的（锁屏时数据保护、权限）不改名，读-改-写直接报错，
//! 不拿空表覆盖真文件。只读的 `contacts()` / `cards()` / `state()` 读不了时给空，只给显示用，不能接着写。

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs::{File, OpenOptions, TryLockError};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

use qingjian_cloud_proto::{MAX_CARD_KEYWORDS, MAX_CARD_TEXT_CHARS, Scene};
use serde::Serialize;
use serde::de::DeserializeOwned;

use super::{
    Card, CardsFile, Contact, DismissedFile, LocalDate, MAX_CONTACTS, MEMORY_DIR, MemoryError,
    MemorySnapshot, now_unix, sanitized_scope,
};
use crate::cloud_config::write_atomic;
use crate::scope::{ScopeState, is_contact_id};

const CONTACTS_FILE: &str = "contacts.json";

const STATE_FILE: &str = "state.json";

const CARDS_FILE: &str = "cards.json";

const DISMISSED_FILE: &str = "dismissed.json";

const LOCK_FILE: &str = ".lock";

/// 等文件锁缺省最多多久（App 用）；另一个进程正常只占几毫秒。
pub const DEFAULT_LOCK_TIMEOUT: Duration = Duration::from_secs(2);

/// 键盘等锁的上限：键盘主线程上不能卡，拿不到就进内存待办、下次刷新再试。
pub const KEYBOARD_LOCK_TIMEOUT: Duration = Duration::from_millis(200);

const LOCK_RETRY: Duration = Duration::from_millis(5);

/// 一个关键词至少、至多几个字（spec 对云端卡关键词的校验，手写卡一并按它）。
const MIN_KEYWORD_CHARS: usize = 2;

const MAX_KEYWORD_CHARS: usize = 8;

/// 「知道了」的记录留多少天。
const DISMISSED_KEEP_DAYS: i64 = 30;

#[derive(Debug, Clone)]
pub struct MemoryStore {
    /// `<学习数据目录>/memory`。
    root: PathBuf,

    /// 等 `.lock` 的上限，超时返回 `MemoryError::LockTimeout`。
    lock_timeout: Duration,
}

impl MemoryStore {
    /// 等锁最多 [`DEFAULT_LOCK_TIMEOUT`]（App 用）。
    pub fn open(user_dir: &Path) -> Self {
        Self::open_with_lock_timeout(user_dir, DEFAULT_LOCK_TIMEOUT)
    }

    /// 自定等锁上限；键盘用 [`KEYBOARD_LOCK_TIMEOUT`]。
    pub fn open_with_lock_timeout(user_dir: &Path, lock_timeout: Duration) -> Self {
        Self {
            root: user_dir.join(MEMORY_DIR),
            lock_timeout,
        }
    }

    pub fn lock_timeout(&self) -> Duration {
        self.lock_timeout
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// 名单；读不了时为空（只给显示用）。
    pub fn contacts(&self) -> Vec<Contact> {
        self.try_contacts().unwrap_or_default()
    }

    pub fn try_contacts(&self) -> Result<Vec<Contact>, MemoryError> {
        let _lock = self.lock()?;
        self.read_contacts()
    }

    /// 一个人的卡片；读不了时为空（只给显示用）。
    pub fn cards(&self, contact_id: &str) -> Vec<Card> {
        self.try_cards(contact_id).unwrap_or_default()
    }

    pub fn try_cards(&self, contact_id: &str) -> Result<Vec<Card>, MemoryError> {
        if !is_contact_id(contact_id) {
            return Err(MemoryError::Invalid("对象编号不对"));
        }
        let _lock = self.lock()?;
        Ok(self.read_cards(contact_id)?.0.cards)
    }

    /// 当前场景与对象；读不了时按缺省（只给显示用）。
    pub fn state(&self) -> ScopeState {
        self.try_state().unwrap_or_default()
    }

    pub fn try_state(&self) -> Result<ScopeState, MemoryError> {
        let _lock = self.lock()?;
        Ok(read_json(&self.state_path())?.0)
    }

    /// 加一个对象或改已有的（同 id）。恋爱场景超过 [`MAX_CONTACTS`] 个返回 [`MemoryError::ContactLimit`]。
    /// 对象目录在这里建，别处写卡片都不建目录。
    pub fn put_contact(&self, contact: Contact) -> Result<(), MemoryError> {
        validate_contacts(std::slice::from_ref(&contact))?;
        let _lock = self.lock()?;
        let mut contacts = self.read_contacts()?;
        let id = contact.id.clone();
        match contacts.iter_mut().find(|c| c.id == contact.id) {
            Some(existing) => *existing = contact,
            None => contacts.push(contact),
        }
        check_limit(&contacts)?;
        std::fs::create_dir_all(self.root.join(&id))?;
        write_json(&self.contacts_path(), &contacts)
    }

    /// 忘掉一个人：先删 `memory/<id>/` 整个目录（卡片与分区学习），删成了再从名单去掉。
    pub fn forget_contact(&self, id: &str) -> Result<(), MemoryError> {
        if !is_contact_id(id) {
            return Err(MemoryError::Invalid("对象编号不对"));
        }
        let _lock = self.lock()?;
        let mut contacts = self.read_contacts()?;
        remove_dir(&self.root.join(id))?;
        contacts.retain(|c| c.id != id);
        write_json(&self.contacts_path(), &contacts)
    }

    /// 整份换掉一个人的卡片（修订号加一）。名单上（磁盘上的）没有这个人就报错。
    pub fn put_cards(&self, contact_id: &str, cards: &[Card]) -> Result<(), MemoryError> {
        if !is_contact_id(contact_id) {
            return Err(MemoryError::Invalid("对象编号不对"));
        }
        validate_cards(cards)?;
        let _lock = self.lock()?;
        self.require_contact(contact_id)?;
        let (file, _) = self.read_cards(contact_id)?;
        self.write_cards(contact_id, file.rev + 1, cards)
    }

    /// 键盘「记一笔」：给磁盘名单上的人加一张手写的 `other` 卡（修订号加一）。读不了就报错、不写。
    pub fn add_note(&self, contact_id: &str, text: &str, now: i64) -> Result<Card, MemoryError> {
        let text = text.trim();
        if text.is_empty() {
            return Err(MemoryError::Invalid("没有要记的文字"));
        }
        if !is_contact_id(contact_id) {
            return Err(MemoryError::Invalid("对象编号不对"));
        }
        let _lock = self.lock()?;
        self.require_contact(contact_id)?;
        let card = Card::note(text, now)?;
        validate_cards(std::slice::from_ref(&card))?;
        let (mut file, _) = self.read_cards(contact_id)?;
        file.cards.push(card.clone());
        self.write_cards(contact_id, file.rev + 1, &file.cards)?;
        Ok(card)
    }

    /// 键盘切场景与对象：在锁里重读 `state.json` 与名单，只改这两个字段；对象不在磁盘名单上或不是恋爱场景就当不指定。
    pub fn update_scope(
        &self,
        scene: Scene,
        contact_id: Option<&str>,
    ) -> Result<ScopeState, MemoryError> {
        let _lock = self.lock()?;
        let contacts = self.read_contacts()?;
        let (mut state, _): (ScopeState, bool) = read_json(&self.state_path())?;
        state.scene = scene;
        state.contact_id = contact_id.map(str::to_owned);
        let state = sanitized_scope(state, &contacts);
        write_json(&self.state_path(), &state)?;
        Ok(state)
    }

    /// App 读的整份数据，带各对象的修订号；卡片文件坏了的对象记进 `broken`。任何一个文件读不了就整份报错。
    pub fn snapshot(&self) -> Result<MemorySnapshot, MemoryError> {
        let _lock = self.lock()?;
        let contacts = self.read_contacts()?;
        let mut snapshot = MemorySnapshot {
            state: read_json(&self.state_path())?.0,
            ..MemorySnapshot::default()
        };
        for contact in contacts.iter().filter(|c| is_contact_id(&c.id)) {
            let (file, quarantined) = self.read_cards(&contact.id)?;
            if quarantined {
                snapshot.broken.push(contact.id.clone());
            }
            snapshot.revs.insert(contact.id.clone(), file.rev);
            snapshot.cards.insert(contact.id.clone(), file.cards);
        }
        snapshot.contacts = contacts;
        Ok(snapshot)
    }

    /// App 整份写回。先校验；锁里逐个比修订号，磁盘上比快照新（键盘这期间改过）就整份不写、返回 [`MemoryError::Conflict`]；
    /// 名单上没了的人连目录一起删；只重写卡片有变化的对象（修订号加一）；`state` 不采纳，当前对象被删了就置空。
    pub fn write_snapshot(&self, snapshot: &MemorySnapshot) -> Result<(), MemoryError> {
        validate_contacts(&snapshot.contacts)?;
        check_limit(&snapshot.contacts)?;
        for (id, cards) in &snapshot.cards {
            if !snapshot.contacts.iter().any(|c| &c.id == id) {
                return Err(MemoryError::Invalid("卡片对不上人"));
            }
            validate_cards(cards)?;
        }
        let _lock = self.lock()?;
        let old = self.read_contacts()?;
        let mut changed = Vec::new();
        for (id, cards) in &snapshot.cards {
            let (disk, _) = self.read_cards(id)?;
            if disk.rev > snapshot.revs.get(id).copied().unwrap_or(0) {
                return Err(MemoryError::Conflict);
            }
            if disk.cards != *cards {
                changed.push((id, disk.rev + 1, cards));
            }
        }
        for gone in old
            .iter()
            .filter(|o| is_contact_id(&o.id) && !snapshot.contacts.iter().any(|c| c.id == o.id))
        {
            remove_dir(&self.root.join(&gone.id))?;
        }
        for contact in &snapshot.contacts {
            std::fs::create_dir_all(self.root.join(&contact.id))?;
        }
        for (id, rev, cards) in changed {
            self.write_cards(id, rev, cards)?;
        }
        write_json(&self.contacts_path(), &snapshot.contacts)?;
        let (state, _): (ScopeState, bool) = read_json(&self.state_path())?;
        let fixed = sanitized_scope(state.clone(), &snapshot.contacts);
        if fixed != state {
            write_json(&self.state_path(), &fixed)?;
        }
        Ok(())
    }

    /// 「知道了」的记录，丢掉 30 天前的与不在 `known` 里的卡；读不了时为空。
    pub fn dismissed(
        &self,
        today: LocalDate,
        known: &HashSet<String>,
    ) -> HashMap<String, LocalDate> {
        let read = || -> Result<DismissedFile, MemoryError> {
            let _lock = self.lock()?;
            Ok(read_json(&self.dismissed_path())?.0)
        };
        let file = read().unwrap_or_else(|error| {
            tracing::warn!(%error, "「知道了」的记录读不了");
            DismissedFile::default()
        });
        file.cards
            .into_iter()
            .filter(|(id, _)| known.contains(id))
            .filter_map(|(id, day)| Some((id, LocalDate::parse(&day)?)))
            .filter(|(_, day)| day.days_until(today) <= DISMISSED_KEEP_DAYS)
            .collect()
    }

    pub fn put_dismissed(&self, dismissed: &HashMap<String, LocalDate>) -> Result<(), MemoryError> {
        let file = DismissedFile {
            cards: dismissed
                .iter()
                .map(|(id, day)| (id.clone(), day.to_string()))
                .collect::<BTreeMap<_, _>>(),
        };
        let _lock = self.lock()?;
        write_json(&self.dismissed_path(), &file)
    }

    /// `contacts.json`、`state.json` 与当前对象 `cards.json` 的修改时间；键盘轮询时比对，变了才重读。
    pub fn stamp(&self, contact_id: Option<&str>) -> [Option<SystemTime>; 3] {
        let cards = contact_id
            .filter(|id| is_contact_id(id))
            .map(|id| self.cards_path(id));
        [
            modified(&self.contacts_path()),
            modified(&self.state_path()),
            cards.as_deref().and_then(modified),
        ]
    }

    /// 拿 `memory/.lock` 的文件锁：`try_lock` 加重试，最多等 `lock_timeout`。返回的文件关掉时锁就放了。
    /// flock 锁的是打开的文件，同一进程里两个 `MemoryStore` 也互斥。
    fn lock(&self) -> Result<File, MemoryError> {
        std::fs::create_dir_all(&self.root)?;
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(self.root.join(LOCK_FILE))?;
        let started = Instant::now();
        loop {
            match file.try_lock() {
                Ok(()) => return Ok(file),
                Err(TryLockError::WouldBlock) if started.elapsed() < self.lock_timeout => {
                    std::thread::sleep(LOCK_RETRY);
                }
                Err(TryLockError::WouldBlock) => {
                    return Err(MemoryError::LockTimeout);
                }
                Err(TryLockError::Error(error)) => return Err(error.into()),
            }
        }
    }

    fn require_contact(&self, contact_id: &str) -> Result<(), MemoryError> {
        if self.read_contacts()?.iter().any(|c| c.id == contact_id) {
            Ok(())
        } else {
            Err(MemoryError::Invalid("名单上没有这个人"))
        }
    }

    fn read_contacts(&self) -> Result<Vec<Contact>, MemoryError> {
        read_json(&self.contacts_path()).map(|(contacts, _)| contacts)
    }

    fn read_cards(&self, contact_id: &str) -> Result<(CardsFile, bool), MemoryError> {
        read_with(&self.cards_path(contact_id), parse_cards)
    }

    fn write_cards(&self, contact_id: &str, rev: u64, cards: &[Card]) -> Result<(), MemoryError> {
        let file = CardsFile {
            rev,
            cards: cards.to_vec(),
        };
        write_json(&self.cards_path(contact_id), &file)
    }

    fn contacts_path(&self) -> PathBuf {
        self.root.join(CONTACTS_FILE)
    }

    fn state_path(&self) -> PathBuf {
        self.root.join(STATE_FILE)
    }

    fn dismissed_path(&self) -> PathBuf {
        self.root.join(DISMISSED_FILE)
    }

    fn cards_path(&self, contact_id: &str) -> PathBuf {
        self.root.join(contact_id).join(CARDS_FILE)
    }
}

/// `cards.json`：现在是 `{"rev","cards"}`，早期是卡片数组（当修订号 0）。
fn parse_cards(text: &str) -> Result<CardsFile, serde_json::Error> {
    serde_json::from_str::<CardsFile>(text).or_else(|error| {
        serde_json::from_str::<Vec<Card>>(text)
            .map(|cards| CardsFile { rev: 0, cards })
            .map_err(|_| error)
    })
}

fn read_json<T: DeserializeOwned + Default>(path: &Path) -> Result<(T, bool), MemoryError> {
    read_with(path, |text| serde_json::from_str(text))
}

/// 读一个 JSON 文件：不在按缺省；解析不了改名备份后按缺省，第二个值为真；其余 io 错误原样返回。
fn read_with<T: Default>(
    path: &Path,
    parse: impl Fn(&str) -> Result<T, serde_json::Error>,
) -> Result<(T, bool), MemoryError> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok((T::default(), false)),
        Err(error) => return Err(error.into()),
    };
    match parse(&text) {
        Ok(value) => Ok((value, false)),
        Err(error) => {
            tracing::warn!(path = %path.display(), %error, "记忆文件坏了，改名备份后按空处理");
            quarantine(path);
            Ok((T::default(), true))
        }
    }
}

fn write_json<T: Serialize + ?Sized>(path: &Path, value: &T) -> Result<(), MemoryError> {
    let bytes =
        serde_json::to_vec_pretty(value).map_err(|_| MemoryError::Invalid("数据编码失败"))?;
    write_atomic(path, &bytes, false)?;
    Ok(())
}

fn quarantine(path: &Path) {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(format!(".broken-{}", now_unix()));
    if let Err(error) = std::fs::rename(path, path.with_file_name(name)) {
        tracing::warn!(%error, "坏文件没改成备份名");
    }
}

fn remove_dir(dir: &Path) -> Result<(), MemoryError> {
    match std::fs::remove_dir_all(dir) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

fn modified(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}

fn check_limit(contacts: &[Contact]) -> Result<(), MemoryError> {
    let dating = contacts.iter().filter(|c| c.scene == Scene::Dating).count();
    if dating > MAX_CONTACTS {
        return Err(MemoryError::ContactLimit);
    }
    Ok(())
}

fn validate_contacts(contacts: &[Contact]) -> Result<(), MemoryError> {
    let mut seen = HashSet::new();
    for contact in contacts {
        if !is_contact_id(&contact.id) {
            return Err(MemoryError::Invalid("对象编号不对"));
        }
        if contact.name.trim().is_empty() {
            return Err(MemoryError::Invalid("名字不能是空的"));
        }
        if !seen.insert(contact.id.as_str()) {
            return Err(MemoryError::Invalid("同一个人出现了两次"));
        }
    }
    Ok(())
}

fn validate_cards(cards: &[Card]) -> Result<(), MemoryError> {
    let mut seen = HashSet::new();
    for card in cards {
        if !is_contact_id(&card.id) {
            return Err(MemoryError::Invalid("卡片编号不对"));
        }
        if card.text.trim().is_empty() {
            return Err(MemoryError::Invalid("卡片内容不能是空的"));
        }
        if card.text.chars().count() > MAX_CARD_TEXT_CHARS {
            return Err(MemoryError::Invalid("一张卡最多 200 个字"));
        }
        if card.keywords.len() > MAX_CARD_KEYWORDS {
            return Err(MemoryError::Invalid("关键词最多 8 个"));
        }
        if card.keywords.iter().any(|keyword| {
            !(MIN_KEYWORD_CHARS..=MAX_KEYWORD_CHARS).contains(&keyword.trim().chars().count())
        }) {
            return Err(MemoryError::Invalid("每个关键词要 2 到 8 个字"));
        }
        if card
            .when
            .as_deref()
            .is_some_and(|when| LocalDate::parse(when).is_none())
        {
            return Err(MemoryError::Invalid("日期要写成 2026-10-04 这样"));
        }
        if !seen.insert(card.id.as_str()) {
            return Err(MemoryError::Invalid("同一张卡片出现了两次"));
        }
    }
    Ok(())
}
```

（卡片 id 与对象 id 同一格式，所以用 `is_contact_id` 校验。）

- [ ] **Step 7: lib.rs 导出**

Modify `src/lib.rs`：在 `pub use self::error::BridgeError;` 之后加：

```rust
pub use self::memory::{
    Card, Contact, DEFAULT_LOCK_TIMEOUT, KEYBOARD_LOCK_TIMEOUT, LocalDate, MAX_CONTACTS, MEMORY_DIR,
    MemoryError, MemorySnapshot, MemoryStore, Pronoun, has_date, new_id, now_unix,
};
```

- [ ] **Step 8: 跑测试看它通过**

Run: `cd /Users/liyuqing/sproot/qingjian-mainline/cloud && cargo test -p qingjian-cloud-bridge memory 2>&1 | tail -15`
Expected: `memory::tests::date::` 5 个、`memory::tests::store::` 11 个、`memory::tests::sync::` 6 个全过，`test result: ok. 22 passed`。`two_processes_lose_no_notes` 会在 stderr 打一行「写回冲突 N 次，都重读合并成功」（N 随调度变，0 也算过；要看它用 `-- --nocapture`）。

- [ ] **Step 9: 全量测试、格式与 clippy**

Run: `cd /Users/liyuqing/sproot/qingjian-mainline/cloud && cargo fmt --all && cargo test -p qingjian-cloud-bridge 2>&1 | grep "test result" && cargo clippy -p qingjian-cloud-bridge --all-targets -- -D warnings 2>&1 | tail -3`
Expected: 全部 `ok`；clippy 末行 `Finished`。

- [ ] **Step 10: 提交**

```bash
cd /Users/liyuqing/sproot/qingjian-mainline
git add cloud/Cargo.lock cloud/crates/qingjian-cloud-bridge/Cargo.toml \
  cloud/crates/qingjian-cloud-bridge/src/cloud_config.rs \
  cloud/crates/qingjian-cloud-bridge/src/memory cloud/crates/qingjian-cloud-bridge/src/lib.rs
git commit -m "feat(cloud): 桥加本地记忆存储 MemoryStore

memory/ 下 contacts.json、state.json、dismissed.json、<id>/cards.json。App 与键盘两个进程都会读-改-写：
每个操作都在 memory/.lock 的 flock（File::try_lock 加重试；App 2 秒超时、键盘 200 毫秒，键盘拿不到锁就把写入放进内存待办，Task 4）里做，读也在锁里；cards.json 带修订号，
App 拿旧快照写回时整份不写、返回 conflict，由 App 重读合并；只重写有变化的对象。
读不了（锁屏）的文件不改名、读-改-写直接报错，不拿空表覆盖；写不建父目录，对象目录只在建对象时创建，忘掉的人不会复活。
「记一笔」与切场景都在锁里按磁盘上的名单与 state 读-改-写；提示开关按人放在 Contact 上；日子按年重复用 next_anniversary。
手写卡文字最多 200 字、关键词最多 8 个且每个 2–8 字（proto 的常量），写入前校验；Card 带上与 proto 一致的 faded / seq / updated_at。
北京时间日期自己算（LocalDate）；id 用 getrandom 的 16 字节。

```

## Task 3：提示 `HintIndex`

**Files:** Create `memory/hint.rs`；测试进 `memory/tests.rs`。

接口：

```rust
pub struct Hint { pub card_id: String, pub text: String, pub reason: HintReason /* Match | Today */, pub more: bool }
pub struct HintIndex { /* 当前对象的卡片与匹配词、节流表 */ }
impl HintIndex {
    pub fn build(cards: &[Card], segment: impl Fn(&str) -> Vec<String>) -> Self;  // segment 由 Session 用引擎词库提供
    pub fn match_text(&mut self, recent: &str, now: i64) -> Option<Hint>;        // recent = 最近上屏 24 字 + 当前首选
    pub fn today(&mut self, now_local_date: NaiveDate, pronoun: Pronoun) -> Option<Hint>;
    pub fn dismiss(&mut self, card_id: &str, today: bool, now: i64);
}
```

- 匹配词 = `keywords` ∪ 卡片文字切分出的两字以上的词 − 停用词表（`memory/stopwords.txt`，随代码提交，先放 100 个常用虚词与泛词）。
- 至少命中一个两字以上的词；按命中词数降序、`touched_at` 降序；同卡 10 分钟内不重复；`dismiss(today=true)` 当天不再出。
- `today`：`date` / `promise` 且 `when` 在 0–3 天内；文案模板「{n} 天后是{称呼}的{text}」/「明天…」/「今天…」，称呼按 `Pronoun` 替换（`ta`→「TA」、`ta_m`→「他」、`ta_f`→「她」、`name`→名字）。
- 日期用北京时间（UTC+8）。

测试：命中与不命中；停用词不触发；节流 10 分钟；dismiss 当天；today 0 / 1 / 3 / 4 天边界；称呼替换四种。

### 与大纲的差异

1. **`today` 的签名改成 `today(&self, today: LocalDate, pronoun: Pronoun, name: &str) -> Option<Hint>`**：`NaiveDate` 换成 Task 2 的 `LocalDate`（不引日期库）；`Pronoun::Name` 要用名字，得把名字传进来；它不改索引状态，用 `&self`。
2. **「用引擎词库切分」落实为用语言模型切：** 上游 `Dictionary` 只能按拼音查、没有按文字查词的接口；`Engine::language_model()` 公开，`qingjian_core::sentence::segment_text(text, model)` 就是引擎自己切上屏文字用的（输入统计按它算词数）。没有 `lm.qj` 时它返回 `None`，这时匹配词只剩 `keywords`（会话侧在 Task 4 接，`HintIndex` 只收一个闭包，不受影响）。
3. **拆文件：** `memory/hint/{mod,entry,index,item,reason}.rs`（`Hint`、`HintReason`、`HintIndex`、索引里的一张卡 `IndexedCard` 各一个文件）；停用词表 `memory/stopwords.txt`；测试放 `memory/tests/hint.rs`。
4. **多两样东西：** `RecentText`（`memory/recent.rs`，最近 24 字的环形缓冲，Task 4 用）与自由函数 `panel_cards`（`qj_memory_cards` 挑「今日相关最多 3 张」的规则，放这里一起测）、`reminder_text`（提醒文案，Swift 侧 App 首页用同一模板）。
5. **节流的具体语义**（大纲只说「同卡 10 分钟内不重复」）：提示行一直命中同一张卡时接着显示（不然打一个字就消失）；每次给出都记时间，**消失之后** 10 分钟内同一张卡不再出。`dismiss(today=false)` 等于「现在起 10 分钟内别出」。
6. **`more` 的语义（决定点 8）：** 除了正在显示的这张，当前对象还有别的卡（`has_other(card_id)`）。索引只收确认过的卡（`confirmed`，手写的都是），没确认的卡（2C 云端下发、待确认）既不提示也不算「别的卡」。
7. **审计修订（决定点 2、4，重要 5，建议「按种类分模板」）：**
   - **日子按年重复：** 自由函数 `days_away(kind, when, today)`：`date` 用 `LocalDate::next_anniversary`（今年那天过了看明年，2 月 29 日平年按 2 月 28 日），`promise` 按写的那天、只提醒一次；`today` 与 `panel_cards` 都走它。
   - **提醒文案按种类分模板：** `reminder_text(kind, days, pronoun, name, text)`：日子「明天是她的生日」，约定「明天：看电影」（「今天：…」「3 天后：…」）。
   - **重建时带上节流与「知道了」：** `HintIndex::rebuild(&self, cards, segment)` 按卡片 id 带过去 `shown` 与正显示的状态，已经不在的卡丢掉；`dismissed` 整份带过去（里面也有别的对象的卡，过期与已删的卡由 `MemoryStore::dismissed` 加载时按 30 天与「是否还存在」清）。`dismissed()` / `set_dismissed()` 给会话落盘与加载 `dismissed.json` 用。

### 步骤

**Files:**
- Create: `cloud/crates/qingjian-cloud-bridge/src/memory/hint/{mod,entry,index,item,reason}.rs`、`src/memory/recent.rs`、`src/memory/stopwords.txt`
- Create: `cloud/crates/qingjian-cloud-bridge/src/memory/tests/hint.rs`
- Modify: `src/memory/mod.rs`、`src/memory/tests/mod.rs`、`src/lib.rs`

- [ ] **Step 1: 写失败的测试**

Create `cloud/crates/qingjian-cloud-bridge/src/memory/tests/hint.rs`：

```rust
//! 提示：两字以上的词才算、停用词不触发、排序、10 分钟节流、「知道了」当天不出、重建时带上节流与「知道了」、
//! 0–3 天的提醒（日子按年重复、约定只一次）与称呼、`more`、面板挑卡、最近 24 字。

use qingjian_cloud_proto::CardKind;

use super::{card, id};
use crate::memory::{
    HintIndex, HintReason, LocalDate, Pronoun, RECENT_CHARS, RecentText, panel_cards, reminder_text,
};

/// 2026-10-04 10:00 北京时间。
const NOW: i64 = 1_791_079_200;

/// 测试用的切词：卡片文字里用 `/` 标好词界。
fn split(text: &str) -> Vec<String> {
    text.split('/').map(str::to_owned).collect()
}

#[test]
fn matches_terms_of_two_or_more_chars() {
    let cards = [card(1, CardKind::Other, "想去/海边/看/日出", &[], None, 1)];
    let mut index = HintIndex::build(&cards, split);
    let hint = index.match_text("周末我们去海边吧", NOW).unwrap();
    assert_eq!(hint.card_id, id(1001));
    assert_eq!(hint.reason, HintReason::Match);
    assert_eq!(hint.text, "想去/海边/看/日出");
    assert!(!hint.more, "只有一张卡");
    assert!(index.match_text("今天天气不错", NOW + 1).is_none());
    assert!(index.match_text("看", NOW + 2000).is_none(), "单字不算词");
}

#[test]
fn keywords_count_and_stopwords_never_trigger() {
    let cards = [card(
        1,
        CardKind::Other,
        "我们/一起/散步",
        &["今天", " 公园 "],
        None,
        1,
    )];
    let mut index = HintIndex::build(&cards, split);
    assert!(index.match_text("我们今天一起吃饭", NOW).is_none());
    assert!(
        index.match_text("去公园吗", NOW + 1).is_some(),
        "关键词去掉首尾空白"
    );
    assert!(index.match_text("饭后去散步", NOW + 2).is_some());
}

#[test]
fn more_hits_then_newer_cards_rank_first() {
    let cards = [
        card(1, CardKind::Other, "海边", &[], None, 5),
        card(2, CardKind::Other, "海边/日出", &[], None, 1),
        card(3, CardKind::Other, "海边", &[], None, 9),
    ];
    let mut index = HintIndex::build(&cards, split);
    let hint = index.match_text("海边看日出", NOW).unwrap();
    assert_eq!(hint.card_id, id(1002));
    assert!(hint.more);
    let mut index = HintIndex::build(&cards, split);
    assert_eq!(
        index.match_text("去海边", NOW).unwrap().card_id,
        id(1003),
        "命中数一样时新改过的在前"
    );
}

#[test]
fn same_card_waits_ten_minutes_after_it_goes_away() {
    let cards = [card(1, CardKind::Other, "海边", &[], None, 1)];
    let mut index = HintIndex::build(&cards, split);
    assert!(index.match_text("海边", NOW).is_some());
    assert!(
        index.match_text("海边呀", NOW + 5).is_some(),
        "一直命中时接着显示"
    );
    assert!(index.match_text("吃饭", NOW + 10).is_none());
    assert!(
        index.match_text("海边", NOW + 60).is_none(),
        "消失后 10 分钟内不再出"
    );
    assert!(index.match_text("海边", NOW + 5 + 600).is_some());

    index.dismiss(&id(1001), false, NOW + 700);
    assert!(
        index.match_text("海边", NOW + 760).is_none(),
        "关掉也算出过"
    );
    assert!(index.match_text("海边", NOW + 1300).is_some());
}

#[test]
fn dismissed_today_stays_quiet_until_tomorrow() {
    let cards = [card(1, CardKind::Other, "海边", &[], None, 1)];
    let mut index = HintIndex::build(&cards, split);
    assert!(index.match_text("海边", NOW).is_some());
    index.dismiss(&id(1001), true, NOW);
    assert!(index.match_text("海边", NOW + 700).is_none());
    assert!(
        index.match_text("海边", NOW + 86_400).is_some(),
        "第二天照常"
    );
}

#[test]
fn reminders_cover_today_to_three_days() {
    let today = LocalDate::parse("2026-10-04").unwrap();
    let at = |when: &str| {
        let cards = [card(1, CardKind::Date, "生日", &[], Some(when), 1)];
        HintIndex::build(&cards, split)
            .today(today, Pronoun::Ta, "小美")
            .map(|hint| hint.text)
    };
    assert_eq!(at("2026-10-04").as_deref(), Some("今天是TA的生日"));
    assert_eq!(at("2026-10-05").as_deref(), Some("明天是TA的生日"));
    assert_eq!(at("2026-10-07").as_deref(), Some("3 天后是TA的生日"));
    assert_eq!(at("2026-10-08"), None);
    assert_eq!(at("2026-10-03"), None, "今年的过了，明年的还远");
    assert_eq!(
        at("1998-10-05").as_deref(),
        Some("明天是TA的生日"),
        "日子按年重复，写出生那年也行"
    );

    let cards = [
        card(1, CardKind::Preference, "生日", &[], Some("2026-10-04"), 1),
        card(2, CardKind::Promise, "看电影", &[], Some("2026-10-06"), 1),
        card(3, CardKind::Date, "纪念日", &[], Some("2026-10-05"), 1),
    ];
    let hint = HintIndex::build(&cards, split)
        .today(today, Pronoun::TaF, "小美")
        .unwrap();
    assert_eq!(hint.reason, HintReason::Today);
    assert_eq!(hint.card_id, id(1003), "只看日子与约定，近的先出");
    assert_eq!(hint.text, "明天是她的纪念日");
}

#[test]
fn dismissed_reminder_stays_quiet_today() {
    let today = LocalDate::from_unix(NOW);
    let cards = [card(1, CardKind::Date, "生日", &[], Some("2026-10-05"), 1)];
    let mut index = HintIndex::build(&cards, split);
    assert!(index.today(today, Pronoun::Ta, "小美").is_some());
    index.dismiss(&id(1001), true, NOW);
    assert!(index.today(today, Pronoun::Ta, "小美").is_none());
    assert!(
        index
            .today(today.add_days(1), Pronoun::Ta, "小美")
            .is_some()
    );
}

#[test]
fn pronouns_fill_the_template() {
    assert_eq!(
        reminder_text(CardKind::Date, 1, Pronoun::Ta, "小美", "生日"),
        "明天是TA的生日"
    );
    assert_eq!(
        reminder_text(CardKind::Date, 1, Pronoun::TaM, "小美", "生日"),
        "明天是他的生日"
    );
    assert_eq!(
        reminder_text(CardKind::Date, 1, Pronoun::TaF, "小美", "生日"),
        "明天是她的生日"
    );
    assert_eq!(
        reminder_text(CardKind::Date, 1, Pronoun::Name, "小美", "生日"),
        "明天是小美的生日"
    );
    assert_eq!(
        reminder_text(CardKind::Date, 0, Pronoun::Ta, "小美", "约会"),
        "今天是TA的约会"
    );
    assert_eq!(
        reminder_text(CardKind::Date, 2, Pronoun::Ta, "小美", "约会"),
        "2 天后是TA的约会"
    );
}

#[test]
fn promises_remind_once_with_their_own_template() {
    let today = LocalDate::parse("2026-10-04").unwrap();
    let at = |when: &str| {
        let cards = [card(1, CardKind::Promise, "看电影", &[], Some(when), 1)];
        HintIndex::build(&cards, split)
            .today(today, Pronoun::TaF, "小美")
            .map(|hint| hint.text)
    };
    assert_eq!(at("2026-10-05").as_deref(), Some("明天：看电影"));
    assert_eq!(at("2026-10-04").as_deref(), Some("今天：看电影"));
    assert_eq!(at("2026-10-07").as_deref(), Some("3 天后：看电影"));
    assert_eq!(at("2025-10-05"), None, "约定不按年重复");
}

#[test]
fn yearly_dates_cross_years_and_leap_days() {
    let remind = |when: &str, today: &str| {
        let cards = [card(1, CardKind::Date, "生日", &[], Some(when), 1)];
        HintIndex::build(&cards, split)
            .today(LocalDate::parse(today).unwrap(), Pronoun::Ta, "小美")
            .map(|hint| hint.text)
    };
    assert_eq!(
        remind("2020-01-02", "2026-12-30").as_deref(),
        Some("3 天后是TA的生日")
    );
    assert_eq!(
        remind("2024-02-29", "2026-02-27").as_deref(),
        Some("明天是TA的生日"),
        "平年按 2 月 28 日"
    );
    assert_eq!(
        remind("2024-02-29", "2028-02-28").as_deref(),
        Some("明天是TA的生日"),
        "闰年是 2 月 29 日"
    );
}

#[test]
fn more_means_another_card_besides_this_one() {
    let one = [card(1, CardKind::Other, "海边", &[], None, 1)];
    assert!(
        !HintIndex::build(&one, split)
            .match_text("海边", NOW)
            .unwrap()
            .more
    );
    let two = [
        card(1, CardKind::Other, "海边", &[], None, 1),
        card(2, CardKind::Other, "猫", &[], None, 1),
    ];
    assert!(
        HintIndex::build(&two, split)
            .match_text("海边", NOW)
            .unwrap()
            .more
    );
    let mut unconfirmed = card(2, CardKind::Other, "猫咪", &[], None, 1);
    unconfirmed.confirmed = false;
    let mut index = HintIndex::build(&[one[0].clone(), unconfirmed], split);
    assert!(index.match_text("猫咪", NOW).is_none(), "没确认的卡不提示");
    assert!(!index.match_text("海边", NOW).unwrap().more, "也不算别的卡");
}

#[test]
fn rebuild_keeps_throttle_and_dismissals() {
    let cards = [
        card(1, CardKind::Other, "海边", &[], None, 1),
        card(2, CardKind::Other, "日出", &[], None, 1),
    ];
    let mut index = HintIndex::build(&cards, split);
    assert!(index.match_text("海边", NOW).is_some());
    assert!(index.match_text("吃饭", NOW + 1).is_none());
    index.dismiss(&id(1002), true, NOW);
    let more = [
        cards[0].clone(),
        cards[1].clone(),
        card(3, CardKind::Other, "猫", &[], None, 1),
    ];
    let mut rebuilt = index.rebuild(&more, split);
    assert!(
        rebuilt.match_text("海边", NOW + 60).is_none(),
        "重建后节流还在"
    );
    assert!(
        rebuilt.match_text("日出", NOW + 60).is_none(),
        "重建后「知道了」还在"
    );
    assert_eq!(rebuilt.dismissed().len(), 1);
    let gone = rebuilt.rebuild(&more[2..], split);
    let mut fresh = gone.rebuild(&more, split);
    assert!(
        fresh.match_text("海边", NOW + 60).is_some(),
        "卡不在了，节流记录跟着丢"
    );
}

#[test]
fn panel_puts_upcoming_dates_then_the_hinted_card_first() {
    let today = LocalDate::parse("2026-10-04").unwrap();
    let cards = vec![
        card(1, CardKind::Other, "猫叫团子", &[], None, 9),
        card(2, CardKind::Promise, "看电影", &[], Some("2026-10-06"), 1),
        card(3, CardKind::Date, "生日", &[], Some("2026-10-05"), 1),
        card(4, CardKind::Recent, "在准备考试", &[], None, 5),
        card(5, CardKind::Date, "纪念日", &[], Some("2026-12-01"), 20),
    ];
    let focus = id(1004);
    let picked: Vec<String> = panel_cards(cards, today, Some(focus.as_str()))
        .into_iter()
        .map(|card| card.id)
        .collect();
    assert_eq!(picked, vec![id(1003), id(1002), id(1004)]);
}

#[test]
fn recent_text_keeps_the_last_chars() {
    let mut recent = RecentText::default();
    recent.push_str(&"一".repeat(30));
    recent.push_str("海边");
    assert_eq!(recent.text().chars().count(), RECENT_CHARS);
    assert!(recent.text().ends_with("海边"));
    recent.clear();
    assert!(recent.text().is_empty());
}
```

Modify `cloud/crates/qingjian-cloud-bridge/src/memory/tests/mod.rs`：`mod date;` 之后加一行 `mod hint;`。

- [ ] **Step 2: 跑测试看它失败**

Run: `cd /Users/liyuqing/sproot/qingjian-mainline/cloud && cargo test -p qingjian-cloud-bridge memory::tests::hint 2>&1 | tail -5`
Expected: 编译失败，`error[E0432]: unresolved imports crate::memory::HintIndex …`。

- [ ] **Step 3: 停用词表**

Create `cloud/crates/qingjian-cloud-bridge/src/memory/stopwords.txt`（100 个，空格分隔、一行十个；`#` 开头是注释）：

```
# 提示的停用词：卡片文字切出来的这些词不当匹配词（太常见，打字时处处会碰上）。空格或换行分隔。
我们 你们 他们 她们 它们 咱们 自己 大家 别人 人家
什么 怎么 怎样 为什么 哪里 哪儿 哪个 多少 几个 如何
这个 那个 这些 那些 这样 那样 这里 那里 这么 那么
时候 现在 今天 明天 昨天 后天 刚才 最近 以后 以前
之前 之后 已经 还是 还有 就是 但是 因为 所以 如果
虽然 然后 而且 或者 不过 只是 不是 没有 可以 可能
应该 需要 知道 觉得 感觉 一个 一些 一点 一下 一起
一直 一定 一样 真的 其实 确实 比较 非常 特别 有点
好像 东西 事情 地方 问题 时间 开始 继续 希望 喜欢
今年 去年 明年 晚上 早上 中午 下午 上午 周末 哈哈
```

Run: `cd /Users/liyuqing/sproot/qingjian-mainline/cloud/crates/qingjian-cloud-bridge && grep -v '^#' src/memory/stopwords.txt | tr ' ' '\n' | grep -c . && grep -v '^#' src/memory/stopwords.txt | tr ' ' '\n' | grep . | sort | uniq -d`
Expected: `100`，第二条命令没有输出（没有重复）。

- [ ] **Step 4: 写 `RecentText`**

Create `cloud/crates/qingjian-cloud-bridge/src/memory/recent.rs`：

```rust
//! 最近上屏的 [`RECENT_CHARS`] 个字：提示拿它加当前首选去匹配卡片。只在内存里，换对象时清空。

use std::collections::VecDeque;

use super::RECENT_CHARS;

#[derive(Debug, Clone, Default)]
pub struct RecentText {
    chars: VecDeque<char>,
}

impl RecentText {
    pub fn push_str(&mut self, text: &str) {
        for c in text.chars() {
            if self.chars.len() == RECENT_CHARS {
                self.chars.pop_front();
            }
            self.chars.push_back(c);
        }
    }

    pub fn text(&self) -> String {
        self.chars.iter().collect()
    }

    pub fn clear(&mut self) {
        self.chars.clear();
    }
}
```

- [ ] **Step 5: 写 `memory/hint/` 五个文件**

Create `cloud/crates/qingjian-cloud-bridge/src/memory/hint/reason.rs`：

```rust
//! 提示是怎么来的：打字碰上了卡片里的词，或日子与约定快到了。

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum HintReason {
    Match,

    Today,
}
```

Create `cloud/crates/qingjian-cloud-bridge/src/memory/hint/item.rs`：

```rust
//! 一条提示，C 接口原样转成 `{"card_id","text","reason","more"}`。

use serde::Serialize;

use super::HintReason;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Hint {
    pub card_id: String,

    /// 提示行上显示的字：匹配时是卡片文字，日子提醒时是套好模板的一句。
    pub text: String,

    pub reason: HintReason,

    /// 除了这张，当前对象还有别的卡（提示行右侧「展开」看得到）。
    pub more: bool,
}
```

Create `cloud/crates/qingjian-cloud-bridge/src/memory/hint/entry.rs`：

```rust
//! 建好索引的一张卡：只留匹配与提醒用得上的字段。

use qingjian_cloud_proto::CardKind;

use crate::memory::LocalDate;

#[derive(Debug)]
pub struct IndexedCard {
    pub id: String,

    pub text: String,

    pub kind: CardKind,

    /// 解析得了的 `when`。
    pub when: Option<LocalDate>,

    pub touched_at: i64,

    /// 匹配词：两字以上、不在停用词表里、去重。
    pub terms: Vec<String>,
}
```

Create `cloud/crates/qingjian-cloud-bridge/src/memory/hint/index.rs`：

```rust
//! 当前对象的提示索引：卡片与匹配词、每张卡上次给出的时间、「知道了」的日子，以及正显示着的那张。
//! 卡片变了用 [`HintIndex::rebuild`] 重建，节流与「知道了」带过去；「知道了」由会话落盘到 `dismissed.json`。

use std::collections::HashMap;

use super::{
    Hint, HintReason, IndexedCard, REMINDER_DAYS, THROTTLE_SECS, days_away, is_term, reminder_text,
};
use crate::memory::{Card, LocalDate, Pronoun};

#[derive(Debug, Default)]
pub struct HintIndex {
    cards: Vec<IndexedCard>,

    /// 卡片 id → 上一次给出的时间（Unix 秒）。
    shown: HashMap<String, i64>,

    /// 卡片 id → 点了「知道了」的那天（北京时间）。
    dismissed: HashMap<String, LocalDate>,

    /// 上一次给出、还在显示的那张：接着命中时不受节流挡。
    current: Option<String>,
}

impl HintIndex {
    /// `segment` 把卡片文字切成词（会话里用引擎的语言模型切）。只收确认过的卡（手写的都确认过）。
    pub fn build(cards: &[Card], segment: impl Fn(&str) -> Vec<String>) -> Self {
        let cards = cards
            .iter()
            .filter(|card| card.confirmed)
            .map(|card| {
                let mut terms: Vec<String> = Vec::new();
                let keywords = card.keywords.iter().map(|k| k.trim().to_owned());
                for term in keywords.chain(segment(&card.text)) {
                    if is_term(&term) && !terms.contains(&term) {
                        terms.push(term);
                    }
                }
                IndexedCard {
                    id: card.id.clone(),
                    text: card.text.clone(),
                    kind: card.kind,
                    when: card.when.as_deref().and_then(LocalDate::parse),
                    touched_at: card.touched_at,
                    terms,
                }
            })
            .collect();
        Self {
            cards,
            ..Self::default()
        }
    }

    /// 卡片变了时重建：还在的卡带上节流时间与正显示的状态；「知道了」整份带过去
    /// （里面也有别的对象的卡，过期与已删的卡由 `MemoryStore::dismissed` 加载时清）。
    pub fn rebuild(&self, cards: &[Card], segment: impl Fn(&str) -> Vec<String>) -> Self {
        let mut next = Self::build(cards, segment);
        let exists = |id: &str| next.cards.iter().any(|card| card.id == id);
        let shown = self
            .shown
            .iter()
            .filter(|(id, _)| exists(id))
            .map(|(id, at)| (id.clone(), *at))
            .collect();
        let current = self.current.clone().filter(|id| exists(id));
        next.shown = shown;
        next.current = current;
        next.dismissed = self.dismissed.clone();
        next
    }

    /// 「知道了」的记录：卡片 id → 点的那天。
    pub fn dismissed(&self) -> &HashMap<String, LocalDate> {
        &self.dismissed
    }

    /// 换上从 `dismissed.json` 读出来的记录。
    pub fn set_dismissed(&mut self, dismissed: HashMap<String, LocalDate>) {
        self.dismissed = dismissed;
    }

    /// `recent` 是最近上屏的字加当前首选。命中词多的、新改过的优先，最多给一条。
    pub fn match_text(&mut self, recent: &str, now: i64) -> Option<Hint> {
        let today = LocalDate::from_unix(now);
        let mut ranked: Vec<(usize, usize)> = self
            .cards
            .iter()
            .enumerate()
            .map(|(i, card)| {
                (
                    i,
                    card.terms
                        .iter()
                        .filter(|t| recent.contains(t.as_str()))
                        .count(),
                )
            })
            .filter(|(_, hits)| *hits > 0)
            .collect();
        ranked.sort_by(|a, b| {
            b.1.cmp(&a.1)
                .then(self.cards[b.0].touched_at.cmp(&self.cards[a.0].touched_at))
        });
        for (i, _) in ranked {
            let card = &self.cards[i];
            if self.dismissed.get(&card.id) == Some(&today) {
                continue;
            }
            let showing = self.current.as_deref() == Some(card.id.as_str());
            let recently = self
                .shown
                .get(&card.id)
                .is_some_and(|&at| now - at < THROTTLE_SECS);
            if !showing && recently {
                continue;
            }
            let hint = Hint {
                card_id: card.id.clone(),
                text: card.text.clone(),
                reason: HintReason::Match,
                more: self.has_other(&card.id),
            };
            self.shown.insert(card.id.clone(), now);
            self.current = Some(card.id.clone());
            return Some(hint);
        }
        self.current = None;
        None
    }

    /// 日子（按年重复）与约定在今天到 3 天后的，挑最近的一条；「知道了」过的当天不出。
    pub fn today(&self, today: LocalDate, pronoun: Pronoun, name: &str) -> Option<Hint> {
        let (days, card) = self
            .cards
            .iter()
            .filter(|card| self.dismissed.get(&card.id) != Some(&today))
            .filter_map(|card| {
                let days = days_away(card.kind, card.when?, today)?;
                (0..=REMINDER_DAYS).contains(&days).then_some((days, card))
            })
            .min_by_key(|(days, card)| (*days, std::cmp::Reverse(card.touched_at)))?;
        Some(Hint {
            card_id: card.id.clone(),
            text: reminder_text(card.kind, days, pronoun, name, &card.text),
            reason: HintReason::Today,
            more: self.has_other(&card.id),
        })
    }

    /// 除了 `card_id` 这张还有别的卡（提示行的「展开」看得到）。
    fn has_other(&self, card_id: &str) -> bool {
        self.cards.iter().any(|card| card.id != card_id)
    }

    /// `today` 为真：当天不再出；为假：从 `now` 起按刚给出过算，10 分钟内不再出。
    pub fn dismiss(&mut self, card_id: &str, today: bool, now: i64) {
        if today {
            self.dismissed
                .insert(card_id.to_owned(), LocalDate::from_unix(now));
        } else {
            self.shown.insert(card_id.to_owned(), now);
        }
        if self.current.as_deref() == Some(card_id) {
            self.current = None;
        }
    }
}
```

Create `cloud/crates/qingjian-cloud-bridge/src/memory/hint/mod.rs`：

```rust
//! 打字时的提示（spec「2A 本地记忆 · 提示」）：按当前对象的卡片建匹配词，拿最近上屏的字加当前首选去碰；
//! 日子与约定在 3 天内的给一条提醒（日子按年重复，约定只提醒一次）。另有对象卡面板挑卡、提醒文案两个自由函数。

mod entry;
mod index;
mod item;
mod reason;

use std::cmp::Reverse;
use std::collections::HashSet;
use std::sync::LazyLock;

use qingjian_cloud_proto::CardKind;

use self::entry::IndexedCard;
use super::{Card, LocalDate, Pronoun, has_date};

pub use self::index::HintIndex;
pub use self::item::Hint;
pub use self::reason::HintReason;

/// 匹配词至少几个字。
const MIN_TERM_CHARS: usize = 2;

/// 同一张卡给出过之后多久不再出（秒）。
const THROTTLE_SECS: i64 = 10 * 60;

/// 日子与约定提前几天提醒（含当天，0–3）。
const REMINDER_DAYS: i64 = 3;

/// 键盘内对象卡面板最多几张。
const PANEL_CARDS: usize = 3;

static STOPWORDS: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    include_str!("../stopwords.txt")
        .lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .flat_map(str::split_whitespace)
        .collect()
});

fn is_term(word: &str) -> bool {
    word.chars().count() >= MIN_TERM_CHARS && !STOPWORDS.contains(word)
}

/// 离 `today` 还有几天：日子按年重复（看今年或明年的那一天），约定按写的那天；别的种类、日期写错都是 `None`。
pub fn days_away(kind: CardKind, when: LocalDate, today: LocalDate) -> Option<i64> {
    match kind {
        CardKind::Date => Some(today.days_until(when.next_anniversary(today))),
        CardKind::Promise => Some(today.days_until(when)),
        _ => None,
    }
}

/// 日子：「今天是她的生日」「明天是…」「3 天后是…」；约定：「今天：看电影」「明天：…」「3 天后：…」。
pub fn reminder_text(
    kind: CardKind,
    days: i64,
    pronoun: Pronoun,
    name: &str,
    text: &str,
) -> String {
    let when = match days {
        0 => "今天".to_owned(),
        1 => "明天".to_owned(),
        n => format!("{n} 天后"),
    };
    if kind == CardKind::Promise {
        return format!("{when}：{text}");
    }
    format!("{when}是{}的{text}", pronoun.label(name))
}

/// 对象卡面板的卡片：3 天内的日子与约定（近的在前），然后是正在提示的那张，其余按最近改动；最多 3 张。
pub fn panel_cards(mut cards: Vec<Card>, today: LocalDate, focus: Option<&str>) -> Vec<Card> {
    cards.sort_by_key(|card| {
        let days = card
            .when
            .as_deref()
            .and_then(LocalDate::parse)
            .filter(|_| has_date(card.kind))
            .and_then(|when| days_away(card.kind, when, today))
            .filter(|days| (0..=REMINDER_DAYS).contains(days));
        let rank = match (days, focus == Some(card.id.as_str())) {
            (Some(days), _) => (0, days),
            (None, true) => (1, 0),
            (None, false) => (2, 0),
        };
        (rank, Reverse(card.touched_at))
    });
    cards.truncate(PANEL_CARDS);
    cards
}
```

（`index.rs` 里 `today` 用了 `std::cmp::Reverse` 的全路径；`hint/mod.rs` 自己的 `panel_cards` 用 `use std::cmp::Reverse`。）

- [ ] **Step 6: 挂模块并导出**

Replace `cloud/crates/qingjian-cloud-bridge/src/memory/mod.rs` 的模块与导出部分，整份变成：

```rust
//! 本地记忆（spec「2A 本地记忆」）：对象、记忆卡、当前场景的存储，打字时的提示，以及 C 接口。
//! 数据在学习数据目录的 `memory/` 下（iOS 开了完全访问时是 App Group 的 `Qingjian/memory/`）；
//! App 整份读写，键盘只读（「记一笔」「知道了」与当前场景除外）；两个进程的读-改-写都在 `memory/.lock` 的文件锁里。
//! 卡片的种类与来源用 proto 的 `CardKind`、`CardSource`。

mod card;
mod cards_file;
mod contact;
mod dismissed_file;
mod error;
mod hint;
mod local_date;
mod pronoun;
mod recent;
mod snapshot;
mod store;

#[cfg(test)]
mod tests;

use std::time::{SystemTime, UNIX_EPOCH};

use qingjian_cloud_proto::{CardKind, Scene};

use self::cards_file::CardsFile;
use self::dismissed_file::DismissedFile;
use crate::scope::ScopeState;

pub use self::card::Card;
pub use self::contact::Contact;
pub use self::error::MemoryError;
pub use self::hint::{Hint, HintIndex, HintReason, days_away, panel_cards, reminder_text};
pub use self::local_date::LocalDate;
pub use self::pronoun::Pronoun;
pub use self::recent::RecentText;
pub use self::snapshot::MemorySnapshot;
pub use self::store::{DEFAULT_LOCK_TIMEOUT, KEYBOARD_LOCK_TIMEOUT, MemoryStore};

/// 学习数据目录下放记忆的子目录。
pub const MEMORY_DIR: &str = "memory";

/// 恋爱场景最多几个对象。
pub const MAX_CONTACTS: usize = 8;

/// 提示拿最近上屏的多少个字去匹配。
pub const RECENT_CHARS: usize = 24;

/// 对象与卡片的 id：16 字节随机数的小写十六进制（32 位，不含名字）。
pub fn new_id() -> Result<String, MemoryError> {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes).map_err(|error| MemoryError::Io(std::io::Error::other(error)))?;
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

/// 现在的 Unix 秒；系统时钟早于 1970 时当 0。
pub fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| {
            i64::try_from(elapsed.as_secs()).unwrap_or(i64::MAX)
        })
}

/// 只有日子与约定的 `when` 有意义（提醒、面板排序、校验都按它）。
pub fn has_date(kind: CardKind) -> bool {
    matches!(kind, CardKind::Date | CardKind::Promise)
}

/// 当前对象不在名单上（被删了）、不是恋爱场景的人，或当前不在恋爱场景：退回不指定。
pub(crate) fn sanitized_scope(mut state: ScopeState, contacts: &[Contact]) -> ScopeState {
    let known = state.contact_id.as_deref().is_some_and(|id| {
        contacts
            .iter()
            .any(|c| c.id == id && c.scene == Scene::Dating)
    });
    if state.scene != Scene::Dating || !known {
        state.contact_id = None;
    }
    state
}
```

Modify `src/lib.rs`：把 Task 2 加的那行 `pub use self::memory::{…};` 换成：

```rust
pub use self::memory::{
    Card, Contact, DEFAULT_LOCK_TIMEOUT, Hint, HintIndex, HintReason, KEYBOARD_LOCK_TIMEOUT,
    LocalDate, MAX_CONTACTS, MEMORY_DIR, MemoryError, MemorySnapshot, MemoryStore, Pronoun,
    RECENT_CHARS, RecentText, days_away, has_date, new_id, now_unix, panel_cards, reminder_text,
};
```

- [ ] **Step 7: 跑测试看它通过**

Run: `cd /Users/liyuqing/sproot/qingjian-mainline/cloud && cargo test -p qingjian-cloud-bridge memory 2>&1 | tail -20`
Expected: `memory::tests::hint::` 14 个、`date` 5 个、`store` 11 个、`sync` 5 个，`test result: ok. 35 passed`。

- [ ] **Step 8: 全量测试、格式与 clippy**

Run: `cd /Users/liyuqing/sproot/qingjian-mainline/cloud && cargo fmt --all && cargo test -p qingjian-cloud-bridge 2>&1 | grep "test result" && cargo clippy -p qingjian-cloud-bridge --all-targets -- -D warnings 2>&1 | tail -3`
Expected: 全部 `ok`；clippy 末行 `Finished`。

- [ ] **Step 9: 提交**

```bash
cd /Users/liyuqing/sproot/qingjian-mainline
git add cloud/crates/qingjian-cloud-bridge/src/memory cloud/crates/qingjian-cloud-bridge/src/lib.rs
git commit -m "feat(cloud): 桥加记忆提示 HintIndex

匹配词是卡片关键词加切出来的两字以上的词，去掉 100 个停用词；命中词多、新改过的优先，一直命中时接着显示，消失后 10 分钟内同一张不再出，「知道了」当天不出。
日子按年重复（2 月 29 日平年按 28 日）、约定只提醒一次，0–3 天内给提醒；日子「明天是她的生日」、约定「明天：看电影」。
more 是除了这张还有别的卡；只提示确认过的卡。重建索引时按卡片 id 带上节流与「知道了」。
对象卡面板挑卡的规则与最近 24 字的缓冲一起放这里；日期按北京时间算。

```

## Task 4：Session 接入与 C 接口

**Files:** Modify `session/mod.rs`（`Session::open` 用 `ScopedLearner`；上屏路径 `commit` / `take_raw` / `punctuate` / `note_passthrough` 把文字追加进最近 24 字的环形缓冲）；Create `memory/ffi.rs`；Modify `src/lib.rs`、`include/qingjian_bridge.h`。

C 接口（会话参数照现有 `qj_preedit` / `qj_commit` 的约定：现有接口若是全局会话就去掉 `QjSession *s`；返回 `char*` 的都用 `qj_string_free` 释放，全部包 `catch_unwind`，参数为空指针或非法 UTF-8 时返回 NULL / 什么都不做）：

```c
void  qj_scope_set(QjSession *s, const char *scene, const char *contact_id /* 可空 */);
char *qj_scope_get(QjSession *s);                         // {"scene":"dating","contact_id":"…"|null}
char *qj_memory_hint(QjSession *s);                       // Hint 的 JSON 或 NULL
void  qj_memory_dismiss(QjSession *s, const char *card_id, bool today);
char *qj_memory_cards(QjSession *s, const char *contact_id); // 今日相关最多 3 张，JSON 数组
char *qj_memory_note(QjSession *s, const char *contact_id, const char *text); // 成功 NULL，失败 {"code","message"}
char *qj_memory_read(const char *user_dir);               // App 用：{"contacts":[…],"cards":{id:[…]},"state":{…}}
char *qj_memory_write(const char *user_dir, const char *json); // App 用：整份写回；失败 {"code","message"}，code ∈ contact_limit / invalid / io
```

- `qj_scope_set` 换叠加层后调 `engine.learner_mut()` 作废格子缓存，并重建 `HintIndex`；状态写进 `state.json`。
- 私密输入时 `qj_memory_hint` 恒返回 NULL。
- 键盘按 `memory/` 下文件修改时间重载（与 cloud.toml 的做法一致）。

测试（`tests/memory_ffi.rs`）：`qj_scope_set` 往返；提示在命中时出现、私密时不出；`qj_memory_note` 建卡；`qj_memory_write` 第 9 个对象返回 `contact_limit`；导出符号与头文件一致（沿用现有 FFI 测试的核对方式）。

### 与大纲的差异

1. **会话参数保留，名字叫 `session`**：现有接口都显式带会话指针（没有全局会话），新接口照写，参数名与头文件里其他函数一致。
2. **会话里的记忆状态收进新类型 `LiveMemory`**（`session/memory/live.rs`），`Session` 只加一个字段 `memory: Option<LiveMemory>`；会话的记忆方法放 `session/memory/mod.rs`（`impl Session`），`session/mod.rs` 只改构造、上屏路径与 `refresh`。按修改时间重载挂在 `Session::poll`（`session/cloud.rs`）开头。
3. **记忆目录 = `<user_dir>/memory`，`qj_session_open` 签名不变**；`user_dir` 为空的会话没有记忆，`qj_scope_get` / `qj_memory_hint` / `qj_memory_cards` 返回 NULL，`qj_memory_note` 返回 `invalid`。
4. **`qj_memory_read` 的 JSON 多 `revs`、`broken`**（Task 2 差异 3、4），有文件读不了时返回 NULL；`qj_memory_write` 不采纳 `state`，多一个失败码 `conflict`（Task 2 差异 3）。
5. **现有 FFI 测试没有「头文件逐个核对」**（`tests/ffi.rs` 只按签名声明调用），新写 `header_declares_every_export`：扫 `src/**/*.rs` 里的 `extern "C" fn qj_*` 与头文件里的 `qj_*(`，两个集合相等。
6. 私密输入时上屏的字也不进最近 24 字的缓冲、不做匹配（不只是 `qj_memory_hint` 返回 NULL），免得私密内容在恢复后触发提示。
7. `punctuate` / `note_passthrough` 后也更新一次提示（它们不走 `refresh`）。
8. **审计修订（重要 1–5、决定点 3、4）：**
   - **读-改-写都交给 `MemoryStore`：** `set_scope` 调 `update_scope`（锁里重读 `state.json` 与磁盘名单，只改场景与对象，读写失败就不切）；`memory_note` 调 `add_note`（按磁盘名单判断这个人还在、读卡片失败返回 `io` 不写）。会话不再用内存里的名单或旧 `state` 去写。
   - **读不了时留着内存里原来的：** `poll_memory` / 换层时重读名单、`state`、卡片都用 `try_*`，失败就记日志、用原来的（锁屏时不会把名单当成空）。
   - **最近 24 字也在键盘收起与换输入框时清：** `Session::flush` 末尾调 `reset_context()`；新 C 函数 `qj_reset_context(session)`，键盘在宿主换了输入框时调（Task 5 用 `textDocumentProxy.documentIdentifier` 判断）。
   - **重建索引带上节流与「知道了」：** `rebuild_hints` 用 `HintIndex::rebuild`；「知道了」（`today = true`）写进 `memory/dismissed.json`，`LiveMemory::open` 时读回（顺带清 30 天前的与已不存在的卡）。
   - **提示开关按人：** `memory_hint` / `update_hint` 看当前对象的 `hint_on`、`remind_on`。
   - `has_contact` 要求当前对象在名单上找得到（名单刷新后被删的人立刻不出提示）。
9. **键盘写入用 200 毫秒锁超时，拿不到进内存待办（审计会话新加）：** `KEYBOARD_LOCK_TIMEOUT`；`memory_note` / `set_scope` 遇 `LockTimeout` 不报错，进 `LiveMemory.pending`，下次 `refresh` / `poll` / `flush` / 下一次记一笔重试（Step 4 续）；待办上限 32 条；`set_scope` 待办只留最新一次；`memory_note` 按顺序重试、成功才出队；`qj_memory_note` 在 `lock_timeout` 时返回 NULL（与成功一致，头文件与注释写清）；`refresh` 拆成 `refresh`（先重试待办）与 `refresh_candidates`；换对象时先清卡片再读（读不了就记 `cards_stale` 补读，不拿上一个人的卡当这个人的）；`memory_note` 在入队前先校验文字（空、超过 200 字）与对象编号；C 接口错误码补 `lock_timeout`（App 等 2 秒仍拿不到时）。

### 步骤

**Files:**
- Create: `cloud/crates/qingjian-cloud-bridge/src/session/memory/{mod,live,tests}.rs`、`src/session/memory/pending/{mod,note}.rs`、`src/memory/ffi.rs`、`tests/memory_ffi.rs`
- Modify: `src/session/mod.rs`（整份替换，见 Step 5）、`src/session/cloud.rs:116-120`（`poll` 开头）、`src/memory/mod.rs`（`mod ffi;`）、`src/lib.rs:457`（`with` 改 `pub(crate)`）、`include/qingjian_bridge.h:88`（`qj_string_free` 之前）

- [ ] **Step 1: 写失败的测试**

Create `cloud/crates/qingjian-cloud-bridge/tests/memory_ffi.rs`：

```rust
//! 本地记忆的 C 接口：按 C 签名直接调。词库用仓库里的样例 `assets/sample/dict.tsv`（按内容认格式，起名 dict.qj 也能读），
//! 不需要产品数据。最后一个测试核对头文件与导出符号逐个一致。

use std::collections::BTreeSet;
use std::ffi::{CStr, CString, c_char};
use std::path::{Path, PathBuf};
use std::ptr;
use std::time::{Duration, Instant};

use qingjian_cloud_bridge::{
    Session, qj_commit, qj_flush, qj_poll, qj_push, qj_session_free, qj_session_open,
    qj_set_private, qj_string_free,
};
use serde_json::{Value, json};

// Session 在 C 侧是不透明指针，这里只传地址
#[allow(improper_ctypes)]
unsafe extern "C" {
    fn qj_scope_set(session: *mut Session, scene: *const c_char, contact_id: *const c_char);
    fn qj_scope_get(session: *mut Session) -> *mut c_char;
    fn qj_reset_context(session: *mut Session);
    fn qj_memory_hint(session: *mut Session) -> *mut c_char;
    fn qj_memory_dismiss(session: *mut Session, card_id: *const c_char, today: bool);
    fn qj_memory_cards(session: *mut Session, contact_id: *const c_char) -> *mut c_char;
    fn qj_memory_note(
        session: *mut Session,
        contact_id: *const c_char,
        text: *const c_char,
    ) -> *mut c_char;
    fn qj_memory_read(user_dir: *const c_char) -> *mut c_char;
    fn qj_memory_write(user_dir: *const c_char, json: *const c_char) -> *mut c_char;
}

const CONTACT: &str = "0123456789abcdef0123456789abcdef";

const CARD: &str = "fedcba9876543210fedcba9876543210";

fn take(raw: *mut c_char) -> Option<String> {
    if raw.is_null() {
        return None;
    }
    let text = unsafe { CStr::from_ptr(raw) }
        .to_string_lossy()
        .into_owned();
    unsafe { qj_string_free(raw) };
    Some(text)
}

fn json_of(raw: *mut c_char) -> Value {
    serde_json::from_str(&take(raw).expect("应当返回 JSON")).unwrap()
}

fn c(text: &str) -> CString {
    CString::new(text).unwrap()
}

/// 临时的数据目录（只有样例词库）与学习数据目录。
fn dirs(name: &str) -> (PathBuf, PathBuf) {
    let root = std::env::temp_dir().join(format!("qj-memory-ffi-{name}-{}", std::process::id()));
    std::fs::remove_dir_all(&root).ok();
    let data = root.join("data");
    let user = root.join("user");
    std::fs::create_dir_all(&data).unwrap();
    std::fs::create_dir_all(&user).unwrap();
    let sample = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../assets/sample/dict.tsv");
    std::fs::copy(sample, data.join("dict.qj")).unwrap();
    (data, user)
}

/// 经 `qj_memory_write` 放一个恋爱场景的对象与一张带关键词「生日」的卡。
fn seed(user: &Path) {
    let mut cards = serde_json::Map::new();
    cards.insert(
        CONTACT.to_owned(),
        json!([{
            "id": CARD, "kind": "other", "text": "想要一个生日蛋糕", "keywords": ["生日"],
            "when": null, "source": "manual", "confirmed": true,
            "created_at": 1_791_043_200, "touched_at": 1_791_043_200
        }]),
    );
    let snapshot = json!({
        "contacts": [{"id": CONTACT, "name": "小美", "pronoun": "ta_f", "scene": "dating", "created_at": 1_791_043_200}],
        "cards": cards,
        "state": {"scene": "daily", "contact_id": null}
    });
    let dir = c(user.to_str().unwrap());
    let text = c(&snapshot.to_string());
    assert_eq!(
        take(unsafe { qj_memory_write(dir.as_ptr(), text.as_ptr()) }),
        None
    );
}

fn open(data: &Path, user: Option<&Path>) -> *mut Session {
    let data = c(data.to_str().unwrap());
    let user = user.map(|dir| c(dir.to_str().unwrap()));
    let user_ptr = user.as_ref().map_or(ptr::null(), |dir| dir.as_ptr());
    let session = unsafe { qj_session_open(data.as_ptr(), user_ptr, ptr::null(), ptr::null()) };
    assert!(!session.is_null());
    session
}

fn set_scope(session: *mut Session, scene: &str, contact: Option<&str>) {
    let scene = c(scene);
    let contact = contact.map(c);
    let contact_ptr = contact.as_ref().map_or(ptr::null(), |id| id.as_ptr());
    unsafe { qj_scope_set(session, scene.as_ptr(), contact_ptr) };
}

fn type_and_commit(session: *mut Session, keys: &str) -> String {
    for key in keys.chars() {
        unsafe { qj_push(session, key as u32) };
    }
    take(unsafe { qj_commit(session, 0) }).unwrap()
}

#[test]
fn scope_set_round_trips() {
    let (data, user) = dirs("scope");
    seed(&user);
    let session = open(&data, Some(&user));
    let scope = json_of(unsafe { qj_scope_get(session) });
    assert_eq!(scope["scene"], "daily");
    assert_eq!(scope["contact_id"], Value::Null);

    set_scope(session, "dating", Some(CONTACT));
    let scope = json_of(unsafe { qj_scope_get(session) });
    assert_eq!(scope["scene"], "dating");
    assert_eq!(scope["contact_id"], CONTACT);
    unsafe { qj_session_free(session) };

    // 写进了 state.json，下次打开还在
    let session = open(&data, Some(&user));
    assert_eq!(
        json_of(unsafe { qj_scope_get(session) })["contact_id"],
        CONTACT
    );
    set_scope(session, "work", Some(CONTACT));
    let scope = json_of(unsafe { qj_scope_get(session) });
    assert_eq!(scope["scene"], "work");
    assert_eq!(scope["contact_id"], Value::Null, "非恋爱场景不带对象");
    set_scope(session, "party", None);
    assert_eq!(
        json_of(unsafe { qj_scope_get(session) })["scene"],
        "work",
        "不认识的场景不动"
    );
    set_scope(session, "dating", Some("ffffffffffffffffffffffffffffffff"));
    assert_eq!(
        json_of(unsafe { qj_scope_get(session) })["contact_id"],
        Value::Null,
        "名单上没有的对象当不指定"
    );
    unsafe { qj_session_free(session) };
}

#[test]
fn hint_shows_on_match_and_hides_when_private() {
    let (data, user) = dirs("hint");
    seed(&user);
    let session = open(&data, Some(&user));
    assert_eq!(type_and_commit(session, "shengri"), "生日");
    assert!(
        take(unsafe { qj_memory_hint(session) }).is_none(),
        "日常场景不出提示"
    );

    set_scope(session, "dating", Some(CONTACT));
    assert!(
        take(unsafe { qj_memory_hint(session) }).is_none(),
        "换对象时清了最近的字"
    );
    assert_eq!(type_and_commit(session, "shengri"), "生日");
    let hint = json_of(unsafe { qj_memory_hint(session) });
    assert_eq!(hint["card_id"], CARD);
    assert_eq!(hint["reason"], "match");
    assert_eq!(hint["text"], "想要一个生日蛋糕");

    unsafe { qj_set_private(session, true) };
    assert!(
        take(unsafe { qj_memory_hint(session) }).is_none(),
        "私密输入不出提示"
    );
    unsafe { qj_set_private(session, false) };
    assert!(take(unsafe { qj_memory_hint(session) }).is_some());

    let card = c(CARD);
    unsafe { qj_memory_dismiss(session, card.as_ptr(), true) };
    assert!(take(unsafe { qj_memory_hint(session) }).is_none());
    assert_eq!(type_and_commit(session, "shengri"), "生日");
    assert!(
        take(unsafe { qj_memory_hint(session) }).is_none(),
        "「知道了」当天不再出"
    );
    unsafe { qj_session_free(session) };

    // 记进了 dismissed.json：键盘重开也记得
    let session = open(&data, Some(&user));
    assert_eq!(type_and_commit(session, "shengri"), "生日");
    assert!(take(unsafe { qj_memory_hint(session) }).is_none());
    assert!(user.join("memory/dismissed.json").is_file());
    unsafe { qj_session_free(session) };
}

#[test]
fn flush_and_new_field_clear_recent_text() {
    let (data, user) = dirs("reset");
    seed(&user);
    let session = open(&data, Some(&user));
    set_scope(session, "dating", Some(CONTACT));
    assert_eq!(type_and_commit(session, "shengri"), "生日");
    assert!(take(unsafe { qj_memory_hint(session) }).is_some());
    unsafe { qj_flush(session) };
    assert!(
        take(unsafe { qj_memory_hint(session) }).is_none(),
        "键盘收起清掉"
    );
    assert_eq!(type_and_commit(session, "dianying"), "电影");
    assert!(
        take(unsafe { qj_memory_hint(session) }).is_none(),
        "旧的「生日」不再触发"
    );

    let (data, user) = dirs("reset-field");
    seed(&user);
    let other = open(&data, Some(&user));
    set_scope(other, "dating", Some(CONTACT));
    assert_eq!(type_and_commit(other, "shengri"), "生日");
    unsafe { qj_reset_context(other) };
    assert_eq!(type_and_commit(other, "dianying"), "电影");
    assert!(
        take(unsafe { qj_memory_hint(other) }).is_none(),
        "换了输入框，旧的字不算"
    );
    unsafe { qj_session_free(session) };
    unsafe { qj_session_free(other) };
}

#[test]
fn hint_switch_is_per_contact() {
    let (data, user) = dirs("hint-off");
    seed(&user);
    let dir = c(user.to_str().unwrap());
    let mut snapshot = json_of(unsafe { qj_memory_read(dir.as_ptr()) });
    snapshot["contacts"][0]["hint_on"] = json!(false);
    let text = c(&snapshot.to_string());
    assert_eq!(
        take(unsafe { qj_memory_write(dir.as_ptr(), text.as_ptr()) }),
        None
    );
    let session = open(&data, Some(&user));
    set_scope(session, "dating", Some(CONTACT));
    assert_eq!(type_and_commit(session, "shengri"), "生日");
    assert!(
        take(unsafe { qj_memory_hint(session) }).is_none(),
        "这个人关了打字时提示"
    );
    unsafe { qj_session_free(session) };
}

#[test]
fn forgotten_contact_stays_forgotten() {
    let (data, user) = dirs("forgotten");
    seed(&user);
    let session = open(&data, Some(&user));
    set_scope(session, "dating", Some(CONTACT));
    let dir = c(user.to_str().unwrap());
    let empty = c(r#"{"contacts":[],"cards":{}}"#);
    assert_eq!(
        take(unsafe { qj_memory_write(dir.as_ptr(), empty.as_ptr()) }),
        None
    );
    let contact = c(CONTACT);
    let text = c("又记一笔");
    let failure = json_of(unsafe { qj_memory_note(session, contact.as_ptr(), text.as_ptr()) });
    assert_eq!(
        failure["code"], "invalid",
        "键盘内存里的名单还没刷新，按磁盘判断"
    );
    unsafe { qj_flush(session) };
    assert!(
        !user.join("memory").join(CONTACT).exists(),
        "对象目录没被重新建出来"
    );
    unsafe { qj_session_free(session) };
}

#[test]
fn note_creates_a_manual_card() {
    let (data, user) = dirs("note");
    seed(&user);
    let session = open(&data, Some(&user));
    let contact = c(CONTACT);
    let text = c("  周末一起看电影 ");
    assert_eq!(
        take(unsafe { qj_memory_note(session, contact.as_ptr(), text.as_ptr()) }),
        None
    );

    let dir = c(user.to_str().unwrap());
    let snapshot = json_of(unsafe { qj_memory_read(dir.as_ptr()) });
    let cards = snapshot["cards"][CONTACT].as_array().unwrap();
    assert_eq!(cards.len(), 2);
    let note = &cards[1];
    assert_eq!(note["kind"], "other");
    assert_eq!(note["text"], "周末一起看电影");
    assert_eq!(note["source"], "manual");
    assert_eq!(note["confirmed"], true);
    assert_eq!(note["id"].as_str().unwrap().len(), 32);

    let panel = json_of(unsafe { qj_memory_cards(session, contact.as_ptr()) });
    assert_eq!(panel.as_array().unwrap().len(), 2);

    let stranger = c("ffffffffffffffffffffffffffffffff");
    let failure = json_of(unsafe { qj_memory_note(session, stranger.as_ptr(), text.as_ptr()) });
    assert_eq!(failure["code"], "invalid");
    let blank = c("   ");
    let failure = json_of(unsafe { qj_memory_note(session, contact.as_ptr(), blank.as_ptr()) });
    assert_eq!(failure["code"], "invalid");
    unsafe { qj_session_free(session) };
}

#[test]
fn ninth_dating_contact_is_rejected() {
    let (_, user) = dirs("limit");
    let people = |count: u32| -> Value {
        let contacts: Vec<Value> = (0..count)
            .map(|n| json!({"id": format!("{n:032x}"), "name": format!("人{n}"), "pronoun": "ta", "scene": "dating", "created_at": 0}))
            .collect();
        json!({"contacts": contacts, "cards": {}, "state": {}})
    };
    let dir = c(user.to_str().unwrap());
    let nine = c(&people(9).to_string());
    let failure = json_of(unsafe { qj_memory_write(dir.as_ptr(), nine.as_ptr()) });
    assert_eq!(failure["code"], "contact_limit");
    assert_eq!(failure["message"], "恋爱场景最多 8 个人");
    let eight = c(&people(8).to_string());
    assert_eq!(
        take(unsafe { qj_memory_write(dir.as_ptr(), eight.as_ptr()) }),
        None
    );
    let snapshot = json_of(unsafe { qj_memory_read(dir.as_ptr()) });
    assert_eq!(snapshot["contacts"].as_array().unwrap().len(), 8);
}

#[test]
fn bad_arguments_do_not_crash() {
    assert!(take(unsafe { qj_scope_get(ptr::null_mut()) }).is_none());
    unsafe { qj_scope_set(ptr::null_mut(), ptr::null(), ptr::null()) };
    assert!(take(unsafe { qj_memory_hint(ptr::null_mut()) }).is_none());
    unsafe { qj_memory_dismiss(ptr::null_mut(), ptr::null(), true) };
    assert!(take(unsafe { qj_memory_cards(ptr::null_mut(), ptr::null()) }).is_none());
    assert!(take(unsafe { qj_memory_read(ptr::null()) }).is_none());
    let failure = json_of(unsafe { qj_memory_write(ptr::null(), ptr::null()) });
    assert_eq!(failure["code"], "invalid");
    let failure = json_of(unsafe { qj_memory_note(ptr::null_mut(), ptr::null(), ptr::null()) });
    assert_eq!(failure["code"], "invalid");

    let (data, user) = dirs("bad-args");
    let dir = c(user.to_str().unwrap());
    let broken = c("{");
    let failure = json_of(unsafe { qj_memory_write(dir.as_ptr(), broken.as_ptr()) });
    assert_eq!(failure["code"], "invalid");

    // 没有学习数据目录的会话：没有记忆
    let session = open(&data, None);
    assert!(take(unsafe { qj_scope_get(session) }).is_none());
    set_scope(session, "dating", Some(CONTACT));
    assert!(take(unsafe { qj_memory_hint(session) }).is_none());
    let contact = c(CONTACT);
    let text = c("x");
    let failure = json_of(unsafe { qj_memory_note(session, contact.as_ptr(), text.as_ptr()) });
    assert_eq!(failure["code"], "invalid");
    unsafe { qj_session_free(session) };
}

/// 模拟 App 占着 `memory/.lock`：持有返回的文件就是持有锁，丢掉即释放。
fn hold_lock(user: &Path) -> std::fs::File {
    let dir = user.join("memory");
    std::fs::create_dir_all(&dir).unwrap();
    let file = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(dir.join(".lock"))
        .unwrap();
    file.lock().unwrap();
    file
}

/// 键盘等锁的上限是 200 毫秒；主线程上一次调用最多容忍这么久（加调度与慢机器的余量）。
const KEYBOARD_BUDGET: Duration = Duration::from_millis(900);

#[test]
fn keyboard_note_is_deferred_while_the_lock_is_held() {
    let (data, user) = dirs("note-locked");
    seed(&user);
    let session = open(&data, Some(&user));
    let contact = c(CONTACT);
    let cards_path = user.join("memory").join(CONTACT).join("cards.json");

    let lock = hold_lock(&user);
    for text in ["周末一起看电影", "她喜欢喝茶"] {
        let text = c(text);
        let started = Instant::now();
        let failure = take(unsafe { qj_memory_note(session, contact.as_ptr(), text.as_ptr()) });
        let elapsed = started.elapsed();
        eprintln!("锁被占着时 qj_memory_note 用了 {elapsed:?}");
        assert_eq!(failure, None, "拿不到锁视同已接受，不报错");
        assert!(elapsed >= Duration::from_millis(150), "应当等满键盘的超时");
        assert!(elapsed < KEYBOARD_BUDGET, "主线程不能卡 2 秒：{elapsed:?}");
    }
    let on_disk = std::fs::read_to_string(&cards_path).unwrap();
    assert!(!on_disk.contains("周末一起看电影"), "锁没放，还没写进磁盘");

    drop(lock);
    unsafe { qj_push(session, 'n' as u32) };
    let dir = c(user.to_str().unwrap());
    let snapshot = json_of(unsafe { qj_memory_read(dir.as_ptr()) });
    let texts: Vec<&str> = snapshot["cards"][CONTACT]
        .as_array()
        .unwrap()
        .iter()
        .map(|card| card["text"].as_str().unwrap())
        .collect();
    assert_eq!(
        texts,
        ["想要一个生日蛋糕", "周末一起看电影", "她喜欢喝茶"],
        "下一次按键补写成功，顺序不变"
    );
    unsafe { qj_session_free(session) };
}

#[test]
fn keyboard_note_retries_on_poll_and_flush_too() {
    let (data, user) = dirs("note-poll");
    seed(&user);
    let session = open(&data, Some(&user));
    let contact = c(CONTACT);
    let text = c("经 poll 补写");
    let lock = hold_lock(&user);
    assert_eq!(
        take(unsafe { qj_memory_note(session, contact.as_ptr(), text.as_ptr()) }),
        None
    );
    drop(lock);
    unsafe { qj_poll(session) };
    let dir = c(user.to_str().unwrap());
    let snapshot = json_of(unsafe { qj_memory_read(dir.as_ptr()) });
    assert_eq!(snapshot["cards"][CONTACT].as_array().unwrap().len(), 2);

    let lock = hold_lock(&user);
    let text = c("经 flush 补写");
    assert_eq!(
        take(unsafe { qj_memory_note(session, contact.as_ptr(), text.as_ptr()) }),
        None
    );
    drop(lock);
    unsafe { qj_flush(session) };
    let snapshot = json_of(unsafe { qj_memory_read(dir.as_ptr()) });
    assert_eq!(snapshot["cards"][CONTACT].as_array().unwrap().len(), 3);
    unsafe { qj_session_free(session) };
}

#[test]
fn keyboard_scope_switch_is_deferred_and_keeps_only_the_latest() {
    let (data, user) = dirs("scope-locked");
    seed(&user);
    let session = open(&data, Some(&user));

    let lock = hold_lock(&user);
    let started = Instant::now();
    set_scope(session, "work", None);
    set_scope(session, "dating", Some(CONTACT));
    let elapsed = started.elapsed();
    eprintln!("锁被占着时两次 qj_scope_set 用了 {elapsed:?}");
    assert!(
        elapsed < KEYBOARD_BUDGET * 2,
        "每次最多等 200 毫秒：{elapsed:?}"
    );
    let scope = json_of(unsafe { qj_scope_get(session) });
    assert_eq!(scope["scene"], "dating", "内存里照切");
    assert_eq!(scope["contact_id"], CONTACT);
    // 磁盘上还没写，轮询也不能把刚切的读回旧的
    assert!(
        !user.join("memory/state.json").exists() || {
            !std::fs::read_to_string(user.join("memory/state.json"))
                .unwrap()
                .contains("dating")
        }
    );
    unsafe { qj_poll(session) };
    assert_eq!(
        json_of(unsafe { qj_scope_get(session) })["contact_id"],
        CONTACT
    );

    drop(lock);
    unsafe { qj_push(session, 's' as u32) };
    let state: Value =
        serde_json::from_str(&std::fs::read_to_string(user.join("memory/state.json")).unwrap())
            .unwrap();
    assert_eq!(state["scene"], "dating", "待办只留最后一次，补写成功");
    assert_eq!(state["contact_id"], CONTACT);
    unsafe { qj_session_free(session) };
}

/// 键盘扩展被系统杀掉：Session 丢了，待办笔记靠 `pending-keyboard.jsonl` 在下次启动时补写。
#[test]
fn pending_note_survives_the_session_being_dropped() {
    let (data, user) = dirs("note-restart");
    seed(&user);
    let pending_file = user.join("memory/pending-keyboard.jsonl");
    let session = open(&data, Some(&user));
    let contact = c(CONTACT);
    let text = c("被杀前记的");
    let lock = hold_lock(&user);
    assert_eq!(
        take(unsafe { qj_memory_note(session, contact.as_ptr(), text.as_ptr()) }),
        None
    );
    assert!(
        std::fs::read_to_string(&pending_file)
            .unwrap()
            .contains("被杀前记的"),
        "入队时就落盘"
    );
    unsafe { qj_session_free(session) };
    drop(lock);

    let session = open(&data, Some(&user));
    unsafe { qj_poll(session) };
    let dir = c(user.to_str().unwrap());
    let snapshot = json_of(unsafe { qj_memory_read(dir.as_ptr()) });
    let texts: Vec<&str> = snapshot["cards"][CONTACT]
        .as_array()
        .unwrap()
        .iter()
        .map(|card| card["text"].as_str().unwrap())
        .collect();
    assert_eq!(texts, ["想要一个生日蛋糕", "被杀前记的"]);
    assert!(!pending_file.exists(), "补写成功后文件清掉");
    unsafe { qj_session_free(session) };
}

#[test]
fn restored_note_for_a_forgotten_contact_is_dropped() {
    let (data, user) = dirs("note-forgotten");
    seed(&user);
    let session = open(&data, Some(&user));
    let contact = c(CONTACT);
    let text = c("忘掉的人的笔记");
    let lock = hold_lock(&user);
    assert_eq!(
        take(unsafe { qj_memory_note(session, contact.as_ptr(), text.as_ptr()) }),
        None
    );
    unsafe { qj_session_free(session) };
    drop(lock);

    let dir = c(user.to_str().unwrap());
    let empty = c(r#"{"contacts":[],"cards":{}}"#);
    assert_eq!(
        take(unsafe { qj_memory_write(dir.as_ptr(), empty.as_ptr()) }),
        None
    );
    let session = open(&data, Some(&user));
    unsafe { qj_poll(session) };
    assert!(
        !user.join("memory").join(CONTACT).exists(),
        "对象目录没被复活"
    );
    assert!(
        !user.join("memory/pending-keyboard.jsonl").exists(),
        "被拒绝的待办丢掉"
    );
    unsafe { qj_session_free(session) };
}

#[test]
fn header_declares_every_export() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let header = std::fs::read_to_string(root.join("include/qingjian_bridge.h")).unwrap();
    let declared: BTreeSet<String> = header
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .flat_map(called_names)
        .collect();
    let mut exported = BTreeSet::new();
    collect_exports(&root.join("src"), &mut exported);
    assert!(exported.contains("qj_memory_write"), "{exported:?}");
    assert_eq!(declared, exported);
}

/// 一行里所有紧跟 `(` 的 `qj_xxx`。
fn called_names(line: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut rest = line;
    while let Some(start) = rest.find("qj_") {
        let tail = &rest[start..];
        let end = tail
            .find(|c: char| !(c.is_ascii_lowercase() || c == '_'))
            .unwrap_or(tail.len());
        if tail[end..].starts_with('(') {
            names.push(tail[..end].to_owned());
        }
        rest = &tail[end..];
    }
    names
}

/// `src/` 下所有 `extern "C" fn qj_xxx(` 的名字。
fn collect_exports(dir: &Path, out: &mut BTreeSet<String>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            collect_exports(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            for line in std::fs::read_to_string(&path).unwrap().lines() {
                if let Some((_, tail)) = line.split_once("extern \"C\" fn ")
                    && let Some((name, _)) = tail.split_once('(')
                {
                    out.insert(name.trim().to_owned());
                }
            }
        }
    }
}
```

- [ ] **Step 2: 跑测试看它失败**

Run: `cd /Users/liyuqing/sproot/qingjian-mainline/cloud && cargo test -p qingjian-cloud-bridge --test memory_ffi 2>&1 | tail -8`
Expected: 链接失败，`undefined symbols … _qj_scope_set`、`_qj_reset_context` 等（或 `undefined reference to qj_scope_set`）。

- [ ] **Step 3: 写 `LiveMemory`**

Create `cloud/crates/qingjian-cloud-bridge/src/session/memory/live.rs`：

```rust
//! 会话里的本地记忆状态：当前场景与对象、换层把手、名单、当前对象的卡片与提示索引、最近上屏的字、两条提示。
//! 键盘只读记忆文件（「记一笔」「知道了」与当前场景除外），App 改了按修改时间重载（见 `Session::poll_memory`）。
//! 读不了（锁屏时数据保护挡住）就留着内存里原来的，不当成空；
//! 写时拿不到文件锁（键盘只等 200 毫秒）的「记一笔」与切场景先放进内存待办，之后重试（见 `pending`）。

use std::collections::HashSet;
use std::path::Path;
use std::time::SystemTime;

use qingjian_cloud_proto::Scene;

use super::pending::PendingWrites;
use crate::memory::{
    Card, Contact, Hint, HintIndex, KEYBOARD_LOCK_TIMEOUT, LocalDate, MemoryStore, RecentText,
    sanitized_scope,
};
use crate::scope::{ScopeHandle, ScopeState, ScopedLearner};

pub(crate) struct LiveMemory {
    pub(super) store: MemoryStore,

    pub(super) handle: ScopeHandle,

    pub(super) state: ScopeState,

    pub(super) contacts: Vec<Contact>,

    /// 当前对象的卡片；没选对象时为空。
    pub(super) cards: Vec<Card>,

    pub(super) hints: HintIndex,

    pub(super) recent: RecentText,

    /// 切到对象时算出的日子提醒，优先于匹配提示；「知道了」后清掉。
    pub(super) today: Option<Hint>,

    /// 最近一次 refresh 匹配到的提示。
    pub(super) current: Option<Hint>,

    /// `contacts.json`、`state.json`、当前对象 `cards.json` 上次读时的修改时间。
    pub(super) stamp: [Option<SystemTime>; 3],

    /// 拿不到锁、等着重试的写入。
    pub(super) pending: PendingWrites,

    /// 换了对象但卡片还没读进来（当时拿不到锁或读不了）：先当没有卡，refresh 时补读。
    pub(super) cards_stale: bool,
}

impl LiveMemory {
    /// 读名单与 `state.json`（对象不在名单上就退回不指定），按当前场景开分区学习器，读入「知道了」的记录。
    /// 提示索引由会话随后建（要用引擎的语言模型切词）。
    pub(in crate::session) fn open(user_dir: &Path) -> (ScopedLearner, Self) {
        let store = MemoryStore::open_with_lock_timeout(user_dir, KEYBOARD_LOCK_TIMEOUT);
        let contacts = store.contacts();
        let state = sanitized_scope(store.state(), &contacts);
        let learner = ScopedLearner::open(
            user_dir,
            store.root(),
            state.scene,
            state.contact_id.as_deref(),
        );
        let handle = learner.handle();
        let cards = state
            .contact_id
            .as_deref()
            .map(|id| store.cards(id))
            .unwrap_or_default();
        let known: HashSet<String> = store
            .snapshot()
            .map(|snapshot| {
                snapshot
                    .cards
                    .into_values()
                    .flatten()
                    .map(|card| card.id)
                    .collect()
            })
            .unwrap_or_default();
        let mut hints = HintIndex::default();
        hints.set_dismissed(store.dismissed(LocalDate::today(), &known));
        let stamp = store.stamp(state.contact_id.as_deref());
        let root = store.root().to_path_buf();
        let memory = Self {
            store,
            handle,
            state,
            contacts,
            cards,
            hints,
            recent: RecentText::default(),
            today: None,
            current: None,
            stamp,
            pending: PendingWrites::open(&root),
            cards_stale: false,
        };
        (learner, memory)
    }

    /// 只在恋爱场景、选了对象时出提示（私密输入由会话另挡）。
    pub(super) fn has_contact(&self) -> bool {
        self.state.scene == Scene::Dating && self.contact().is_some()
    }

    pub(super) fn contact(&self) -> Option<&Contact> {
        let id = self.state.contact_id.as_deref()?;
        self.contacts.iter().find(|c| c.id == id)
    }

    /// 重读名单；读不了时留着原来的。
    pub(super) fn reload_contacts(&mut self) {
        match self.store.try_contacts() {
            Ok(contacts) => self.contacts = contacts,
            Err(error) => tracing::warn!(%error, "名单读不了，先用原来的"),
        }
    }

    /// 重读当前对象的卡片与修改时间（换对象、「记一笔」、App 改了之后）；读不了时留着原来的，返回是否读成了。
    pub(super) fn reload_cards(&mut self) -> bool {
        let id = self.state.contact_id.clone();
        let loaded = match id.as_deref().map(|id| self.store.try_cards(id)) {
            None => {
                self.cards.clear();
                true
            }
            Some(Ok(cards)) => {
                self.cards = cards;
                true
            }
            Some(Err(error)) => {
                tracing::warn!(%error, "卡片读不了，先用原来的");
                false
            }
        };
        self.stamp = self.store.stamp(id.as_deref());
        if loaded {
            self.cards_stale = false;
        }
        loaded
    }

    /// 最近上屏的字与正在显示的匹配提示都清掉（键盘收起、换了输入框、换了对象）。
    pub(super) fn forget_context(&mut self) {
        self.recent.clear();
        self.current = None;
    }
}
```

- [ ] **Step 4: 会话的记忆方法**

Create `cloud/crates/qingjian-cloud-bridge/src/session/memory/mod.rs`：

```rust
//! 会话里的本地记忆接口（C 接口在 `crate::memory::ffi`）：切场景与对象、取提示、对象卡、「记一笔」、「知道了」，
//! 以及上屏路径喂进来的最近 24 字、换输入框时清空、按修改时间重载。读-改-写都交给 `MemoryStore`（文件锁里读磁盘再写）。
//! 键盘只等 200 毫秒的锁：拿不到时「记一笔」与切场景进内存待办，下次 refresh / poll / flush 或下一次记一笔时重试，主线程不卡。

mod live;
mod pending;

#[cfg(test)]
mod tests;

use qingjian_cloud_proto::{MAX_CARD_TEXT_CHARS, Scene};
use qingjian_core::sentence::{LanguageModel, segment_text};

use super::Session;
use crate::entry::Entry;
use crate::memory::{Card, Hint, LocalDate, MemoryError, now_unix, panel_cards, sanitized_scope};
use crate::scope::{ScopeState, is_contact_id};

use self::pending::PendingNote;

pub(super) use self::live::LiveMemory;

impl Session {
    /// 切场景与对象：交给 `MemoryStore::update_scope` 在锁里重读 `state.json` 与名单，只改这两个字段。
    /// 非恋爱场景、磁盘名单上没有的对象都当不指定。读写失败（锁屏）就不切，记日志；
    /// 只是拿不到锁（`LockTimeout`）时内存里照切，写盘进待办（只留最新一次）稍后重试。
    pub fn set_scope(&mut self, scene: Scene, contact: Option<&str>) {
        let Some(memory) = self.memory.as_mut() else {
            return;
        };
        let mut deferred = false;
        let next = match memory.store.update_scope(scene, contact) {
            Ok(state) => {
                memory.pending.take_scope();
                state
            }
            Err(MemoryError::LockTimeout) => {
                deferred = true;
                memory.pending.set_scope(scene, contact.map(str::to_owned));
                let wanted = ScopeState {
                    scene,
                    contact_id: contact.map(str::to_owned),
                };
                sanitized_scope(wanted, &memory.contacts)
            }
            Err(error) => {
                tracing::warn!(%error, "切场景没写进 state.json，不切");
                return;
            }
        };
        let moved = next != memory.state;
        memory.state = next;
        if moved {
            self.switch_layers(deferred);
        }
    }

    /// 当前场景与对象；没有学习数据目录的会话没有记忆，返回 `None`。
    pub fn scope(&self) -> Option<ScopeState> {
        self.memory.as_ref().map(|memory| memory.state.clone())
    }

    /// 提示行要显示的：日子提醒优先，其次是匹配提示，各按当前对象的两个开关。
    /// 私密输入、非恋爱场景、没选对象时没有。
    pub fn memory_hint(&self) -> Option<&Hint> {
        if self.engine.is_private() {
            return None;
        }
        let memory = self.memory.as_ref()?;
        if !memory.has_contact() {
            return None;
        }
        let contact = memory.contact()?;
        memory
            .today
            .as_ref()
            .filter(|_| contact.remind_on)
            .or_else(|| memory.current.as_ref().filter(|_| contact.hint_on))
    }

    /// 「知道了」：`today` 为真当天不再出这张卡（写进 `dismissed.json`，键盘重启也记得），为假 10 分钟内不再出。
    pub fn dismiss_hint(&mut self, card_id: &str, today: bool) {
        let Some(memory) = self.memory.as_mut() else {
            return;
        };
        memory.hints.dismiss(card_id, today, now_unix());
        if today && let Err(error) = memory.store.put_dismissed(memory.hints.dismissed()) {
            tracing::warn!(%error, "「知道了」没记下来");
        }
        if memory
            .today
            .as_ref()
            .is_some_and(|hint| hint.card_id == card_id)
        {
            memory.today = None;
        }
        if memory
            .current
            .as_ref()
            .is_some_and(|hint| hint.card_id == card_id)
        {
            memory.current = None;
        }
    }

    /// 键盘内对象卡面板：今日相关的最多 3 张（规则见 [`panel_cards`]）。
    pub fn memory_cards(&self, contact_id: &str) -> Vec<Card> {
        let Some(memory) = self.memory.as_ref() else {
            return Vec::new();
        };
        let cards = if memory.state.contact_id.as_deref() == Some(contact_id) {
            memory.cards.clone()
        } else {
            memory.store.cards(contact_id)
        };
        let focus = memory.current.as_ref().map(|hint| hint.card_id.as_str());
        panel_cards(cards, LocalDate::today(), focus)
    }

    /// 键盘「记一笔」：交给 `MemoryStore::add_note`（锁里按磁盘名单判断这个人还在、读卡片失败就不写）。
    /// 拿不到锁（`LockTimeout`）时不报错，进内存待办稍后补写（视同成功）；前面还有没写进去的就排在后面，保持顺序。
    pub fn memory_note(&mut self, contact_id: &str, text: &str) -> Result<(), MemoryError> {
        if self.memory.is_none() {
            return Err(MemoryError::Invalid("这个键盘没有记忆目录"));
        }
        let text = text.trim();
        if text.is_empty() {
            return Err(MemoryError::Invalid("没有要记的文字"));
        }
        if text.chars().count() > MAX_CARD_TEXT_CHARS {
            return Err(MemoryError::Invalid("一张卡最多 200 个字"));
        }
        if !is_contact_id(contact_id) {
            return Err(MemoryError::Invalid("对象编号不对"));
        }
        self.retry_pending();
        let Some(memory) = self.memory.as_mut() else {
            return Ok(());
        };
        let now = now_unix();
        let later = PendingNote {
            contact_id: contact_id.to_owned(),
            text: text.to_owned(),
            at: now,
        };
        if memory.pending.note_count() > 0 {
            memory.pending.push_note(later);
            return Ok(());
        }
        match memory.store.add_note(contact_id, text, now) {
            Ok(_) => {}
            Err(MemoryError::LockTimeout) => {
                memory.pending.push_note(later);
                return Ok(());
            }
            Err(error) => return Err(error),
        }
        if memory.state.contact_id.as_deref() == Some(contact_id) {
            memory.reload_cards();
            self.rebuild_hints();
        }
        Ok(())
    }

    /// 重试拿不到锁时放进待办的写入：「记一笔」按顺序补写（成功才出队，被拒绝的丢掉），再补写最新一次切场景，
    /// 最后补读换对象时没读成的卡片。仍拿不到锁或读写不了就留着，这一轮到此为止（最多再等一个 200 毫秒）；
    /// 补写的场景换了叠加层返回 true。
    pub(super) fn retry_pending(&mut self) -> bool {
        let Some(memory) = self.memory.as_mut() else {
            return false;
        };
        if memory.pending.is_empty() && !memory.cards_stale {
            return false;
        }
        let mut wrote_note = false;
        let mut blocked = false;
        while let Some(note) = memory.pending.front_note() {
            match memory.store.add_note(&note.contact_id, &note.text, note.at) {
                Ok(_) => {
                    wrote_note = true;
                    memory.pending.pop_note();
                }
                Err(error @ (MemoryError::LockTimeout | MemoryError::Io(_))) => {
                    tracing::warn!(%error, "待写的记一笔先留着");
                    blocked = true;
                    break;
                }
                Err(error) => {
                    tracing::warn!(%error, "待写的记一笔被拒绝，丢掉");
                    memory.pending.pop_note();
                }
            }
        }
        let mut moved = false;
        if !blocked && let Some(scope) = memory.pending.take_scope() {
            match memory.store.update_scope(scope.0, scope.1.as_deref()) {
                Ok(state) => {
                    moved = state != memory.state;
                    memory.state = state;
                }
                Err(error @ (MemoryError::LockTimeout | MemoryError::Io(_))) => {
                    tracing::warn!(%error, "待写的场景先留着");
                    memory.pending.restore_scope(scope);
                    blocked = true;
                }
                Err(error) => tracing::warn!(%error, "待写的场景被拒绝，丢掉"),
            }
        }
        if moved {
            self.switch_layers(false);
            return true;
        }
        let Some(memory) = self.memory.as_mut() else {
            return false;
        };
        if wrote_note || (memory.cards_stale && !blocked) {
            memory.reload_cards();
            self.rebuild_hints();
        }
        false
    }

    /// 宿主换了输入框（或键盘收起）：最近上屏的字清掉，免得在 A 聊天里打的字在 B 里触发提示。
    pub fn reset_context(&mut self) {
        if let Some(memory) = self.memory.as_mut() {
            memory.forget_context();
        }
    }

    /// 上屏路径（选词、回车原样、标点、直通的空格回车）调：私密输入时不记。
    pub(super) fn note_committed(&mut self, text: &str) {
        if self.engine.is_private() {
            return;
        }
        if let Some(memory) = self.memory.as_mut() {
            memory.recent.push_str(text);
        }
    }

    /// 每次 refresh 后：最近 24 字加当前首选去碰当前对象的卡片。
    pub(super) fn update_hint(&mut self) {
        let first = self
            .entries
            .first()
            .map(Entry::text)
            .unwrap_or_default()
            .to_owned();
        let private = self.engine.is_private();
        let Some(memory) = self.memory.as_mut() else {
            return;
        };
        let hint_on = memory.contact().is_some_and(|contact| contact.hint_on);
        if private || !memory.has_contact() || !hint_on {
            memory.current = None;
            return;
        }
        let probe = format!("{}{first}", memory.recent.text());
        memory.current = memory.hints.match_text(&probe, now_unix());
    }

    /// 按当前对象的卡片重建索引（切词用引擎的语言模型；节流与「知道了」带过去），并算一次日子提醒。
    pub(super) fn rebuild_hints(&mut self) {
        let Some(memory) = self.memory.as_mut() else {
            return;
        };
        let model = self.engine.language_model();
        memory.hints = memory
            .hints
            .rebuild(&memory.cards, |text| words_of(text, model));
        memory.current = None;
        memory.today = memory.contact().and_then(|contact| {
            memory
                .hints
                .today(LocalDate::today(), contact.pronoun, &contact.name)
        });
    }

    /// `Session::poll` 开头调：`memory/` 下的文件被 App 改了（修改时间变了）就重读，读不了的留着原来的；
    /// 当前对象被删时退回「恋爱 · 不指定」。换了叠加层（候选重排过）返回 true。
    pub(super) fn poll_memory(&mut self) -> bool {
        let rescoped = self.retry_pending();
        let Some(memory) = self.memory.as_mut() else {
            return false;
        };
        let stamp = memory.store.stamp(memory.state.contact_id.as_deref());
        if stamp == memory.stamp {
            return rescoped;
        }
        // 还有没写进磁盘的（锁一直被占着）：以内存里的为准，别读回旧的把刚切的切回去，也不再多等几个 200 毫秒
        if !memory.pending.is_empty() || memory.cards_stale {
            return rescoped;
        }
        memory.reload_contacts();
        let disk = match memory.store.try_state() {
            Ok(state) => state,
            Err(error) => {
                tracing::warn!(%error, "state.json 读不了，先用原来的");
                memory.state.clone()
            }
        };
        let next = sanitized_scope(disk, &memory.contacts);
        let moved = next != memory.state;
        memory.state = next;
        if moved {
            self.switch_layers(false);
        } else {
            memory.reload_cards();
            self.rebuild_hints();
        }
        moved || rescoped
    }

    /// 状态里的场景或对象变了：换叠加层、清最近的字，作废格子缓存后重读名单与卡片（`deferred` 为真说明刚拿不到锁，
    /// 不再读盘，卡片先当没有、记下待补读）、重建提示、重排候选。
    fn switch_layers(&mut self, deferred: bool) {
        let Some(memory) = self.memory.as_mut() else {
            return;
        };
        memory
            .handle
            .switch(memory.state.scene, memory.state.contact_id.as_deref());
        memory.forget_context();
        memory.cards.clear();
        memory.cards_stale = deferred;
        if !deferred {
            memory.reload_contacts();
            memory.reload_cards();
        }
        // 叠加层换了，格子缓存里的排序作废（learner_mut 会清缓存）
        self.engine.learner_mut();
        self.rebuild_hints();
        self.refresh_candidates();
    }
}

/// 卡片文字切成词；没有语言模型（或一个词都不认识）时切不出来，只靠关键词。
fn words_of(text: &str, model: &dyn LanguageModel) -> Vec<String> {
    segment_text(text, model)
        .into_iter()
        .flatten()
        .flatten()
        .collect()
}
```

- [ ] **Step 4（续）：键盘待办队列（审计会话新加）**

键盘只等 200 毫秒的锁（`MemoryStore::open_with_lock_timeout(user_dir, KEYBOARD_LOCK_TIMEOUT)`，`LiveMemory::open` 里已经这么开）。拿不到（`LockTimeout`）时不报错、不卡主线程，而是把写入放进 `LiveMemory.pending`：
`memory_note` 按顺序排队、上限 32 条（超出丢最旧的并 `tracing::warn!`）、重试成功才出队、被拒绝（`invalid` 等）的丢掉、`io` 的留着；`set_scope` 内存里照切、写盘只留最新一次。
`Session::retry_pending` 在每次 `refresh`（按键）、`poll`（`poll_memory` 开头）、`flush` 与下一次 `memory_note` 开头重试，一轮遇到第一个超时就停（最多再等 200 毫秒）；`switch_layers(deferred)` 在刚超时时不再读盘（卡片先当没有，`cards_stale` 记下，下次补读）；
`poll_memory` 在还有待办或卡片没补读时不读盘（免得读回旧的 `state.json` 把刚切的切回去）。`qj_memory_note` 在 `lock_timeout` 时返回 NULL（已接受，稍后写入），与成功一致；App 侧（`qj_memory_read` / `qj_memory_write`）仍等 2 秒。

**待办笔记落盘（审计会话建议）：** 键盘扩展随时可能被系统杀掉，内存里的待办会丢，所以笔记队列同时写 `memory/pending-keyboard.jsonl`（一行一条 `PendingNote`）。只有键盘一个写者，不需要 flock；选的写法是**队列一变就整份写临时文件再改名**（复用 `write_atomic(.., create_parent = false)`，读到的不会是半截行；文件始终等于队列，32 条上限自然一致；队列空了删文件），不用追加（追加要另想删行、截断与半行）。`memory/` 不存在时不建目录、不报错。`LiveMemory::open` 时 `PendingWrites::open` 读回来（坏行跳过并 `tracing::warn!`，超过 32 条只留最后 32 条），下次 refresh / poll / flush 补写；对象已被忘掉的在补写时被 `add_note` 拒绝而丢弃，不复活目录。切场景的待办不落盘（丢了只是回到上次的场景）。

Create `cloud/crates/qingjian-cloud-bridge/src/session/memory/pending/note.rs`：

```rust
//! 一条等着写的「记一笔」，也是 `pending-keyboard.jsonl` 里的一行。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(in crate::session) struct PendingNote {
    pub(in crate::session) contact_id: String,

    pub(in crate::session) text: String,

    /// 用户点「记」的时间（Unix 秒），补写时仍按这个时间建卡。
    pub(in crate::session) at: i64,
}
```

Create `cloud/crates/qingjian-cloud-bridge/src/session/memory/pending/mod.rs`：

```rust
//! 键盘拿不到 `memory/.lock`（200 毫秒超时）时先记在内存里的写入：「记一笔」按顺序排队，切场景只留最新一次。
//! 下次 refresh、poll、flush 或下一次写入时重试；卡片与场景在内存里已经生效，只是磁盘上晚几步。
//! 笔记队列同时落在 `memory/pending-keyboard.jsonl`（一行一条），键盘扩展被系统杀掉也不丢，下次启动读回来接着补写；切场景不落盘。
//! 只有键盘这一个进程写这个文件，不需要 flock：每次队列变了就整份写临时文件再改名（读到的不会是半截），
//! 文件始终等于队列，上限自然一致；队列空了就删文件。`memory/` 不存在时不建、不报错。

mod note;

use std::collections::VecDeque;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use qingjian_cloud_proto::Scene;

use crate::cloud_config::write_atomic;

pub(in crate::session) use self::note::PendingNote;

/// 待写的「记一笔」最多几条，超出丢最旧的。
pub(in crate::session) const MAX_PENDING_NOTES: usize = 32;

/// 落盘文件在 `memory/` 下的名字。
pub(in crate::session) const PENDING_FILE: &str = "pending-keyboard.jsonl";

#[derive(Debug, Default)]
pub(in crate::session) struct PendingWrites {
    notes: VecDeque<PendingNote>,

    /// 笔记队列落盘的文件；没有记忆目录时为空（不落盘）。
    file: Option<PathBuf>,

    scope: Option<(Scene, Option<String>)>,
}

impl PendingWrites {
    /// 读回上次被杀时留下的笔记（坏行跳过并记日志，只留最后 [`MAX_PENDING_NOTES`] 条）。
    pub(in crate::session) fn open(memory_dir: &Path) -> Self {
        let file = memory_dir.join(PENDING_FILE);
        let mut notes = VecDeque::new();
        match std::fs::read_to_string(&file) {
            Ok(text) => {
                for line in text.lines().filter(|line| !line.trim().is_empty()) {
                    match serde_json::from_str::<PendingNote>(line) {
                        Ok(note) => notes.push_back(note),
                        Err(error) => tracing::warn!(%error, "待写笔记里有一行坏了，跳过"),
                    }
                }
                while notes.len() > MAX_PENDING_NOTES {
                    notes.pop_front();
                }
            }
            Err(error) if error.kind() == ErrorKind::NotFound => {}
            Err(error) => tracing::warn!(%error, "待写笔记文件读不了"),
        }
        Self {
            notes,
            file: Some(file),
            scope: None,
        }
    }

    /// 队列变了就整份落盘；空了删文件。目录不在（`memory/` 还没建）时不写也不报错。
    fn persist(&self) {
        let Some(file) = self.file.as_deref() else {
            return;
        };
        let result = if self.notes.is_empty() {
            std::fs::remove_file(file)
        } else {
            let mut text = String::new();
            for note in &self.notes {
                if let Ok(line) = serde_json::to_string(note) {
                    text.push_str(&line);
                    text.push('\n');
                }
            }
            write_atomic(file, text.as_bytes(), false)
        };
        match result {
            Err(error) if error.kind() != ErrorKind::NotFound => {
                tracing::warn!(%error, "待写笔记没落盘");
            }
            _ => {}
        }
    }

    pub(in crate::session) fn is_empty(&self) -> bool {
        self.notes.is_empty() && self.scope.is_none()
    }

    #[cfg(test)]
    pub(in crate::session) fn has_scope(&self) -> bool {
        self.scope.is_some()
    }

    pub(in crate::session) fn note_count(&self) -> usize {
        self.notes.len()
    }

    /// 排到队尾；满了丢最旧的（记日志）。
    pub(in crate::session) fn push_note(&mut self, note: PendingNote) {
        if self.notes.len() >= MAX_PENDING_NOTES {
            self.notes.pop_front();
            tracing::warn!("待写的记一笔太多，丢掉最旧的一条");
        }
        self.notes.push_back(note);
        self.persist();
    }

    pub(in crate::session) fn front_note(&self) -> Option<&PendingNote> {
        self.notes.front()
    }

    pub(in crate::session) fn pop_note(&mut self) -> Option<PendingNote> {
        let note = self.notes.pop_front();
        self.persist();
        note
    }

    /// 后一次覆盖前一次。
    pub(in crate::session) fn set_scope(&mut self, scene: Scene, contact: Option<String>) {
        self.scope = Some((scene, contact));
    }

    pub(in crate::session) fn take_scope(&mut self) -> Option<(Scene, Option<String>)> {
        self.scope.take()
    }

    /// 重试没成功时放回去；期间若有更新的一次（重入）就不覆盖它。
    pub(in crate::session) fn restore_scope(&mut self, scope: (Scene, Option<String>)) {
        self.scope.get_or_insert(scope);
    }
}
```

Create `cloud/crates/qingjian-cloud-bridge/src/session/memory/tests.rs`（队列的单元测试；拿不到文件锁的端到端行为在 `tests/memory_ffi.rs` 的 `keyboard_note_is_deferred_while_the_lock_is_held`、`keyboard_note_retries_on_poll_and_flush_too`、`keyboard_scope_switch_is_deferred_and_keeps_only_the_latest`，用 `hold_lock` 占住 `memory/.lock`）：

```rust
//! 键盘待办队列：「记一笔」按顺序、有上限、落盘，切场景只留最新一次。拿不到文件锁的端到端行为见 `tests/memory_ffi.rs`。

use qingjian_cloud_proto::Scene;

use std::path::PathBuf;

use super::pending::{MAX_PENDING_NOTES, PENDING_FILE, PendingNote, PendingWrites};

fn note(n: usize) -> PendingNote {
    PendingNote {
        contact_id: "0123456789abcdef0123456789abcdef".to_owned(),
        text: format!("第 {n} 条"),
        at: i64::try_from(n).unwrap(),
    }
}

#[test]
fn pending_notes_keep_order_and_drop_the_oldest_over_the_cap() {
    let mut pending = PendingWrites::default();
    assert!(pending.is_empty());
    for n in 0..MAX_PENDING_NOTES + 5 {
        pending.push_note(note(n));
    }
    assert_eq!(pending.note_count(), MAX_PENDING_NOTES);
    assert_eq!(
        pending.front_note().unwrap().text,
        "第 5 条",
        "丢的是最旧的 5 条"
    );
    let mut texts = Vec::new();
    while let Some(next) = pending.pop_note() {
        texts.push(next.at);
    }
    let expected: Vec<i64> = (5..i64::try_from(MAX_PENDING_NOTES).unwrap() + 5).collect();
    assert_eq!(texts, expected);
    assert!(pending.is_empty());
}

#[test]
fn pending_scope_keeps_only_the_latest() {
    let mut pending = PendingWrites::default();
    pending.set_scope(Scene::Work, None);
    pending.set_scope(Scene::Dating, Some("a".repeat(32)));
    assert!(pending.has_scope());
    assert_eq!(
        pending.take_scope(),
        Some((Scene::Dating, Some("a".repeat(32))))
    );
    assert!(!pending.has_scope());

    // 重试失败放回时，不压过期间新来的一次
    pending.set_scope(Scene::Daily, None);
    pending.restore_scope((Scene::Work, None));
    assert_eq!(pending.take_scope(), Some((Scene::Daily, None)));
}

fn memory_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("qj-pending-{name}-{}", std::process::id()));
    std::fs::remove_dir_all(&dir).ok();
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn pending_notes_survive_a_restart_and_the_file_follows_the_queue() {
    let dir = memory_dir("restart");
    let mut pending = PendingWrites::open(&dir);
    pending.push_note(note(1));
    pending.push_note(note(2));
    let reopened = PendingWrites::open(&dir);
    assert_eq!(reopened.note_count(), 2);
    assert_eq!(reopened.front_note(), Some(&note(1)));

    pending.pop_note();
    assert_eq!(PendingWrites::open(&dir).front_note(), Some(&note(2)));
    pending.pop_note();
    assert!(!dir.join(PENDING_FILE).exists(), "队列空了就删文件");
}

#[test]
fn a_corrupt_line_is_skipped_and_the_rest_restored() {
    let dir = memory_dir("corrupt");
    let good = |n| serde_json::to_string(&note(n)).unwrap();
    let text = format!("{}\n{{坏了\n\n{}\n", good(1), good(3));
    std::fs::write(dir.join(PENDING_FILE), text).unwrap();
    let mut pending = PendingWrites::open(&dir);
    assert_eq!(pending.pop_note(), Some(note(1)));
    assert_eq!(pending.pop_note(), Some(note(3)));
    assert_eq!(pending.pop_note(), None);
}

#[test]
fn the_file_obeys_the_same_cap_as_memory() {
    let dir = memory_dir("cap");
    let mut pending = PendingWrites::open(&dir);
    for n in 0..MAX_PENDING_NOTES + 5 {
        pending.push_note(note(n));
    }
    let lines = std::fs::read_to_string(dir.join(PENDING_FILE)).unwrap();
    assert_eq!(lines.lines().count(), MAX_PENDING_NOTES);
    let mut reopened = PendingWrites::open(&dir);
    assert_eq!(reopened.pop_note(), Some(note(5)), "文件里丢的也是最旧的");

    // 手工写了超过上限的文件，读回来也只留最后 32 条
    let many: String = (0..MAX_PENDING_NOTES + 3)
        .map(|n| format!("{}\n", serde_json::to_string(&note(n)).unwrap()))
        .collect();
    std::fs::write(dir.join(PENDING_FILE), many).unwrap();
    let reopened = PendingWrites::open(&dir);
    assert_eq!(reopened.note_count(), MAX_PENDING_NOTES);
    assert_eq!(reopened.front_note(), Some(&note(3)));
}

#[test]
fn a_missing_memory_dir_is_neither_created_nor_an_error() {
    let dir = memory_dir("nodir").join("memory");
    let mut pending = PendingWrites::open(&dir);
    pending.push_note(note(1));
    pending.pop_note();
    assert!(!dir.exists());
}
```

- [ ] **Step 5: 改 `session/mod.rs`**

Replace `cloud/crates/qingjian-cloud-bridge/src/session/mod.rs`（整份；与原文件相比：文件头补一句、`mod memory;`、`memory` 字段、`open` 里学习器换成 `LiveMemory::open` 并在最后 `rebuild_hints`、四个上屏方法喂缓冲、`refresh` 不再提前 `return` 并在最后 `update_hint`、`flush` 末尾清最近的字、删掉 `load_learner`（挪进 `scope::load_layer`））：

```rust
//! 一次键盘会话：持有 Engine，每次缓冲变化后重查一遍候选缓存起来，给 C ABI 按下标取。
//! 配了青简 Cloud 时还挂着大模型联想、润色与学习数据同步（见 `cloud.rs`）；
//! 有学习数据目录时挂着本地记忆：场景 / 对象分区学习与打字提示（见 `memory/`）。

mod cloud;
mod config;
mod memory;

use std::path::{Path, PathBuf};

use qingjian_cloud_client::DataSync;
use qingjian_core::{Engine, Learner, SurroundingText};
use qingjian_dictionary::Dictionary;
use qingjian_learning::{FrequencyLearner, InputLog};
use qingjian_lm::BigramModel;

use self::memory::LiveMemory;
use crate::clipboard::Clipboard;
use crate::cloud_config::CloudConfig;
use crate::entry::Entry;
use crate::error::BridgeError;
use crate::rewrite::Rewriter;

/// 候选栏是横向滚动的一行，再多也翻不到，截断省得每键复制几百个候选。
const MAX_CANDIDATES: usize = 120;

pub struct Session {
    engine: Engine,

    /// 候选栏里的格子，下标与 Swift 那边显示的一致；云端结果回来后插在首选之后。
    entries: Vec<Entry>,

    /// 拼音行：Core 切好音节、补了 `'` 的显示串；没在组句时为空。
    preedit: String,

    /// 学习数据目录；同步的收件箱也在这下面。
    user_dir: Option<PathBuf>,

    /// 宿主光标前后的文字，发联想请求时带上。
    context: Option<SurroundingText>,

    data_sync: Option<DataSync>,

    rewriter: Option<Rewriter>,

    clipboard: Option<Clipboard>,

    /// 与 Mac 同格式的 `config.toml`（模糊音、双拼、繁体、领域词库、自定义短语等）与它上次套用时的修改时间。
    config_path: Option<PathBuf>,

    config_modified: Option<std::time::SystemTime>,

    /// 随包领域词库所在目录（`Data/dicts`）。
    dicts_dir: PathBuf,

    /// 青简 Cloud 的连接配置；云联想选青简 Cloud 时端点从这里来。离线为 `None`。
    cloud: Option<CloudConfig>,

    /// 本地记忆；没有学习数据目录（只在内存里学）时为 `None`，学习器也就不分区。
    memory: Option<LiveMemory>,
}

impl Session {
    /// `data_dir` 里要有 `dict.qj`，`lm.qj` 可选（没有就退回词频整句）；
    /// `user_dir` 给了就从 `user.tsv` 读学习数据并在 [`Self::flush`] 时写回，记忆在它下面的 `memory/`；没给只在内存里学、没有记忆；
    /// `config` 是设置文件 `config.toml`，不给就用 `user_dir` 下的（iOS 上没有完全访问时学习数据在扩展容器、设置在 App Group，两处分开）；
    /// `cloud` 给了就接上大模型与同步，没给完全离线。
    pub fn open(
        data_dir: &Path,
        user_dir: Option<&Path>,
        config: Option<&Path>,
        cloud: Option<CloudConfig>,
    ) -> Result<Self, BridgeError> {
        let dictionary = Dictionary::from_path(data_dir.join("dict.qj"))?;
        let (learner, memory): (Box<dyn Learner>, Option<LiveMemory>) = match user_dir {
            Some(dir) => {
                let (learner, memory) = LiveMemory::open(dir);
                (Box::new(learner), Some(memory))
            }
            None => (Box::new(FrequencyLearner::default()), None),
        };
        let mut engine = Engine::new(dictionary).with_learner(learner);
        let lm = data_dir.join("lm.qj");
        if lm.is_file() {
            match BigramModel::from_path(&lm) {
                Ok(model) => engine = engine.with_language_model(Box::new(model)),
                Err(error) => tracing::warn!(%error, "语言模型加载失败，使用词频整句"),
            }
        }
        // 只在登录了且开了「上传输入日志」时记日志：离线或没开的用户，输入不落任何日志
        if let (Some(dir), Some(cloud)) = (user_dir, &cloud)
            && cloud.logs
        {
            engine =
                engine.with_input_logger(Box::new(InputLog::open(dir.join("input-log.jsonl"))));
        }
        let mut session = Self {
            engine,
            entries: Vec::new(),
            preedit: String::new(),
            user_dir: user_dir.map(Path::to_path_buf),
            context: None,
            data_sync: None,
            rewriter: None,
            clipboard: None,
            config_path: config
                .map(Path::to_path_buf)
                .or_else(|| user_dir.map(|dir| dir.join("config.toml"))),
            config_modified: None,
            dicts_dir: data_dir.join("dicts"),
            cloud: cloud.clone(),
            memory,
        };
        session.reload_config();
        if let Some(cloud) = cloud {
            session.connect(&cloud);
        }
        session.apply_inbox();
        session.rebuild_hints();
        Ok(session)
    }

    pub fn composing(&self) -> bool {
        !self.engine.composition().is_empty()
    }

    pub fn preedit(&self) -> &str {
        &self.preedit
    }

    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    pub fn push(&mut self, c: char) {
        self.engine.push(c);
        self.refresh();
    }

    pub fn backspace(&mut self) {
        self.engine.backspace();
        self.refresh();
    }

    pub fn clear(&mut self) {
        self.engine.clear();
        self.refresh();
    }

    /// 上屏第 `index` 格；本地候选只吃掉一部分拼音时剩下的留在缓冲区，接着出候选。
    pub fn commit(&mut self, index: usize) -> Option<String> {
        let text = match self.entries.get(index)? {
            Entry::Local(candidate) | Entry::Cloud(candidate) => {
                let candidate = candidate.clone();
                self.engine.commit(&candidate)
            }
            Entry::Sentence(sentence) => {
                let sentence = sentence.clone();
                self.engine.accept_prediction(&sentence)
            }
        };
        self.note_committed(&text);
        self.refresh();
        Some(text)
    }

    /// 敲过的字母原样上屏（回车）。
    pub fn take_raw(&mut self) -> String {
        let text = self.engine.take_raw();
        self.note_committed(&text);
        self.refresh();
        text
    }

    /// 没在组句时的标点：中文模式转全角，不需要转的原样返回并记成直通字符。
    pub fn punctuate(&mut self, c: char) -> String {
        let text = match self.engine.punctuate(c) {
            Some(text) => text.to_owned(),
            None => {
                self.engine.note_passthrough(c);
                c.to_string()
            }
        };
        self.note_committed(&text);
        self.update_hint();
        text
    }

    /// 没在组字时直接输出的字符（空格、回车）告诉引擎，输入日志里的句子边界才对。
    pub fn note_passthrough(&mut self, c: char) {
        self.engine.note_passthrough(c);
        self.note_committed(c.encode_utf8(&mut [0; 4]));
        self.update_hint();
    }

    /// 键盘收起或进入后台时调，学习数据落盘（键盘扩展随时可能被系统杀掉），清掉最近上屏的字，再催一轮同步。
    pub fn flush(&mut self) {
        self.retry_pending();
        self.engine.flush_learning();
        self.reset_context();
        self.sync_now();
    }

    /// 每次按键后：先补写拿不到锁时留下的待办，再重查候选。
    fn refresh(&mut self) {
        self.retry_pending();
        self.refresh_candidates();
    }

    fn refresh_candidates(&mut self) {
        self.entries.clear();
        self.preedit.clear();
        if self.composing() {
            match self.engine.query() {
                Ok(query) => {
                    self.preedit = query.marked_text();
                    self.entries.extend(
                        query
                            .candidates
                            .items
                            .into_iter()
                            .take(MAX_CANDIDATES)
                            .map(Entry::Local),
                    );
                }
                // 拼不成音节（如 `vvv`）：显示原样输入，没有候选，回车原样上屏。
                Err(_) => self.preedit = self.engine.composition().text().to_owned(),
            }
            self.request_prediction();
        } else {
            self.engine.cancel_prediction();
        }
        self.update_hint();
    }
}
```

Modify `cloud/crates/qingjian-cloud-bridge/src/session/cloud.rs` 的 `poll`：第 116–120 行

```rust
    /// 键盘可见期间定时调：合并收件箱、取回大模型结果。候选栏变了返回 true。
    pub fn poll(&mut self) -> bool {
        self.apply_inbox();
        // 设置变了（主 App 改的或从 Mac 同步来的）候选要重排
        let reloaded = self.reload_config() && self.composing();
```

换成

```rust
    /// 键盘可见期间定时调：合并收件箱、按修改时间重载记忆、取回大模型结果。候选栏变了返回 true。
    pub fn poll(&mut self) -> bool {
        self.apply_inbox();
        let rescoped = self.poll_memory();
        // 设置变了（主 App 改的或从 Mac 同步来的）、App 删了当前对象（换了叠加层），候选要重排
        let config_changed = self.reload_config();
        let reloaded = (config_changed || rescoped) && self.composing();
```

（其余不变。）

- [ ] **Step 6: C 接口**

Create `cloud/crates/qingjian-cloud-bridge/src/memory/ffi.rs`：

```rust
//! 本地记忆的 C 接口，与 `include/qingjian_bridge.h` 一一对应。`qj_scope_*` 与键盘用的 `qj_memory_*` 带会话（只在主线程上用）；
//! `qj_memory_read` / `qj_memory_write` 是 App 用的，按学习数据目录传（与 `qj_settings_*` 同一做法）。
//! 返回的字符串都用 `qj_string_free` 释放；失败返回 `{"code","message"}`（见 [`MemoryError`]）。全部折掉 panic。

use std::ffi::c_char;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::Path;
use std::ptr;

use super::{MemoryError, MemorySnapshot, MemoryStore};
use crate::scope::{parse_scene, scene_name};
use crate::session::Session;
use crate::{owned, path_arg, with};

/// 切场景与对象；`contact_id` 可为空（不指定）。场景认不得时什么都不做。
///
/// # Safety
/// `session` 来自 `qj_session_open` 且未释放；`scene` 为有效 UTF-8 C 字符串，`contact_id` 为空或同上。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qj_scope_set(
    session: *mut Session,
    scene: *const c_char,
    contact_id: *const c_char,
) {
    let Some(scene) = (unsafe { path_arg(scene) }).and_then(parse_scene) else {
        return;
    };
    let contact = unsafe { path_arg(contact_id) }.map(str::to_owned);
    with(session, (), |s| s.set_scope(scene, contact.as_deref()));
}

/// `{"scene":"dating","contact_id":"…"|null}`；没有记忆的会话返回空指针。
///
/// # Safety
/// 同 [`qj_scope_set`]。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qj_scope_get(session: *mut Session) -> *mut c_char {
    with(session, ptr::null_mut(), |s| {
        s.scope().map_or(ptr::null_mut(), |state| {
            let json = serde_json::json!({
                "scene": scene_name(state.scene),
                "contact_id": state.contact_id,
            });
            owned(&json.to_string())
        })
    })
}

/// 宿主换了输入框：清掉最近上屏的字与正在显示的匹配提示（键盘收起时 `qj_flush` 也会清）。
///
/// # Safety
/// 同 [`qj_scope_set`]。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qj_reset_context(session: *mut Session) {
    with(session, (), Session::reset_context);
}

/// 当前提示 `{"card_id","text","reason","more"}`；没有时返回空指针（私密输入时恒为空）。
///
/// # Safety
/// 同 [`qj_scope_set`]。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qj_memory_hint(session: *mut Session) -> *mut c_char {
    with(session, ptr::null_mut(), |s| {
        s.memory_hint()
            .and_then(|hint| serde_json::to_string(hint).ok())
            .map_or(ptr::null_mut(), |json| owned(&json))
    })
}

/// 「知道了」：`today` 为真当天不再出，为假 10 分钟内不再出。
///
/// # Safety
/// 同 [`qj_scope_set`]；`card_id` 为有效 UTF-8 C 字符串。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qj_memory_dismiss(
    session: *mut Session,
    card_id: *const c_char,
    today: bool,
) {
    let Some(card_id) = (unsafe { path_arg(card_id) }).map(str::to_owned) else {
        return;
    };
    with(session, (), |s| s.dismiss_hint(&card_id, today));
}

/// 键盘内对象卡面板：今日相关最多 3 张卡的 JSON 数组；没有记忆的会话返回空指针。
///
/// # Safety
/// 同 [`qj_scope_set`]；`contact_id` 为有效 UTF-8 C 字符串。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qj_memory_cards(
    session: *mut Session,
    contact_id: *const c_char,
) -> *mut c_char {
    let Some(contact_id) = (unsafe { path_arg(contact_id) }).map(str::to_owned) else {
        return ptr::null_mut();
    };
    with(session, ptr::null_mut(), |s| {
        if s.scope().is_none() {
            return ptr::null_mut();
        }
        serde_json::to_string(&s.memory_cards(&contact_id))
            .map_or(ptr::null_mut(), |json| owned(&json))
    })
}

/// 键盘「记一笔」：给对象建一张 `other` 卡。成功返回空指针，失败返回 `{"code","message"}`。
/// 键盘只等 200 毫秒的锁：拿不到（`lock_timeout`）时也返回空指针，表示已接受、稍后写入（内存待办，下次按键、poll、flush 时补写）。
///
/// # Safety
/// 同 [`qj_scope_set`]；两个字符串参数为有效 UTF-8 C 字符串。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qj_memory_note(
    session: *mut Session,
    contact_id: *const c_char,
    text: *const c_char,
) -> *mut c_char {
    let (Some(contact_id), Some(text)) = (
        unsafe { path_arg(contact_id) }.map(str::to_owned),
        unsafe { path_arg(text) }.map(str::to_owned),
    ) else {
        return owned(&MemoryError::Invalid("参数无效").to_json());
    };
    // 会话为空或 panic 时 with 给的是这个兜底；不能用空指针兜底，空指针在这里表示成功
    let noted = with(session, Err(MemoryError::Invalid("参数无效")), |s| {
        s.memory_note(&contact_id, &text)
    });
    match noted {
        Ok(()) => ptr::null_mut(),
        Err(error) => owned(&error.to_json()),
    }
}

/// App 用：整份读出 `{"contacts","cards","revs","state","broken"}`；参数无效或有文件读不了（锁屏）时返回空指针。
///
/// # Safety
/// `user_dir` 为有效 UTF-8 C 字符串。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qj_memory_read(user_dir: *const c_char) -> *mut c_char {
    let Some(user_dir) = (unsafe { path_arg(user_dir) }) else {
        return ptr::null_mut();
    };
    catch_unwind(|| {
        let snapshot = MemoryStore::open(Path::new(user_dir)).snapshot().ok()?;
        serde_json::to_string(&snapshot).ok()
    })
    .ok()
    .flatten()
    .map_or(ptr::null_mut(), |json| owned(&json))
}

/// App 用：整份写回。成功返回空指针，失败返回 `{"code","message"}`，code 取 `contact_limit` / `invalid` / `conflict` / `lock_timeout`（等了 2 秒没拿到锁）/ `io`；
/// `conflict` 表示键盘这期间改过，App 重读、合并后再写。
///
/// # Safety
/// 两个参数为有效 UTF-8 C 字符串。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qj_memory_write(
    user_dir: *const c_char,
    json: *const c_char,
) -> *mut c_char {
    let (Some(user_dir), Some(json)) = (unsafe { path_arg(user_dir) }, unsafe { path_arg(json) })
    else {
        return owned(&MemoryError::Invalid("参数无效").to_json());
    };
    let written = catch_unwind(AssertUnwindSafe(|| {
        let snapshot: MemorySnapshot =
            serde_json::from_str(json).map_err(|_| MemoryError::Invalid("数据格式不对"))?;
        MemoryStore::open(Path::new(user_dir)).write_snapshot(&snapshot)
    }));
    match written {
        Ok(Ok(())) => ptr::null_mut(),
        Ok(Err(error)) => owned(&error.to_json()),
        Err(_) => owned(&MemoryError::Invalid("写入时出错").to_json()),
    }
}
```

Modify `cloud/crates/qingjian-cloud-bridge/src/memory/mod.rs`：模块列表里 `mod error;` 之后加一行 `mod ffi;`。

Modify `cloud/crates/qingjian-cloud-bridge/src/lib.rs`：第 458 行 `fn with<T>(` 改成 `pub(crate) fn with<T>(`。

- [ ] **Step 7: 头文件**

Modify `cloud/crates/qingjian-cloud-bridge/include/qingjian_bridge.h`：在第 88 行 `void qj_string_free(char *text);` 之前插入：

```c
// 本地记忆（素笺 2A）：场景、对象、打字提示、对象卡、「记一笔」。会话没有学习数据目录（user_dir 为 NULL）时都是空操作 / 返回 NULL。
// App 与键盘的读-改-写都在 memory/.lock 的文件锁里做。
// scene 取 daily / dating / work；contact_id 是 32 位十六进制，可为 NULL（不指定）；非恋爱场景、磁盘名单上没有的对象都当不指定。
// 切换时在锁里重读 memory/state.json、只改场景与对象再写回；锁屏读不了就不切。候选按新的分区学习重排。
// 键盘只等 200 毫秒的锁：拿不到时内存里照切，写盘进待办（只留最新一次），下次按键、qj_poll、qj_flush 时补写。
void qj_scope_set(QjSession *session, const char *scene, const char *contact_id);
// {"scene":"dating","contact_id":"…"|null}
char *qj_scope_get(QjSession *session);
// 宿主换了输入框时调：清掉最近上屏的字与正在显示的匹配提示（qj_flush 也会清）。
void qj_reset_context(QjSession *session);
// 当前提示 {"card_id","text","reason":"match"|"today","more":bool}（more：除了这张还有别的卡）；
// 没有、私密输入、非恋爱场景、没选对象或这个人的开关关着时为 NULL。
char *qj_memory_hint(QjSession *session);
// 「知道了」：today 为 true 时当天不再出这张卡（记进 memory/dismissed.json），false 时 10 分钟内不再出。
void qj_memory_dismiss(QjSession *session, const char *card_id, bool today);
// 键盘内对象卡面板：今日相关最多 3 张卡的 JSON 数组。
char *qj_memory_cards(QjSession *session, const char *contact_id);
// 「记一笔」：给磁盘名单上的对象建一张 other 卡。成功返回 NULL，失败返回 {"code","message"}
// （invalid：没有这个人、没有文字或超过 200 字；io：卡片读不了，例如锁屏，此时什么都不写）。
// 键盘只等 200 毫秒的锁：另一个进程占着锁（lock_timeout）时也返回 NULL，表示已接受、稍后写入：这条记在内存待办里
// （最多 32 条，满了丢最旧的），下次按键、qj_poll、qj_flush 或下一次记一笔时按顺序补写，主线程不会卡住。
char *qj_memory_note(QjSession *session, const char *contact_id, const char *text);
// App 用，user_dir 是 App Group 里的 Qingjian 目录（记忆在它下面的 memory/）。read 返回
// {"contacts":[…],"cards":{id:[…]},"revs":{id:n},"state":{…},"broken":[id…]}（revs 是各对象卡片的修订号；broken 是卡片文件损坏、
// 已备份的对象；参数无效或有文件读不了时为 NULL）。
// write 整份写回：成功返回 NULL，失败返回 {"code","message"}，code 取 contact_limit / invalid / conflict / lock_timeout / io（lock_timeout：App 等了 2 秒还拿不到锁，稍后再试）。
// 某个对象磁盘上的修订号比 revs 新（键盘这期间记过一笔）就整份不写、返回 conflict，App 重读合并后再写；
// 只重写有变化的对象；state 不采纳；名单上没了的对象连目录一起删。
char *qj_memory_read(const char *user_dir);
char *qj_memory_write(const char *user_dir, const char *json);
```

- [ ] **Step 8: 跑测试看它通过**

Run: `cd /Users/liyuqing/sproot/qingjian-mainline/cloud && cargo test -p qingjian-cloud-bridge --test memory_ffi 2>&1 | tail -12`
Expected: `test result: ok. 12 passed; 0 failed`（大纲的四类加头文件核对，另有键盘待办的三个测试（锁被占着时记一笔 / 切场景进待办、释放后补写）、「键盘收起与换输入框清最近的字」「提示开关按人」「忘掉的人不复活」、以及提示测试末尾的「知道了」重开键盘仍记得）。

- [ ] **Step 9: 用真实产品数据跑一遍会话测试（有数据才跑）**

Run: `cd /Users/liyuqing/sproot/qingjian-mainline/cloud && QINGJIAN_DATA=/Users/liyuqing/sproot/qingjian-mainline/data/generated cargo test -p qingjian-cloud-bridge --test session 2>&1 | grep "test result"`
Expected: `test result: ok. 4 passed; 0 failed`（整句、原样上屏、候选串、`logs_only_when_connected`）。**这几个测试靠环境变量 `QINGJIAN_DATA` 指向含 `dict.qj` 的产品数据目录才真正运行，没设就静默跳过并显示 `ok`**，所以上面的命令必须带它，否则「全过」没有意义。`logs_only_when_connected` 原来因桥的四个开关缺省关（账号改造的结果）而失败（测试的 `cloud.toml` 没写 `logs = true`），已由单独提交 `fix(cloud): 桥的会话测试配置补上 logs = true` 修好；若这里仍失败，说明 Task 4 破坏了输入日志的接入，要修，不是已知问题。

- [ ] **Step 10: 全量测试、格式与 clippy**

Run: `cd /Users/liyuqing/sproot/qingjian-mainline/cloud && cargo fmt --all && cargo test -p qingjian-cloud-bridge 2>&1 | grep "test result" && cargo clippy -p qingjian-cloud-bridge --all-targets -- -D warnings 2>&1 | tail -3`
Expected: 全部 `ok`；clippy 末行 `Finished`。

- [ ] **Step 11: iOS 工程照样能编（只加了函数，Swift 还没用）**

Run: `cd /Users/liyuqing/sproot/qingjian-mainline/cloud/ios && scripts/build-bridge.sh && xcodegen generate && xcodebuild -scheme QingjianCloud -destination 'platform=iOS Simulator,name=iPhone 17e' build 2>&1 | tail -3`
Expected: `QingjianBridge.xcframework（release）与产品数据已就绪`，最后 `** BUILD SUCCEEDED **`。

- [ ] **Step 12: 提交**

```bash
cd /Users/liyuqing/sproot/qingjian-mainline
git add cloud/crates/qingjian-cloud-bridge/src cloud/crates/qingjian-cloud-bridge/include/qingjian_bridge.h \
  cloud/crates/qingjian-cloud-bridge/tests/memory_ffi.rs
git commit -m "feat(cloud): 会话接上本地记忆与 qj_scope_* / qj_memory_* 接口

有学习数据目录的会话用 ScopedLearner，记忆在它下面的 memory/，qj_session_open 签名不变。
上屏路径（选词、回车、标点、直通）喂最近 24 字，refresh 后拿它加首选匹配卡片；私密输入不记、不出提示；键盘收起（qj_flush）与换输入框（新的 qj_reset_context）时清掉。
切场景与「记一笔」都交给 MemoryStore 在文件锁里按磁盘上的 state 与名单读-改-写，读不了就不写；poll 按修改时间重载，读不了留着原来的，当前对象被删退回恋爱 · 不指定。
重建提示索引时带上节流与「知道了」，「知道了」写进 dismissed.json；提示开关按人。
App 侧 qj_memory_read / write 整份读写，写回冲突返回 conflict；新加头文件与导出符号逐个核对的测试。

```

## Task 5：iOS 键盘

**Files:** Create `ScopeChip.swift`、`ScopePicker.swift`、`HintRow.swift`、`ContactCardPanel.swift`、`MemoryBridge.swift`；Modify `KeyboardView.swift`（VStack 加提示行）、`IdleBar`（左侧牌子、「记一笔」）、`KeyboardModel.swift`（refresh 后取 `qj_memory_hint`）、`project.yml`（新文件）。

- 牌子：恋爱场景显示「{对象名} · 恋爱」，强调色；日常 / 工作只显示灰色场景名。点它在键位区换成 `ScopePicker`：分段控件（日常 / 恋爱 / 工作）+ 对象格子（≤8）+「不指定」+「新对象」（提示「在素笺 App 里新建」）；底部一行「对象只能你自己切，键盘不知道你在和谁聊」。
- 提示行：有提示时在候选栏上方出现，键盘高度加一行（0.2 秒动画；审计修订后改为恋爱场景选了对象时一直保留这一行，见差异 8）；左侧强调色圆点、文字、右侧「展开」；`reason=today` 时右侧是「知道了」（调 `qj_memory_dismiss(today=true)`）。
- `ContactCardPanel`：替换键位区，头像字、名字、认识天数、最多 3 张卡，底部「全部记忆在素笺 App 里」与「收起」。
- 「记一笔」：只在开了完全访问且剪贴板有文字时显示；点后确认条「记到 {对象}」/「忽略」，确认调 `qj_memory_note`。
- 没开完全访问时牌子点开显示「开启完全访问后才能使用记忆」（App Group 读不到）。

验收：模拟器构建通过；按 01 键盘 1a–1e、05 的 2c 截图对照（结构一致即可，视觉打磨留给子项目 3）；键盘高度变化不遮挡宿主输入框。

### 与大纲的差异

1. **键盘高度在 `KeyboardViewController` 里管**（`mountKeyboard` 钉在 `view.heightAnchor` 上的约束），触摸层的键区与 ⌄ 的位置也在控制器里算；提示行出现时要改这三处，所以**加改 `KeyboardViewController.swift`**（大纲没列）：存下高度约束、`syncHintRow()` 跟着 `model.hint` 改高度（0.2 秒）、键区与 ⌄ 往下挪一行。
2. **`project.yml` 不用改**：各 target 的 `sources` 是目录，xcodegen 自动收录新文件（`Shared/Memory/` 也在 App 与键盘两个 target 里）。
3. **新增 `Shared/Memory/` 一组共用的模型与小工具**（`MemoryContact`、`MemoryPronoun`、`MemoryCard`、`MemoryScope`、`MemorySnapshot`、`MemoryHint`、`MemoryFailure`、`MemoryDate`、`MemoryID`、`MemoryFiles`、`MemoryAvatar`）：键盘与 Task 6 的 App 都用，放 `Shared` 不重复写。
4. **另改 `KeyboardPanel.swift`**（加 `.scope`、`.contactCard` 两种面板）、**`KeyStyle.swift`**（提示行高度 34pt）、**`Engine.swift`**（`session`、`take`、`withOptionalCString` 放宽到模块内可见，`MemoryBridge.swift` 做成 `extension Engine` 要用）。
5. **选择面板的名单经 `qj_memory_read` 读**（App 的接口，按 App Group 路径读，只读不写）：大纲的键盘接口里没有列名单的函数，不为此新加 C 函数。
6. 选择面板加了「收起」：大纲没写怎么回到键区。选了对象、换到日常 / 工作时自动收起。
7. 「记一笔」还要求选了对象（确认条要写「记到 {对象}」），并且不在私密输入框里。
8. **审计修订（重要 4、6，建议「没开完全访问时禁止切换场景」）：**
   - **（2026-10-04 真机后改定，以此为准：提示行只在「恋爱 · 某人」且有提示或记一笔条时出现，没提示不占行，照设计稿；用户把常驻的空行当成 bug，宿主界面跳 34pt 的代价可以接受。下面这条作废。）** **提示行恋爱场景选了对象时一直保留**（没有提示时是空行），日常、工作与「恋爱 · 不指定」没有这一行；键盘高度与 0.2 秒动画只在进出「恋爱 · 某人」时发生（`KeyboardModel.hasHintRow`），提示出现消失不再让宿主界面跳 34pt。差异 1 里「跟着 `model.hint` 改高度」作废，改为跟着 `hasHintRow`。
   - **换输入框清最近上屏的字：** 控制器在 `textDidChange` 里比较 `textDocumentProxy.documentIdentifier`，变了就调 `model.hostChanged()` → `Engine.resetContext()` → 桥的 `qj_reset_context`；键盘收起时 `model.dismiss()` 里的 `flush` 由桥清。
   - **没开完全访问时不让切场景：** 桥不知道有没有完全访问，这道门在 Swift 侧：`ScopePicker` 在 `fullAccess` 为假时只显示「开启完全访问后才能使用记忆」与「收起」，没有场景分段。
   - `MemoryContact` 带 `hintOn` / `remindOn`（缺省开，旧文件兼容）；`MemoryScope` 只有场景与对象；`MemorySnapshot` 带 `revs`；`MemoryFailure` 多 `conflict`；`MemoryCard.daysAway` 日子按年重复、`reminderText` 按种类分模板（与桥一致）；`MemoryDate.nextAnniversary`。
   - **手写卡上限：** 新文件 `Shared/Memory/MemoryLimits.swift`（200 字、8 个关键词、每个 2–8 字，按 Unicode 标量计数，与 Rust 的 `chars()` 一致）；「记一笔」的剪贴板文字截到 200 字；`MemoryCard` 带上 `faded` / `seq` / `updatedAt`（与 proto 同语义，写回不丢）。
   - `Engine.memoryNote` 注释写清「桥返回 NULL 即成功」（Task 6 的单测 `testNoteNullMeansSuccess` 锁住这条约定）。
9. **落地时的修正（2026-10-04，模拟器里真编真跑）：**
   - 计划里的 Swift 代码按原文一次编过，Swift 6 严格并发下 `UIView.animate` 的 `[weak self]` 闭包不用退路写法，`switch` 表达式赋值、着色之外的部分都无需改动；`xcodebuild … test` 用 Xcode 自动生成的 scheme 即带测试 target，不需要 `schemes:`。
   - **真崩溃（计划原文有）：** `textDocumentProxy.documentIdentifier` 声明为非可选 `UUID`，键盘刚弹出、连上宿主之前系统返回 nil，`textDidChange` 里直接读会在 `UUID._unconditionallyBridgeFromObjectiveC` 处 EXC_BREAKPOINT（模拟器里完全访问开着的第一次弹出就崩）。改为 `hostDocumentIdentifier`：走 KVC 取成 `UUID?`，比较交给纯函数 `HostDocument.changed(from:to:)`。
   - 新增 `Shared/Memory/ScopeDisplay.swift`（提示行在不在、牌子文字、选择面板模式、`canNote`）、`HostDocument.swift`，`Tests/MemoryModelTests.swift`（21 个测试：上限、桥 JSON 解码含旧文件缺省、`qj_memory_note` 返回 NULL 即成功、`daysAway` 按年重复、闰日、提醒文案、`knownDays`、id 格式、提示行 / 牌子 / 面板 / `canNote`、换输入框），`KeyboardModel` 与两个视图调用它们。

### 步骤

**Files:**
- Create: `cloud/ios/Shared/Memory/{MemoryPronoun,MemoryContact,MemoryCard,MemoryScope,MemorySnapshot,MemoryHint,MemoryFailure,MemoryDate,MemoryID,MemoryFiles,MemoryAvatar,MemoryLimits,ScopeDisplay,HostDocument}.swift`
- Create: `cloud/ios/Tests/MemoryModelTests.swift`
- Create: `cloud/ios/Keyboard/Sources/{MemoryBridge,ScopeChip,ScopePicker,HintRow,ContactCardPanel}.swift`
- Modify: `cloud/ios/Keyboard/Sources/{Engine,KeyStyle,KeyboardPanel,KeyboardView,IdleBar,KeyboardModel,KeyboardViewController}.swift`

- [ ] **Step 1: 共用模型（Shared/Memory）**

Create `cloud/ios/Shared/Memory/MemoryPronoun.swift`：

```swift
// 提醒文案里怎么称呼对象，与桥的 Pronoun 一一对应（JSON 值 ta / ta_m / ta_f / name）。

enum MemoryPronoun: String, Codable, CaseIterable, Sendable {
    case ta
    case taM = "ta_m"
    case taF = "ta_f"
    case name

    /// 选称呼时的顺序（缺省 TA）。
    static let choices: [MemoryPronoun] = [.taM, .taF, .ta, .name]

    /// 文案里的称呼，与桥的 `Pronoun::label` 一致。
    func label(name: String) -> String {
        switch self {
        case .ta: "TA"
        case .taM: "他"
        case .taF: "她"
        case .name: name
        }
    }

    /// 选称呼时的选项名。
    var title: String {
        switch self {
        case .ta: "TA"
        case .taM: "他"
        case .taF: "她"
        case .name: "直接用名字"
        }
    }
}
```

Create `cloud/ios/Shared/Memory/MemoryContact.swift`：

```swift
// 记忆里的一个人（contacts.json 的一项）。名字只在这里，对象目录名用随机 id。两个提示开关按人设置，旧文件没有时按开。

import Foundation

struct MemoryContact: Codable, Identifiable, Hashable, Sendable {
    let id: String

    var name: String

    var pronoun: MemoryPronoun

    var scene: String

    let createdAt: Int64

    /// 打字时按这个人的卡片提示。
    var hintOn = true

    /// 这个人的日子与约定快到时提醒。
    var remindOn = true

    enum CodingKeys: String, CodingKey {
        case id, name, pronoun, scene
        case createdAt = "created_at"
        case hintOn = "hint_on"
        case remindOn = "remind_on"
    }

    /// 恋爱场景的新对象。
    static func new(name: String, pronoun: MemoryPronoun) -> MemoryContact {
        MemoryContact(
            id: MemoryID.make(), name: name, pronoun: pronoun, scene: MemoryScope.dating,
            createdAt: Int64(Date().timeIntervalSince1970))
    }

    /// 认识了几天：按北京时间的日历日，建的那天算第 1 天。
    func knownDays(now: Date = Date()) -> Int {
        MemoryDate.daysBetween(Date(timeIntervalSince1970: TimeInterval(createdAt)), now) + 1
    }
}

extension MemoryContact {
    /// 缺 `hint_on` / `remind_on` 的旧文件按开（init 写在扩展里，成员逐一构造器才留得住）。
    init(from decoder: any Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        self.init(
            id: try container.decode(String.self, forKey: .id),
            name: try container.decode(String.self, forKey: .name),
            pronoun: try container.decodeIfPresent(MemoryPronoun.self, forKey: .pronoun) ?? .ta,
            scene: try container.decode(String.self, forKey: .scene),
            createdAt: try container.decode(Int64.self, forKey: .createdAt),
            hintOn: try container.decodeIfPresent(Bool.self, forKey: .hintOn) ?? true,
            remindOn: try container.decodeIfPresent(Bool.self, forKey: .remindOn) ?? true)
    }
}
```

Create `cloud/ios/Shared/Memory/MemoryCard.swift`：

```swift
// 一张记忆卡（<对象 id>/cards.json 的一项），与桥的 Card 一一对应。文字最多 200 字、关键词最多 8 个（见 MemoryLimits）。
// faded / seq / updatedAt 是 2C 云端卡的字段，手写卡恒为 false / 0 / 0，读进来原样写回，不丢。

import Foundation

struct MemoryCard: Codable, Identifiable, Hashable, Sendable {
    let id: String

    var kind: Kind

    var text: String

    var keywords: [String]

    /// `yyyy-MM-dd`（北京时间），只对日子与约定有意义。
    var when: String?

    var source: String

    var confirmed: Bool

    /// 已淡出（过期不再提示）。
    var faded = false

    /// 服务端序号，每个用户单独递增。
    var seq: Int64 = 0

    /// 服务端记的最后修改时间，Unix 毫秒。
    var updatedAt: Int64 = 0

    /// 建卡时间，Unix 秒。
    let createdAt: Int64

    /// 本地最后一次改的时间，Unix 秒；写回冲突时两边都改了以它新者为准。
    var touchedAt: Int64

    enum CodingKeys: String, CodingKey {
        case id, kind, text, keywords, when, source, confirmed, faded, seq
        case updatedAt = "updated_at"
        case createdAt = "created_at"
        case touchedAt = "touched_at"
    }

    enum Kind: String, Codable, CaseIterable, Sendable {
        case date, promise, preference, recent, other

        var title: String {
            switch self {
            case .date: "日子"
            case .promise: "约定"
            case .preference: "喜好"
            case .recent: "近况"
            case .other: "其他"
            }
        }

        var hasDate: Bool { self == .date || self == .promise }
    }

    /// 手写的新卡。
    static func new(kind: Kind, text: String, when: String?, keywords: [String]) -> MemoryCard {
        let now = Int64(Date().timeIntervalSince1970)
        return MemoryCard(
            id: MemoryID.make(), kind: kind, text: text, keywords: keywords, when: when,
            source: "manual", confirmed: true, createdAt: now, touchedAt: now)
    }

    /// 离今天还有几天：日子按年重复（今年那天过了看明年），约定按写的那天；别的种类、日期写错为 nil。与桥的 `days_away` 一致。
    func daysAway(now: Date = Date()) -> Int? {
        guard let when, let date = MemoryDate.parse(when) else { return nil }
        switch kind {
        case .date: return MemoryDate.daysBetween(now, MemoryDate.nextAnniversary(of: date, from: now))
        case .promise: return MemoryDate.daysBetween(now, date)
        default: return nil
        }
    }

    /// 对象卡左列：日子 / 约定 3 天内写相对（今天、明天、周几），更远的写 `M.dd`；别的种类没有日期，为 nil。
    func dateLabel(now: Date = Date()) -> String? {
        guard let days = daysAway(now: now), let when, let date = MemoryDate.parse(when) else { return nil }
        let target = kind == .date ? MemoryDate.nextAnniversary(of: date, from: now) : date
        switch days {
        case 0: return "今天"
        case 1: return "明天"
        case 2...3:
            let names = ["周日", "周一", "周二", "周三", "周四", "周五", "周六"]
            return names[MemoryDate.calendar.component(.weekday, from: target) - 1]
        default:
            let parts = MemoryDate.calendar.dateComponents([.month, .day], from: target)
            return String(format: "%d.%02d", parts.month ?? 0, parts.day ?? 0)
        }
    }

    /// 对象卡标题下的小字：关键词（跟标题一样的不重复写），没有就留空。
    var subtitle: String {
        keywords.filter { $0 != text }.joined(separator: " · ")
    }

    /// 日子「明天是她的生日」，约定「明天：看电影」：与桥的 `reminder_text` 同一模板（扩展之前的最后一个方法）。
    func reminderText(days: Int, contact: MemoryContact) -> String {
        let when = switch days {
        case 0: "今天"
        case 1: "明天"
        default: "\(days) 天后"
        }
        if kind == .promise { return "\(when)：\(text)" }
        return "\(when)是\(contact.pronoun.label(name: contact.name))的\(text)"
    }
}

extension MemoryCard {
    /// 缺字段的旧文件按缺省（init 写在扩展里，成员逐一构造器才留得住）。
    init(from decoder: any Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        self.init(
            id: try container.decode(String.self, forKey: .id),
            kind: try container.decode(Kind.self, forKey: .kind),
            text: try container.decode(String.self, forKey: .text),
            keywords: try container.decodeIfPresent([String].self, forKey: .keywords) ?? [],
            when: try container.decodeIfPresent(String.self, forKey: .when),
            source: try container.decodeIfPresent(String.self, forKey: .source) ?? "manual",
            confirmed: try container.decodeIfPresent(Bool.self, forKey: .confirmed) ?? false,
            faded: try container.decodeIfPresent(Bool.self, forKey: .faded) ?? false,
            seq: try container.decodeIfPresent(Int64.self, forKey: .seq) ?? 0,
            updatedAt: try container.decodeIfPresent(Int64.self, forKey: .updatedAt) ?? 0,
            createdAt: try container.decode(Int64.self, forKey: .createdAt),
            touchedAt: try container.decode(Int64.self, forKey: .touchedAt))
    }
}
```

Create `cloud/ios/Shared/Memory/MemoryScope.swift`：

```swift
// 键盘当前的场景与对象（memory/state.json；qj_scope_get 给的就是这两项）。提示开关在各个对象上（MemoryContact）。

struct MemoryScope: Codable, Equatable, Sendable {
    static let daily = "daily"

    static let dating = "dating"

    static let work = "work"

    var scene = MemoryScope.daily

    var contactId: String?

    enum CodingKeys: String, CodingKey {
        case scene
        case contactId = "contact_id"
    }

    /// 场景的中文名。
    static func title(of scene: String) -> String {
        switch scene {
        case dating: "恋爱"
        case work: "工作"
        default: "日常"
        }
    }
}

extension MemoryScope {
    /// 缺的字段按缺省（init 写在扩展里，成员逐一构造器才留得住）。
    init(from decoder: any Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        self.init()
        scene = try container.decodeIfPresent(String.self, forKey: .scene) ?? MemoryScope.daily
        contactId = try container.decodeIfPresent(String.self, forKey: .contactId)
    }
}
```

Create `cloud/ios/Shared/Memory/MemorySnapshot.swift`：

```swift
// App 整份读写的记忆数据：{"contacts","cards","revs","state","broken"}，与桥的 MemorySnapshot 一一对应。
// revs 是读时各对象卡片的修订号，原样带回去写；桥据此发现键盘这期间「记一笔」改过（返回 conflict）。

struct MemorySnapshot: Codable, Equatable, Sendable {
    var contacts: [MemoryContact] = []

    /// 对象 id → 卡片。
    var cards: [String: [MemoryCard]] = [:]

    /// 对象 id → 读时的修订号。
    var revs: [String: UInt64] = [:]

    /// 键盘当前的场景与对象，只给显示；写回时桥不看。
    var state = MemoryScope()

    /// 这次读时卡片文件坏了、已备份的对象；写回时桥不看。
    var broken: [String] = []

    enum CodingKeys: String, CodingKey {
        case contacts, cards, revs, state, broken
    }
}

extension MemorySnapshot {
    init(from decoder: any Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        self.init()
        contacts = try container.decodeIfPresent([MemoryContact].self, forKey: .contacts) ?? []
        cards = try container.decodeIfPresent([String: [MemoryCard]].self, forKey: .cards) ?? [:]
        revs = try container.decodeIfPresent([String: UInt64].self, forKey: .revs) ?? [:]
        state = try container.decodeIfPresent(MemoryScope.self, forKey: .state) ?? MemoryScope()
        broken = try container.decodeIfPresent([String].self, forKey: .broken) ?? []
    }
}
```

Create `cloud/ios/Shared/Memory/MemoryHint.swift`：

```swift
// 提示行的一条：qj_memory_hint 的 JSON。

struct MemoryHint: Decodable, Equatable, Sendable {
    let cardId: String

    let text: String

    let reason: Reason

    /// 对象还有别的卡（「展开」看得到）。
    let more: Bool

    enum Reason: String, Decodable, Sendable {
        case match, today
    }

    enum CodingKeys: String, CodingKey {
        case text, reason, more
        case cardId = "card_id"
    }
}
```

Create `cloud/ios/Shared/Memory/MemoryFailure.swift`：

```swift
// 桥写记忆失败时返回的 JSON：{"code": "...", "message": "..."}。code 给界面分支，message 是给用户看的中文。

import Foundation

struct MemoryFailure: Equatable, Sendable {
    let code: Code

    let message: String

    /// 与桥的 `MemoryError::code` 一一对应；认不得的值按 other。
    enum Code: String, Decodable, Sendable {
        case contactLimit = "contact_limit"
        case invalid
        case conflict
        case io
        case other

        init(from decoder: any Decoder) throws {
            let raw = try decoder.singleValueContainer().decode(String.self)
            self = Code(rawValue: raw) ?? .other
        }
    }

    /// 给用户看的话：人数上限用固定文案，其余照桥给的。
    var userMessage: String { code == .contactLimit ? "恋爱场景最多 8 个人" : message }

    /// nil 表示成功；解析不了时整段当 message、code 为 other。
    static func decode(_ json: String?) -> MemoryFailure? {
        guard let json else { return nil }
        struct Wire: Decodable {
            let code: Code
            let message: String
        }
        if let data = json.data(using: .utf8), let wire = try? JSONDecoder().decode(Wire.self, from: data) {
            return MemoryFailure(code: wire.code, message: wire.message)
        }
        return MemoryFailure(code: .other, message: json)
    }
}
```

Create `cloud/ios/Shared/Memory/MemoryDate.swift`：

```swift
// 记忆里的日期：yyyy-MM-dd，按北京时间（与桥的 LocalDate 一致）。

import Foundation

enum MemoryDate {
    static let timeZone = TimeZone(identifier: "Asia/Shanghai") ?? TimeZone(secondsFromGMT: 8 * 3600)!

    static var calendar: Calendar {
        var calendar = Calendar(identifier: .gregorian)
        calendar.timeZone = timeZone
        return calendar
    }

    static func parse(_ text: String) -> Date? { formatter().date(from: text) }

    static func format(_ date: Date) -> String { formatter().string(from: date) }

    /// 两个时刻之间隔了几个日历日（北京时间）。
    static func daysBetween(_ from: Date, _ to: Date) -> Int {
        calendar.dateComponents([.day], from: calendar.startOfDay(for: from), to: calendar.startOfDay(for: to))
            .day ?? 0
    }

    /// 按年重复的日子：`date` 的月日落在 `now` 当天或之后最近的一次；2 月 29 日在平年算 2 月 28 日（与桥的 `next_anniversary` 一致）。
    static func nextAnniversary(of date: Date, from now: Date) -> Date {
        let cal = calendar
        let parts = cal.dateComponents([.month, .day], from: date)
        let today = cal.startOfDay(for: now)
        let year = cal.component(.year, from: today)
        func occurrence(_ year: Int) -> Date {
            var target = DateComponents(year: year, month: parts.month, day: 1)
            let first = cal.date(from: target) ?? today
            let days = cal.range(of: .day, in: .month, for: first)?.count ?? 28
            target.day = min(parts.day ?? 1, days)
            return cal.date(from: target) ?? today
        }
        let thisYear = occurrence(year)
        return thisYear >= today ? thisYear : occurrence(year + 1)
    }

    /// 从今天到 `text` 那天还有几天，过去的是负数；日期写错时为 nil。
    static func daysUntil(_ text: String, now: Date = Date()) -> Int? {
        parse(text).map { daysBetween(now, $0) }
    }

    /// DateFormatter 不是 Sendable，每次现建。
    private static func formatter() -> DateFormatter {
        let formatter = DateFormatter()
        formatter.locale = Locale(identifier: "en_US_POSIX")
        formatter.timeZone = timeZone
        formatter.dateFormat = "yyyy-MM-dd"
        return formatter
    }
}
```

Create `cloud/ios/Shared/Memory/MemoryID.swift`：

```swift
// 新对象、新卡片的 id：16 字节随机数的小写十六进制，与桥的 new_id 同格式（桥写入时会校验）。

import Foundation
import Security

enum MemoryID {
    static func make() -> String {
        var bytes = [UInt8](repeating: 0, count: 16)
        if SecRandomCopyBytes(kSecRandomDefault, bytes.count, &bytes) != errSecSuccess {
            bytes = withUnsafeBytes(of: UUID().uuid) { Array($0) }
        }
        return bytes.map { String(format: "%02x", $0) }.joined()
    }
}
```

Create `cloud/ios/Shared/Memory/MemoryFiles.swift`：

```swift
// 经桥整份读写 memory/（qj_memory_read / qj_memory_write）。App 与键盘共用；键盘只读名单，写只在 App 里。

import Foundation
import QingjianBridge

enum MemoryFiles {
    /// 读 `userDirectory/memory/` 的全部数据；参数无效或解析不了时为 nil。
    static func read(userDirectory: URL) -> MemorySnapshot? {
        decode(take(userDirectory.path.withCString { qj_memory_read($0) }))
    }

    /// 整份写回；成功返回 nil。
    static func write(_ snapshot: MemorySnapshot, userDirectory: URL) -> MemoryFailure? {
        guard let data = try? JSONEncoder().encode(snapshot),
              let json = String(data: data, encoding: .utf8)
        else { return MemoryFailure(code: .invalid, message: "数据编码失败") }
        let raw = userDirectory.path.withCString { dir in json.withCString { qj_memory_write(dir, $0) } }
        return MemoryFailure.decode(take(raw))
    }

    /// 取走桥返回的字符串并释放。
    static func take(_ raw: UnsafeMutablePointer<CChar>?) -> String? {
        guard let raw else { return nil }
        defer { qj_string_free(raw) }
        return String(cString: raw)
    }

    static func decode<T: Decodable>(_ json: String?) -> T? {
        guard let data = json?.data(using: .utf8) else { return nil }
        return try? JSONDecoder().decode(T.self, from: data)
    }
}
```

Create `cloud/ios/Shared/Memory/MemoryAvatar.swift`：

```swift
// 头像字：名字的第一个字放在圆里，选中时底色换成强调色（App 的列表、详情与键盘的对象卡、选择面板共用）。

import SwiftUI

struct MemoryAvatar: View {
    let name: String

    var size: CGFloat = 40

    var selected = false

    var body: some View {
        Text(name.first.map(String.init) ?? "?")
            .font(.system(size: size * 0.45, weight: .semibold))
            .foregroundStyle(Theme.accentInk.color)
            .frame(width: size, height: size)
            .background(Circle().fill(selected ? Theme.accent.color : Theme.accentSoft.color))
            .accessibilityHidden(true)
    }
}
```

Create `cloud/ios/Shared/Memory/MemoryLimits.swift`：

```swift
// 手写卡的上限，与桥（proto 的 MAX_CARD_TEXT_CHARS / MAX_CARD_KEYWORDS）一致：文字最多 200 字、关键词最多 8 个、每个 2–8 字。
// 「字」按 Unicode 标量数，与 Rust 的 chars().count() 一样（中文、单个 emoji 各算一个）；桥写入前还会再校验一遍。

enum MemoryLimits {
    static let maxTextChars = 200

    static let maxKeywords = 8

    static let keywordChars = 2...8

    /// 文字的字数（Unicode 标量数）。
    static func count(_ text: String) -> Int { text.unicodeScalars.count }

    /// 超过 200 字就截掉后面的。
    static func clampText(_ text: String) -> String {
        guard count(text) > maxTextChars else { return text }
        return String(String.UnicodeScalarView(text.unicodeScalars.prefix(maxTextChars)))
    }

    /// 编辑器里的计数，如「137 / 200」。
    static func counter(_ text: String) -> String { "\(count(text)) / \(maxTextChars)" }

    /// 这个关键词能不能加：去掉首尾空白后 2–8 字、还没满 8 个、没重复。
    static func canAdd(_ keyword: String, to keywords: [String]) -> Bool {
        let trimmed = keyword.trimmingCharacters(in: .whitespacesAndNewlines)
        return keywords.count < maxKeywords && keywordChars.contains(count(trimmed)) && !keywords.contains(trimmed)
    }
}
```

Create `cloud/ios/Shared/Memory/ScopeDisplay.swift`（落地时新增：提示行在不在、牌子写什么、选择面板能不能用、「记一笔」出不出，提成纯函数方便单测）：

```swift
// 键盘上与记忆有关的显示判断，提成纯函数方便单测：提示行在不在、牌子写什么、选择面板能不能用、「记一笔」出不出。

enum ScopeDisplay {
    /// 恋爱场景选了对象时提示行一直在（没有提示时是空行）；日常、工作与「恋爱 · 不指定」没有这一行。
    static func hasHintRow(scene: String, hasContact: Bool) -> Bool {
        scene == MemoryScope.dating && hasContact
    }

    /// 牌子上的字：恋爱选了对象是「小美 · 恋爱」，其余只是场景名。
    static func chipTitle(scene: String, contactName: String?) -> String {
        guard scene == MemoryScope.dating, let contactName else { return MemoryScope.title(of: scene) }
        return "\(contactName) · 恋爱"
    }

    enum PickerMode: Equatable {
        case picker
        case needsFullAccess
    }

    /// 没开完全访问读不到 App Group 里的记忆，也不让切场景（桥不知道有没有完全访问，这道门在 Swift 侧）。
    static func pickerMode(fullAccess: Bool) -> PickerMode { fullAccess ? .picker : .needsFullAccess }

    static let needsFullAccessText = "开启完全访问后才能使用记忆"

    /// 「记一笔」：开了完全访问、剪贴板有字、选了对象、不在私密输入框。
    static func canNote(fullAccess: Bool, clipboardHasText: Bool, privateField: Bool, hasContact: Bool) -> Bool {
        fullAccess && clipboardHasText && !privateField && hasContact
    }

    /// 恋爱场景最多几个对象（与桥的上限一致）。
    static let maxContacts = 8

    static func contactSubtitle(knownDays: Int) -> String { "认识 \(knownDays) 天" }

    static func newContactSubtitle(count: Int) -> String { "\(count) / \(maxContacts)" }

    static let noScopeSubtitle = "只用场景"

    /// 对象卡页脚左边：面板只列与今天有关的卡。
    static func cardFooter(count: Int) -> String { "只显示与今天有关的 \(count) 条" }

    /// 键盘扩展没有官方办法打开容器 App，点了只给提示。
    static let allMemoryNotice = "在素笺 App 里查看全部记忆"

    static let enableFullAccessNotice = "在素笺 App 里按引导开启完全访问"
}
```

Create `cloud/ios/Shared/Memory/HostDocument.swift`（落地时新增：换输入框的判断）：

```swift
// 宿主输入框有没有换：textDocumentProxy.documentIdentifier 每个输入框一个，我们自己上屏不会改它。

import Foundation

enum HostDocument {
    /// 变了（包括第一次看到）就该让桥清掉最近上屏的字，免得在 A 聊天里打的字在 B 里触发记忆提示。
    static func changed(from last: UUID?, to current: UUID?) -> Bool { last != current }
}
```

Create `cloud/ios/Tests/MemoryModelTests.swift`（落地时新增，先红后绿；Task 6 的 `MemoryStoreTests` 另写；`QingjianCloudTests` 依赖 App target，`Shared/Memory` 的类型直接 `@testable import`）：

```swift
// 键盘与 App 共用的记忆模型：桥 JSON 解码、上限、日子换算、提示行与牌子的显示判断、换输入框判断。

import XCTest
@testable import QingjianCloud

final class MemoryModelTests: XCTestCase {
    private func contact(_ name: String = "小美") -> MemoryContact {
        MemoryContact.new(name: name, pronoun: .taF)
    }

    private func date(_ text: String) throws -> Date { try XCTUnwrap(MemoryDate.parse(text)) }

    // MARK: 上限

    func testLimitsCountUnicodeScalars() {
        XCTAssertEqual(MemoryLimits.maxTextChars, 200)
        XCTAssertEqual(MemoryLimits.maxKeywords, 8)
        XCTAssertEqual(MemoryLimits.count("不吃香菜"), 4)
        XCTAssertEqual(MemoryLimits.count("🎂"), 1)
    }

    func testClampTextCutsAt200Scalars() {
        let long = String(repeating: "字", count: 250)
        XCTAssertEqual(MemoryLimits.count(MemoryLimits.clampText(long)), 200)
        let short = "不吃香菜"
        XCTAssertEqual(MemoryLimits.clampText(short), short)
        XCTAssertEqual(MemoryLimits.counter("不吃香菜"), "4 / 200")
    }

    func testKeywordRules() {
        XCTAssertTrue(MemoryLimits.canAdd("蛋糕", to: []))
        XCTAssertTrue(MemoryLimits.canAdd(" 蛋糕 ", to: []), "首尾空白不计字数")
        XCTAssertFalse(MemoryLimits.canAdd("糕", to: []), "少于 2 字")
        XCTAssertTrue(MemoryLimits.canAdd("草莓味的奶油蛋糕", to: []), "8 字可以")
        XCTAssertFalse(MemoryLimits.canAdd("草莓味的奶油蛋糕卷", to: []), "多于 8 字")
        XCTAssertFalse(MemoryLimits.canAdd("蛋糕", to: ["蛋糕"]), "重复")
        let full = (0..<8).map { "关键词\($0)" }
        XCTAssertFalse(MemoryLimits.canAdd("新词语", to: full), "已满 8 个")
    }

    // MARK: 桥的 JSON

    func testContactDecodesWithDefaultsForOldFiles() throws {
        let json = #"{"id":"0123456789abcdef0123456789abcdef","name":"小美","pronoun":"ta_f","scene":"dating","created_at":100}"#
        let contact = try JSONDecoder().decode(MemoryContact.self, from: Data(json.utf8))
        XCTAssertEqual(contact.name, "小美")
        XCTAssertEqual(contact.pronoun, .taF)
        XCTAssertTrue(contact.hintOn)
        XCTAssertTrue(contact.remindOn)
        let off = #"{"id":"a","name":"b","scene":"dating","created_at":1,"hint_on":false,"remind_on":false}"#
        let decoded = try JSONDecoder().decode(MemoryContact.self, from: Data(off.utf8))
        XCTAssertFalse(decoded.hintOn)
        XCTAssertFalse(decoded.remindOn)
        XCTAssertEqual(decoded.pronoun, .ta, "缺称呼按 TA")
    }

    func testCardKeepsCloudFieldsOnRoundTrip() throws {
        let json = #"{"id":"a","kind":"date","text":"生日","keywords":[],"when":"1998-05-20","source":"cloud","confirmed":true,"faded":true,"seq":7,"updated_at":1700000000000,"created_at":1,"touched_at":2}"#
        let card = try JSONDecoder().decode(MemoryCard.self, from: Data(json.utf8))
        XCTAssertTrue(card.faded)
        XCTAssertEqual(card.seq, 7)
        XCTAssertEqual(card.updatedAt, 1_700_000_000_000)
        let back = try JSONDecoder().decode(MemoryCard.self, from: JSONEncoder().encode(card))
        XCTAssertEqual(back, card)
    }

    func testCardDecodesWithoutOptionalFields() throws {
        let json = #"{"id":"a","kind":"preference","text":"不吃香菜","created_at":1,"touched_at":2}"#
        let card = try JSONDecoder().decode(MemoryCard.self, from: Data(json.utf8))
        XCTAssertEqual(card.keywords, [])
        XCTAssertNil(card.when)
        XCTAssertEqual(card.source, "manual")
        XCTAssertFalse(card.faded)
        XCTAssertEqual(card.seq, 0)
    }

    func testSnapshotDecodesFromBridge() throws {
        let json = """
        {"contacts":[{"id":"a","name":"小美","pronoun":"ta_f","scene":"dating","created_at":1}],
         "cards":{"a":[{"id":"c","kind":"other","text":"x","created_at":1,"touched_at":1}]},
         "revs":{"a":3},"state":{"scene":"dating","contact_id":"a"},"broken":["b"]}
        """
        let snapshot = try JSONDecoder().decode(MemorySnapshot.self, from: Data(json.utf8))
        XCTAssertEqual(snapshot.contacts.count, 1)
        XCTAssertEqual(snapshot.cards["a"]?.count, 1)
        XCTAssertEqual(snapshot.revs["a"], 3)
        XCTAssertEqual(snapshot.state, MemoryScope(scene: "dating", contactId: "a"))
        XCTAssertEqual(snapshot.broken, ["b"])
        let empty = try JSONDecoder().decode(MemorySnapshot.self, from: Data("{}".utf8))
        XCTAssertEqual(empty, MemorySnapshot())
    }

    func testScopeDecodes() throws {
        let scope = try JSONDecoder().decode(MemoryScope.self, from: Data(#"{"scene":"work","contact_id":null}"#.utf8))
        XCTAssertEqual(scope, MemoryScope(scene: "work", contactId: nil))
        XCTAssertEqual(MemoryScope.title(of: "dating"), "恋爱")
        XCTAssertEqual(MemoryScope.title(of: "work"), "工作")
        XCTAssertEqual(MemoryScope.title(of: "daily"), "日常")
    }

    func testHintDecodes() throws {
        let json = #"{"card_id":"c","text":"明天是她的生日","reason":"today","more":true}"#
        let hint = try JSONDecoder().decode(MemoryHint.self, from: Data(json.utf8))
        XCTAssertEqual(hint, MemoryHint(cardId: "c", text: "明天是她的生日", reason: .today, more: true))
    }

    // MARK: qj_memory_note 的返回值：NULL 就是成功

    func testNoteNullMeansSuccess() {
        XCTAssertNil(MemoryFailure.decode(nil), "桥返回 NULL（含已接受、稍后写入）：成功")
    }

    func testNoteFailureDecodes() {
        let failure = MemoryFailure.decode(#"{"code":"io","message":"读不了"}"#)
        XCTAssertEqual(failure, MemoryFailure(code: .io, message: "读不了"))
        XCTAssertEqual(MemoryFailure.decode(#"{"code":"contact_limit","message":"x"}"#)?.userMessage, "恋爱场景最多 8 个人")
        XCTAssertEqual(MemoryFailure.decode(#"{"code":"zzz","message":"m"}"#)?.code, .other)
        XCTAssertEqual(MemoryFailure.decode("not json")?.message, "not json")
    }

    // MARK: 日子

    func testDaysAwayRepeatsDatesYearlyAndPromisesOnce() throws {
        let now = try date("2026-10-04")
        var card = MemoryCard.new(kind: .date, text: "生日", when: "1998-10-05", keywords: [])
        XCTAssertEqual(card.daysAway(now: now), 1)
        card.when = "1998-10-04"
        XCTAssertEqual(card.daysAway(now: now), 0)
        card.when = "1998-10-03"
        XCTAssertEqual(card.daysAway(now: now), 364, "今年那天过了看明年")
        card.kind = .promise
        card.when = "2026-10-03"
        XCTAssertEqual(card.daysAway(now: now), -1, "约定不重复")
        card.kind = .preference
        XCTAssertNil(card.daysAway(now: now))
    }

    func testLeapDayFallsOnFeb28InCommonYears() throws {
        let next = MemoryDate.nextAnniversary(of: try date("2000-02-29"), from: try date("2026-10-04"))
        XCTAssertEqual(MemoryDate.format(next), "2027-02-28")
        let leap = MemoryDate.nextAnniversary(of: try date("2000-02-29"), from: try date("2027-10-04"))
        XCTAssertEqual(MemoryDate.format(leap), "2028-02-29")
    }

    func testReminderTextMatchesBridgeTemplates() {
        let person = contact()
        let birthday = MemoryCard.new(kind: .date, text: "生日", when: "1998-10-05", keywords: [])
        XCTAssertEqual(birthday.reminderText(days: 1, contact: person), "明天是她的生日")
        XCTAssertEqual(birthday.reminderText(days: 0, contact: person), "今天是她的生日")
        XCTAssertEqual(birthday.reminderText(days: 3, contact: person), "3 天后是她的生日")
        let promise = MemoryCard.new(kind: .promise, text: "看电影", when: "2026-10-05", keywords: [])
        XCTAssertEqual(promise.reminderText(days: 1, contact: person), "明天：看电影")
    }

    func testKnownDaysCountsCreationDayAsFirst() {
        var person = contact()
        XCTAssertEqual(person.knownDays(), 1)
        person = MemoryContact(
            id: "a", name: "小美", pronoun: .ta, scene: "dating",
            createdAt: Int64(Date().timeIntervalSince1970) - 12 * 86400)
        XCTAssertEqual(person.knownDays(), 13)
    }

    func testIDFormatIs32LowercaseHex() {
        let id = MemoryID.make()
        XCTAssertEqual(id.count, 32)
        XCTAssertTrue(id.allSatisfy { $0.isHexDigit && !$0.isUppercase })
        XCTAssertNotEqual(id, MemoryID.make())
    }

    // MARK: 提示行、牌子、面板

    func testHintRowOnlyForDatingWithContact() {
        XCTAssertTrue(ScopeDisplay.hasHintRow(scene: "dating", hasContact: true))
        XCTAssertFalse(ScopeDisplay.hasHintRow(scene: "dating", hasContact: false), "恋爱不指定没有提示行")
        XCTAssertFalse(ScopeDisplay.hasHintRow(scene: "daily", hasContact: true))
        XCTAssertFalse(ScopeDisplay.hasHintRow(scene: "work", hasContact: true))
    }

    func testChipTitle() {
        XCTAssertEqual(ScopeDisplay.chipTitle(scene: "dating", contactName: "小美"), "小美 · 恋爱")
        XCTAssertEqual(ScopeDisplay.chipTitle(scene: "dating", contactName: nil), "恋爱")
        XCTAssertEqual(ScopeDisplay.chipTitle(scene: "work", contactName: "小美"), "工作")
        XCTAssertEqual(ScopeDisplay.chipTitle(scene: "daily", contactName: nil), "日常")
    }

    func testPickerNeedsFullAccess() {
        XCTAssertEqual(ScopeDisplay.pickerMode(fullAccess: false), .needsFullAccess)
        XCTAssertEqual(ScopeDisplay.pickerMode(fullAccess: true), .picker)
        XCTAssertEqual(ScopeDisplay.needsFullAccessText, "开启完全访问后才能使用记忆")
    }

    func testCanNoteNeedsEverything() {
        XCTAssertTrue(ScopeDisplay.canNote(fullAccess: true, clipboardHasText: true, privateField: false, hasContact: true))
        XCTAssertFalse(ScopeDisplay.canNote(fullAccess: false, clipboardHasText: true, privateField: false, hasContact: true))
        XCTAssertFalse(ScopeDisplay.canNote(fullAccess: true, clipboardHasText: false, privateField: false, hasContact: true))
        XCTAssertFalse(ScopeDisplay.canNote(fullAccess: true, clipboardHasText: true, privateField: true, hasContact: true))
        XCTAssertFalse(ScopeDisplay.canNote(fullAccess: true, clipboardHasText: true, privateField: false, hasContact: false))
    }

    // MARK: 换输入框

    func testHostChangedComparesDocumentIdentifier() {
        let a = UUID()
        let b = UUID()
        XCTAssertFalse(HostDocument.changed(from: a, to: a))
        XCTAssertTrue(HostDocument.changed(from: a, to: b))
        XCTAssertTrue(HostDocument.changed(from: nil, to: a), "第一次看到输入框也清一次")
        XCTAssertFalse(HostDocument.changed(from: nil, to: nil))
    }
}
```

- [ ] **Step 2: `Engine` 放宽可见性，加 `MemoryBridge.swift`**

Modify `cloud/ios/Keyboard/Sources/Engine.swift`：
- 第 8–9 行换成

```swift
    /// deinit 不在主线程隔离里，要绕开 Sendable 检查；释放时已没有别的引用。MemoryBridge.swift 的扩展也用它。
    nonisolated(unsafe) let session: OpaquePointer
```

- 第 119 行 `private func take(` 改成 `func take(`；第 125 行 `private static func withOptionalCString<T>(` 改成 `static func withOptionalCString<T>(`。

Create `cloud/ios/Keyboard/Sources/MemoryBridge.swift`：

```swift
// 调桥的本地记忆接口（qj_scope_* / qj_memory_*）：Engine 的扩展，用同一个会话指针，只在主线程上用。

import Foundation
import QingjianBridge

extension Engine {
    /// 当前场景与对象；会话没有记忆目录时为 nil。
    var scope: MemoryScope? { MemoryFiles.decode(take(qj_scope_get(session))) }

    func setScope(scene: String, contactId: String?) {
        scene.withCString { s in
            Self.withOptionalCString(contactId) { qj_scope_set(session, s, $0) }
        }
    }

    /// 提示行要显示的；私密输入、非恋爱场景、没选对象时为 nil。
    var memoryHint: MemoryHint? { MemoryFiles.decode(take(qj_memory_hint(session))) }

    /// 「知道了」：today 为真当天不再出，为假 10 分钟内不再出。
    func dismissHint(_ cardId: String, today: Bool) {
        cardId.withCString { qj_memory_dismiss(session, $0, today) }
    }

    /// 对象卡面板：今日相关最多 3 张。
    func memoryCards(_ contactId: String) -> [MemoryCard] {
        let raw = contactId.withCString { qj_memory_cards(session, $0) }
        return MemoryFiles.decode(take(raw)) ?? []
    }

    /// 宿主换了输入框：桥清掉最近上屏的字，免得 A 聊天里打的字在 B 里触发提示（键盘收起时 flush 也会清）。
    func resetContext() { qj_reset_context(session) }

    /// 「记一笔」。桥返回 NULL 表示成功（这里得到 nil），失败才返回 `{"code","message"}`：
    /// invalid 是没有这个人或没有文字，io 是卡片读不了（锁屏），此时桥什么都没写。
    func memoryNote(_ contactId: String, text: String) -> MemoryFailure? {
        let raw = contactId.withCString { c in text.withCString { qj_memory_note(session, c, $0) } }
        return MemoryFailure.decode(take(raw))
    }
}
```

- [ ] **Step 3: 尺寸与面板种类**

Modify `cloud/ios/Keyboard/Sources/KeyStyle.swift`：第 17 行 `static let candidateBarHeight: CGFloat = 50` 之后加：

```swift

    /// 候选栏上方记忆提示行的高度。
    static let hintRowHeight: CGFloat = 34
```

Replace `cloud/ios/Keyboard/Sources/KeyboardPanel.swift`：

```swift
// 候选栏下面那块区域现在显示什么。

enum KeyboardPanel: Hashable {
    case keys

    /// 点候选栏右端 ⌄ 展开的全部候选。
    case candidates

    case emoji

    /// 点场景牌子打开的场景 / 对象选择。
    case scope

    /// 提示行「展开」打开的对象卡。
    case contactCard
}
```

- [ ] **Step 4: 键盘模型**

Modify `cloud/ios/Keyboard/Sources/KeyboardModel.swift`：

(a) 文件头第 4 行之后加一行：

```swift
// 本地记忆：场景牌子与选择面板、候选栏上方的提示行、对象卡、「记一笔」都经 MemoryBridge 调桥；名单读 App Group 里的 memory/。
```

(b) 第 25 行 `private(set) var privateField = false` 之后插入：

```swift

    /// 记忆的提示行：恋爱场景、选了对象、碰上卡片里的词或日子快到时有。
    private(set) var hint: MemoryHint?

    /// 当前场景与对象（只能用户自己切）。
    private(set) var scope = MemoryScope()

    /// App 里建的恋爱场景的人；开了完全访问才读得到 App Group。
    private(set) var contacts: [MemoryContact] = []

    /// 「记一笔」确认条里的剪贴板文字；nil 时不显示。
    private(set) var noteDraft: String?

    /// 对象卡面板里的卡片。
    private(set) var panelCards: [MemoryCard] = []

    /// 本机剪贴板里有没有文字（键盘出现时看一次；只看不读，不弹粘贴授权）。
    private(set) var clipboardHasText = false

    /// 开了完全访问：读 App Group 里的名单、「记一笔」读剪贴板都要它。控制器每次出现时设。
    var fullAccess = false
```

(c) `replaceEngine`（第 56–65 行）换成：

```swift
    func replaceEngine(_ engine: Engine?) {
        self.engine?.flush()
        self.engine = engine
        if privateField { engine?.setPrivate(true) }
        let shown = EngineDisplay.afterReplace(hasEngine: engine != nil, preedit: preedit, candidates: candidates)
        if shown.preedit != preedit { output?.setMarked(shown.preedit) }
        preedit = shown.preedit
        candidates = shown.candidates
        refresh()
        syncScope()
    }
```

(d) `dismiss()`（第 161–167 行）换成：

```swift
    func dismiss() {
        dismissRewrite()
        noteDraft = nil
        engine?.clear()
        engine?.flush()
        panel = .keys
        refresh()
    }
```

(e) `appear()`（第 170–174 行）换成：

```swift
    func appear() {
        engine?.syncNow()
        engine?.refreshClipboard()
        checkPasteboard()
        clipboardHasText = output?.pasteboardHasText ?? false
        syncScope()
    }
```

(f) `setPrivateField`（第 177–188 行）换成：

```swift
    func setPrivateField(_ value: Bool) {
        guard value != privateField else { return }
        privateField = value
        engine?.setPrivate(value)
        if value {
            dismissRewrite()
            clipOffer = nil
            pasteboardChanged = false
            noteDraft = nil
        } else {
            checkPasteboard()
        }
        refreshHint()
    }
```

(g) `poll()`（第 239–257 行）开头的 `guard let engine else { return }` 之后插入：

```swift
        refreshHint()
        // App 删了当前对象时桥会退回「恋爱 · 不指定」
        if let next = engine.scope, next != scope {
            scope = next
            reloadContacts()
        }
```

(h) `refresh()`（第 352–359 行）最后一行 `if !composing, panel == .candidates { panel = .keys }` 之后加 `refreshHint()`。

(i) 在 `private func typeLetter(_ letter: Character) {`（第 284 行）之前插入：

```swift
    /// 提示行这一行在不在：恋爱场景且选了对象时一直在（没有提示时是空行），日常、工作与「恋爱 · 不指定」没有这一行。
    /// 键盘高度只在进出这个状态时变，提示出现与消失不再让宿主界面跳。
    var hasHintRow: Bool { ScopeDisplay.hasHintRow(scene: scope.scene, hasContact: currentContact != nil) }

    /// 宿主换了输入框（控制器按 documentIdentifier 判断）：清掉最近上屏的字与提示。
    func hostChanged() {
        engine?.resetContext()
        refreshHint()
    }

    /// 当前对象（名单里找得到的）。
    var currentContact: MemoryContact? {
        guard let id = scope.contactId else { return nil }
        return contacts.first { $0.id == id }
    }

    /// 「记一笔」：开了完全访问、剪贴板有字、选了对象、不在私密输入框。
    var canNote: Bool {
        ScopeDisplay.canNote(
            fullAccess: fullAccess, clipboardHasText: clipboardHasText, privateField: privateField,
            hasContact: currentContact != nil)
    }

    func openScopePicker() {
        reloadContacts()
        panel = .scope
    }

    /// 选场景与对象。选了对象或换到日常 / 工作就收起面板；换到恋爱还没选对象时留着接着选。
    func chooseScope(scene: String, contactId: String?) {
        guard let engine else { return }
        engine.setScope(scene: scene, contactId: contactId)
        scope = engine.scope ?? scope
        refresh()
        if scene != MemoryScope.dating || contactId != nil { panel = .keys }
    }

    func openContactCard() {
        guard let id = scope.contactId else { return }
        panelCards = engine?.memoryCards(id) ?? []
        panel = .contactCard
    }

    /// 日子提醒的「知道了」：当天不再出。
    func acknowledgeHint() {
        guard let hint else { return }
        engine?.dismissHint(hint.cardId, today: hint.reason == .today)
        refreshHint()
    }

    /// 点「记一笔」：读剪贴板（可能弹系统的粘贴授权），显示确认条。
    func startNote() {
        guard canNote,
              let text = output?.readPasteboard()?.trimmingCharacters(in: .whitespacesAndNewlines),
              !text.isEmpty
        else { return }
        // 一张卡最多 200 字（桥也会校验），长的剪贴板截掉后面的，确认条里看得到截后的样子
        noteDraft = MemoryLimits.clampText(text)
    }

    func confirmNote() {
        guard let text = noteDraft, let id = scope.contactId else { return }
        noteDraft = nil
        // nil 即成功（含桥「已接受、稍后写入」）；写不进（App Group 不可写、对象刚被删）时不弹错，不打断打字
        _ = engine?.memoryNote(id, text: text)
        refreshHint()
    }

    func cancelNote() {
        noteDraft = nil
    }

    /// 换了引擎、键盘出现时：从桥取当前场景，重读名单与提示。
    private func syncScope() {
        scope = engine?.scope ?? MemoryScope()
        reloadContacts()
        refreshHint()
    }

    private func reloadContacts() {
        guard fullAccess, let directory = SharedStore.directory,
              let snapshot = MemoryFiles.read(userDirectory: directory)
        else {
            contacts = []
            return
        }
        contacts = snapshot.contacts.filter { $0.scene == MemoryScope.dating }
    }

    private func refreshHint() {
        let next = privateField ? nil : engine?.memoryHint
        if next != hint { hint = next }
    }

```

- [ ] **Step 5: 提示行、牌子、选择面板、对象卡**

Create `cloud/ios/Keyboard/Sources/HintRow.swift`：

```swift
// 候选栏上方的记忆提示行：恋爱场景选了对象时一直在（没有提示时是空行，高度不变，宿主界面不跳）。
// 有提示时左边强调色圆点与文字（命中的词加粗），右边是「你写的」灰色小字（卡片来源，不可点）和按钮：匹配提示是「展开」（键区换成对象卡），
// 日子 / 约定提醒是「知道了」。

import SwiftUI

struct HintRow: View {
    let model: KeyboardModel

    let hint: MemoryHint?

    var body: some View {
        HStack(spacing: 8) {
            if let hint {
                Circle()
                    .fill(Theme.accent.color)
                    .frame(width: 6, height: 6)
                    .padding(.leading, 12)
                Text(HintText.attributed(text: hint.text, emphasis: model.hintEmphasis))
                    .font(.system(size: 14))
                    .foregroundStyle(Theme.accentInk.color)
                    .lineLimit(1)
                    .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .leading)
                    .onKeyboardPress { model.openContactCard() }
                if let label = HintText.sourceLabel(for: model.hintSource) {
                    Text(label)
                        .font(.system(size: 11))
                        .foregroundStyle(.secondary.opacity(0.7))
                }
                Text(hint.reason == .today ? "知道了" : "展开")
                    .font(.system(size: 14, weight: .medium))
                    .foregroundStyle(Theme.accentInk.color)
                    .padding(.trailing, 12)
                    .frame(maxHeight: .infinity)
                    .onKeyboardPress {
                        if hint.reason == .today { model.acknowledgeHint() } else { model.openContactCard() }
                    }
            } else {
                Color.clear.frame(maxWidth: .infinity)
            }
        }
        .frame(height: KeyStyle.hintRowHeight)
        .background(Theme.accentSoft.color.opacity(hint == nil ? 0 : 1))
        .accessibilityElement(children: .contain)
        .accessibilityLabel(hint.map { "记忆提示：\($0.text)" } ?? "")
    }
}
```

Create `cloud/ios/Keyboard/Sources/ScopeChip.swift`：

```swift
// 候选栏左侧的场景牌子：恋爱场景是强调色的「小美 · 恋爱」，日常 / 工作只是灰色场景名（强调色只跟对象有关）。点它打开选择面板。

import SwiftUI

struct ScopeChip: View {
    let model: KeyboardModel

    var body: some View {
        Text(title)
            .font(.system(size: 14, weight: dating ? .semibold : .regular))
            .foregroundStyle(dating ? Theme.accentInk.color : Color.secondary)
            .lineLimit(1)
            .padding(.horizontal, 10)
            .padding(.vertical, 5)
            .background(Capsule().fill(dating ? Theme.accentSoft.color : Color.secondary.opacity(0.12)))
            .padding(.leading, 8)
            .frame(maxHeight: .infinity)
            .onKeyboardPress { model.openScopePicker() }
            .accessibilityAddTraits(.isButton)
            .accessibilityLabel("场景：\(title)")
    }

    private var dating: Bool { model.scope.scene == MemoryScope.dating }

    private var title: String {
        ScopeDisplay.chipTitle(scene: model.scope.scene, contactName: model.currentContact?.name)
    }
}
```

Create `cloud/ios/Keyboard/Sources/ScopePicker.swift`：

```swift
// 点牌子后键区换成的选择面板：场景三选一；恋爱场景再选对象（App 里建的，最多 8 个，带头像与副文字）或不指定。
// 键盘扩展打不开 App，「新对象」只提示去 App 新建。没开完全访问时读不到 App Group 里的名单，也不让切场景（切了也用不上记忆），
// 面板里只有一句「开启完全访问后才能使用记忆」与「去开启」；桥不知道有没有完全访问，这道门在 Swift 侧。
// 「完成」/「收起」在候选栏那一行右端（IdleBar.panelBar）。

import SwiftUI

struct ScopePicker: View {
    let model: KeyboardModel

    @State private var showsNewContactTip = false

    private static let scenes = [MemoryScope.daily, MemoryScope.dating, MemoryScope.work]

    private let columns = Array(repeating: GridItem(.flexible(), spacing: 8), count: 4)

    var body: some View {
        switch ScopeDisplay.pickerMode(fullAccess: model.fullAccess) {
        case .picker: picker
        case .needsFullAccess: noAccess
        }
    }

    private var noAccess: some View {
        VStack(spacing: 10) {
            Spacer(minLength: 0)
            Text(ScopeDisplay.needsFullAccessText)
                .font(.system(size: 15))
                .foregroundStyle(.secondary)
            Text("去开启")
                .font(.system(size: 15, weight: .medium))
                .foregroundStyle(Theme.accentInk.color)
                .padding(.horizontal, 14)
                .frame(height: 32)
                .background(Capsule().fill(Theme.accentSoft.color))
                .onKeyboardPress { model.showNotice(ScopeDisplay.enableFullAccessNotice) }
            Text(model.notice ?? " ")
                .font(.system(size: 12))
                .foregroundStyle(Theme.accentInk.color)
            Spacer(minLength: 0)
        }
        .frame(maxWidth: .infinity)
    }

    private var picker: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack(spacing: 2) {
                ForEach(Self.scenes, id: \.self) { scene in
                    let selected = model.scope.scene == scene
                    Text(MemoryScope.title(of: scene))
                        .font(.system(size: 15, weight: selected ? .semibold : .regular))
                        .frame(maxWidth: .infinity, minHeight: 30)
                        .background(RoundedRectangle(cornerRadius: 7).fill(selected ? KeyStyle.keyFill : Color.clear))
                        .onKeyboardPress {
                            let keep = scene == model.scope.scene ? model.scope.contactId : nil
                            model.chooseScope(scene: scene, contactId: keep)
                        }
                }
            }
            .padding(2)
            .background(RoundedRectangle(cornerRadius: 9).fill(Color.secondary.opacity(0.15)))
            if model.scope.scene == MemoryScope.dating { contacts }
            Spacer(minLength: 0)
            Text("对象只能你自己切，键盘不知道你在和谁聊")
                .font(.system(size: 12))
                .foregroundStyle(.secondary)
        }
        .padding(.horizontal, 10)
        .padding(.vertical, 8)
    }

    @ViewBuilder
    private var contacts: some View {
        LazyVGrid(columns: columns, spacing: 6) {
            ForEach(model.contacts) { contact in
                cell(
                    avatar: contact.name, title: contact.name,
                    subtitle: ScopeDisplay.contactSubtitle(knownDays: contact.knownDays()),
                    selected: model.scope.contactId == contact.id
                ) {
                    model.chooseScope(scene: MemoryScope.dating, contactId: contact.id)
                }
            }
            cell(
                avatar: "–", title: "不指定", subtitle: ScopeDisplay.noScopeSubtitle,
                selected: model.scope.contactId == nil
            ) {
                model.chooseScope(scene: MemoryScope.dating, contactId: nil)
                model.closePanel()
            }
            cell(
                avatar: "+", title: "新对象",
                subtitle: showsNewContactTip ? "在素笺 App 里新建" : ScopeDisplay.newContactSubtitle(count: model.contacts.count),
                selected: false
            ) { showsNewContactTip = true }
        }
    }

    private func cell(
        avatar: String, title: String, subtitle: String, selected: Bool, action: @escaping () -> Void
    ) -> some View {
        HStack(spacing: 5) {
            MemoryAvatar(name: avatar, size: 24, selected: selected)
            VStack(alignment: .leading, spacing: 0) {
                Text(title).font(.system(size: 13, weight: .medium)).lineLimit(1)
                Text(subtitle)
                    .font(.system(size: 10))
                    .foregroundStyle(.secondary)
                    .lineLimit(1)
                    .minimumScaleFactor(0.8)
            }
            Spacer(minLength: 0)
        }
        .padding(.horizontal, 6)
        .frame(maxWidth: .infinity, minHeight: 40)
        .background(RoundedRectangle(cornerRadius: 8).fill(KeyStyle.keyFill))
        .overlay(
            RoundedRectangle(cornerRadius: 8)
                .stroke(selected ? Theme.accent.color : Color.clear, lineWidth: 2)
        )
        .foregroundStyle(selected ? Theme.accentInk.color : Color.primary)
        .onKeyboardPress(action)
    }
}
```

Create `cloud/ios/Keyboard/Sources/ContactCardPanel.swift`：

```swift
// 提示行「展开」后键区换成的对象卡：头像、名字、认识几天、今日相关最多 3 张卡（左列相对日子，右边标题加小字），页脚是数量说明与「全部记忆」。
// 键盘扩展打不开 App，「全部记忆」只在面板里提示去 App 看；「收起」在候选栏那一行右端（IdleBar.panelBar）。

import SwiftUI

struct ContactCardPanel: View {
    let model: KeyboardModel

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            if let contact = model.currentContact {
                HStack(spacing: 10) {
                    MemoryAvatar(name: contact.name, size: 38, selected: true)
                    VStack(alignment: .leading, spacing: 2) {
                        Text(contact.name).font(.system(size: 17, weight: .semibold))
                        Text(ScopeDisplay.contactSubtitle(knownDays: contact.knownDays()))
                            .font(.system(size: 12))
                            .foregroundStyle(.secondary)
                    }
                }
                if model.panelCards.isEmpty {
                    Text("还没有记下这个人的事")
                        .font(.system(size: 14))
                        .foregroundStyle(.secondary)
                }
                ForEach(model.panelCards) { card in
                    HStack(alignment: .firstTextBaseline, spacing: 8) {
                        Text(card.dateLabel() ?? card.kind.title)
                            .font(.system(size: 12))
                            .foregroundStyle(card.dateLabel() == nil ? Color.secondary : Theme.accentInk.color)
                            .frame(width: 40, alignment: .leading)
                        VStack(alignment: .leading, spacing: 0) {
                            Text(card.text).font(.system(size: 15)).lineLimit(1)
                            if !card.subtitle.isEmpty {
                                Text(card.subtitle).font(.system(size: 11)).foregroundStyle(.secondary).lineLimit(1)
                            }
                        }
                    }
                }
            }
            Spacer(minLength: 0)
            HStack {
                Text(model.notice ?? ScopeDisplay.cardFooter(count: model.panelCards.count))
                    .font(.system(size: 12))
                    .foregroundStyle(model.notice == nil ? Color.secondary : Theme.accentInk.color)
                Spacer()
                Text("全部记忆")
                    .font(.system(size: 14, weight: .medium))
                    .foregroundStyle(Theme.accentInk.color)
                    .padding(.horizontal, 10)
                    .frame(height: 28)
                    .onKeyboardPress { model.showNotice(ScopeDisplay.allMemoryNotice) }
            }
        }
        .padding(.horizontal, 14)
        .padding(.vertical, 8)
    }
}
```

- [ ] **Step 6: 键盘视图与候选栏**

Replace `cloud/ios/Keyboard/Sources/KeyboardView.swift`：

```swift
// 整个键盘：提示行（恋爱场景选了对象时一直在）+ 候选栏 + 键区（或展开的候选 / 表情 / 场景选择 / 对象卡）。键的位置与触摸范围由 KeyboardLayout 算；
// 进出「恋爱 · 某人」时键盘高度加减一行，高度约束在控制器里改（KeyboardViewController.syncHintRow）。

import SwiftUI

struct KeyboardView: View {
    let model: KeyboardModel

    /// 系统要求自带切换键（没有键盘下方的地球键）时，底行 😀 的位置换成地球键。
    let showsGlobe: Bool

    /// 键区高度：四行键加行距与上下留白，展开面板也占这么高，切换时键盘不跳。
    static let keyAreaHeight = KeyStyle.keyHeight * 4 + KeyStyle.rowSpacing * 3 + KeyboardLayout.topPadding + 6

    var body: some View {
        VStack(spacing: 0) {
            if model.hasHintRow {
                Group {
                    if model.noteDraft != nil || model.noteDone {
                        NoteBar(model: model)
                    } else {
                        HintRow(model: model, hint: model.hint)
                    }
                }
                .transition(.move(edge: .top).combined(with: .opacity))
            }
            CandidateBar(model: model)
            Group {
                switch model.panel {
                case .keys: keys
                // 面板是控制器挂的 UIKit 视图（CandidatePanelView），这里留空占位
                case .candidates: Color.clear
                case .emoji: EmojiPanel(model: model)
                case .scope: ScopePicker(model: model)
                case .contactCard: ContactCardPanel(model: model)
                }
            }
            .frame(height: Self.keyAreaHeight)
        }
        .frame(maxHeight: .infinity, alignment: .top)
        .animation(.easeOut(duration: 0.2), value: model.hasHintRow)
    }

    private var keys: some View {
        GeometryReader { geometry in
            let slots = KeyboardLayout.slots(layer: model.layer, showsGlobe: showsGlobe, size: geometry.size)
            ZStack(alignment: .topLeading) {
                ForEach(Array(slots.enumerated()), id: \.offset) { index, slot in
                    KeyButton(
                        key: slot.key, model: model, insets: slot.insets,
                        pressed: model.pressedSlots.contains(index))
                        .frame(width: slot.cell.width, height: slot.cell.height)
                        .position(x: slot.cell.midX, y: slot.cell.midY)
                }
            }
        }
    }
}
```

Replace `cloud/ios/Keyboard/Sources/IdleBar.swift`：

```swift
// 没在组字时的候选栏：私密输入框只亮一把锁；场景 / 对象卡面板打开时只留牌子与「完成」/「收起」；有别的设备刚复制的文字就提示它；
// 否则左边是场景牌子与「✨ 润色」，右边是「记一笔」与「发到其他设备」。润色进行中整栏交给 RewriteBar。
// 「记一笔」的确认条在提示行的位置（NoteBar），这一行的牌子照常在。

import SwiftUI

struct IdleBar: View {
    let model: KeyboardModel

    var body: some View {
        Group {
            if model.privateField {
                Label("隐私输入：不学习、不上传", systemImage: "lock.fill")
                    .font(.system(size: 13))
                    .foregroundStyle(.secondary)
                    .frame(maxWidth: .infinity)
            } else if model.panel == .scope || model.panel == .contactCard {
                panelBar
            } else if let offer = model.clipOffer {
                ClipOfferBar(model: model, offer: offer)
            } else if model.rewrite != .idle {
                RewriteBar(model: model)
            } else {
                actions
            }
        }
        .frame(height: KeyStyle.candidateBarHeight)
    }

    private var actions: some View {
        HStack(spacing: 0) {
            ScopeChip(model: model)
            if model.rewriteAvailable {
                Label("润色", systemImage: "sparkles")
                    .font(.system(size: 16))
                    .padding(.horizontal, 12)
                    .frame(maxHeight: .infinity)
                    .onKeyboardTap { model.startRewrite() }
            }
            Spacer()
            if model.canNote, model.noteDraft == nil, !model.noteDone {
                Label("记一笔", systemImage: "square.and.pencil")
                    .font(.system(size: 15))
                    .padding(.horizontal, 10)
                    .frame(maxHeight: .infinity)
                    .onKeyboardTap { model.startNote() }
            }
            if model.pasteboardChanged {
                Label("发到其他设备", systemImage: "arrow.up.doc.on.clipboard")
                    .font(.system(size: 15))
                    .padding(.horizontal, 10)
                    .frame(maxHeight: .infinity)
                    .onKeyboardTap { model.pushPasteboard() }
                Image(systemName: "xmark")
                    .font(.system(size: 13, weight: .medium))
                    .foregroundStyle(.secondary)
                    .frame(width: 40, height: KeyStyle.candidateBarHeight)
                    .onKeyboardPress { model.dismissPasteboard() }
                    .accessibilityLabel("不发送")
            }
        }
    }

    /// 场景选择与对象卡打开时的工具栏：牌子与右端的「完成」（选择面板）/「收起」（对象卡、没开完全访问）。
    private var panelBar: some View {
        HStack(spacing: 0) {
            ScopeChip(model: model)
            Spacer()
            Text(model.panel == .scope && model.fullAccess ? "完成" : "收起")
                .font(.system(size: 15, weight: .medium))
                .foregroundStyle(Theme.accentInk.color)
                .padding(.horizontal, 14)
                .frame(maxHeight: .infinity)
                .onKeyboardPress { model.closePanel() }
        }
    }
}
```

- [ ] **Step 7: 控制器：高度约束与触摸范围**

Modify `cloud/ios/Keyboard/Sources/KeyboardViewController.swift`：

(a) 第 26 行 `private var engineSignature = ""` 之后加：

```swift

    /// 钉在 inputView 上的键盘高度；提示行出现时加一行（见 `syncHintRow`）。
    private var heightConstraint: NSLayoutConstraint?
```

(b) `viewDidLoad` 里 `mountTouchView()` 之后加一行 `syncHintRow()`。

(c) `viewDidLayoutSubviews`（第 44–53 行）里的 `let keyArea = CGRect(…)` 换成：

```swift
        let keyArea = CGRect(
            x: 0, y: hintInset + KeyStyle.candidateBarHeight, width: view.bounds.width,
            height: KeyboardView.keyAreaHeight)
```

(d) `viewWillAppear` 里 `model.appear()` 之前加一行 `model.fullAccess = hasFullAccess`。

(e) `mountKeyboard()` 里

```swift
        let height = view.heightAnchor.constraint(
            equalToConstant: KeyStyle.candidateBarHeight + KeyboardView.keyAreaHeight)
```

换成

```swift
        let height = view.heightAnchor.constraint(equalToConstant: baseHeight)
```

并在 `NSLayoutConstraint.activate([…])` 之后加一行 `heightConstraint = height`。

(f) `syncTouchView()`（第 215–232 行）换成：

```swift
    private func syncTouchView() {
        let size = CGSize(width: view.bounds.width, height: KeyboardView.keyAreaHeight)
        let (layer, panel, composing, hinted) = withObservationTracking {
            (model.layer, model.panel, model.composing, model.hasHintRow)
        } onChange: { [weak self] in
            Task { @MainActor in self?.syncTouchView() }
        }
        let slots = panel == .keys
            ? KeyboardLayout.slots(layer: layer, showsGlobe: needsInputModeSwitchKey, size: size)
            : []
        if slots.map(\.key) != touchView.slots.map(\.key) { touchView.resetTouches() }
        touchView.slots = slots
        let top = hinted ? KeyStyle.hintRowHeight : 0
        touchView.chevron = composing
            ? CGRect(
                x: view.bounds.width - CandidateBar.chevronWidth, y: top,
                width: CandidateBar.chevronWidth, height: KeyStyle.candidateBarHeight)
            : nil
    }
```

(g) 在 `syncPanel()` 之后加：

```swift
    /// 进出「恋爱 · 某人」时提示行这一行加上 / 去掉：键盘高度加减一行（0.2 秒），键区与 ⌄ 的触摸范围在
    /// viewDidLayoutSubviews / syncTouchView 里跟着下移。提示本身出现消失不改高度（行一直在，空着而已）。
    private func syncHintRow() {
        let visible = withObservationTracking {
            model.hasHintRow
        } onChange: { [weak self] in
            Task { @MainActor in self?.syncHintRow() }
        }
        let height = baseHeight + (visible ? KeyStyle.hintRowHeight : 0)
        guard let heightConstraint, heightConstraint.constant != height else { return }
        heightConstraint.constant = height
        view.setNeedsLayout()
        UIView.animate(withDuration: 0.2) { [weak self] in
            self?.view.layoutIfNeeded()
        }
    }

    /// 没有提示行时的键盘高度：候选栏加键区。
    private var baseHeight: CGFloat { KeyStyle.candidateBarHeight + KeyboardView.keyAreaHeight }

    /// 提示行占掉的高度：键区与 ⌄ 往下挪这么多。
    private var hintInset: CGFloat { model.hasHintRow ? KeyStyle.hintRowHeight : 0 }
```

(h) `textDidChange(_:)`（第 136–139 行）换成（宿主换了输入框就让桥清掉最近上屏的字；`documentIdentifier` 每个输入框一个，我们自己上屏不会改它）：

```swift
    /// 同一次弹出里焦点也会换输入框（填完用户名跳到验证码），每次都重判；换了输入框还要清掉最近上屏的字，
    /// 免得在 A 聊天里打的字在 B 里触发记忆提示。
    override func textDidChange(_ textInput: (any UITextInput)?) {
        super.textDidChange(textInput)
        updatePrivacy()
        let document = hostDocumentIdentifier
        if HostDocument.changed(from: lastDocument, to: document) {
            lastDocument = document
            model.hostChanged()
        }
    }

    /// 宿主输入框的标识。`documentIdentifier` 声明为非可选，但连上宿主之前系统返回 nil，Swift 桥接时直接崩（textDidChange 在这之前就会被调，
    /// 模拟器里实测 EXC_BREAKPOINT），所以走 KVC 取成可选值。
    private var hostDocumentIdentifier: UUID? {
        (textDocumentProxy as? NSObject)?.value(forKey: "documentIdentifier") as? UUID
    }
```

(i) (a) 里加的 `heightConstraint` 之后再加：

```swift

    /// 上一次看到的宿主输入框（`textDocumentProxy.documentIdentifier`）。
    private var lastDocument: UUID?
```

### Step 7b：设计稿 01 / 05 对照后的修订（2026-10-04，审计会话审 1568829 后）

6 处界面按设计稿改，代码已落地（`xcodebuild test` 49 个）。上面各 Step 的代码块已同步成实际文件内容，下面是新增的文件与对 `KeyboardModel.swift`、`KeyboardViewController.swift` 的改动。

1. **强调色**：灰绿 `Theme.accent / accentInk / accentSoft`（`Shared/Theme.swift`，色值用 `OKLCH.srgb` 换算并有单测核对）；只用在牌子（恋爱）、提示行、对象卡与选择面板、确认条；日常 / 工作牌子灰色。`CandidateCell`、`ClipOfferBar`、`RewriteBar` 里的 `Color.accentColor` 是云端候选 / 剪贴板 / 润色的标识，不属于对象，未动。
2. **提示行加粗**：桥的 `Hint` 没有命中词字段，不改桥；`HintText.emphasis` 日子 / 约定提醒取卡片文字、匹配提示取卡片关键词，`HintText.attributed` 加粗第一个命中的词；卡片来自 `qj_memory_read` 的快照，`KeyboardModel.cardIndex` 缓存。
3. **对象卡**：左列 `MemoryCard.dateLabel`（今天 / 明天 / 周几 / `M.dd`），标题下小字 `MemoryCard.subtitle`，页脚「只显示与今天有关的 N 条」与「全部记忆」。键盘扩展没有官方办法打开容器 App（沿响应链调 `UIApplication.open` 会被 App Store 审核拒绝，不用），点「全部记忆」只在面板里提示「在素笺 App 里查看全部记忆」2 秒。「收起」在候选栏那一行右端，「记一笔」在这一屏不显示（`IdleBar.panelBar`）。
4. **选择面板**：对象格带头像与副文字（对象：认识 N 天，没有「最近使用」数据；不指定：只用场景；新对象：n / 8），「完成」在候选栏那一行右端。
5. **记一笔**：确认条（`NoteBar`）占提示行的位置（选了对象才有记一笔，那一行必在），牌子那一行保留；「刚复制的」加粗加截断原文，「忽略」描边、「记到 小美」强调色底；记下后一行「记下了」2 秒。
6. **没开完全访问**：句子下加「去开启」，点了只提示「在素笺 App 里按引导开启完全访问」2 秒；面板高度与正常键盘一致（同一个 `keyAreaHeight`）。
7. **来源标签（设计稿 05 的 2c）**：手动卡的提示右边是灰色小字「你写的」（不可点，`HintText.sourceLabel`）；2A 的提示全是手动卡，「展开」仍保留在它右边（设计稿没说 2c 里怎么进对象卡，且去掉「展开」会让 1a 进不了面板）；点提示文字本身也能展开。

新增文件：

Create `cloud/ios/Shared/OKLCH.swift`：

```swift
// oklch 颜色换算成 sRGB（Björn Ottosson 的 OKLab 矩阵，sRGB 传输函数），Theme 的色值用它核对。

import Foundation

enum OKLCH {
    /// 每个通道 0–255；超出 sRGB 色域的通道夹到边界。
    static func srgb(l: Double, c: Double, h: Double) -> (r: Int, g: Int, b: Int) {
        let radians = h * .pi / 180
        let a = c * cos(radians)
        let b = c * sin(radians)
        let l1 = pow(l + 0.3963377774 * a + 0.2158037573 * b, 3)
        let m1 = pow(l - 0.1055613458 * a - 0.0638541728 * b, 3)
        let s1 = pow(l - 0.0894841775 * a - 1.2914855480 * b, 3)
        let red = 4.0767416621 * l1 - 3.3077115913 * m1 + 0.2309699292 * s1
        let green = -1.2684380046 * l1 + 2.6097574011 * m1 - 0.3413193965 * s1
        let blue = -0.0041960863 * l1 - 0.7034186147 * m1 + 1.7076147010 * s1
        return (channel(red), channel(green), channel(blue))
    }

    private static func channel(_ linear: Double) -> Int {
        let clamped = min(max(linear, 0), 1)
        let encoded = clamped <= 0.0031308 ? 12.92 * clamped : 1.055 * pow(clamped, 1 / 2.4) - 0.055
        return Int((encoded * 255).rounded())
    }
}
```

Create `cloud/ios/Shared/ThemeSwatch.swift`：

```swift
// 主题里的一个颜色：写死的 sRGB 十六进制，加上它出自的 oklch 值（单测用 OKLCH.srgb 核对两者一致）。

import SwiftUI
import UIKit

struct ThemeSwatch: Sendable {
    let hex: UInt32

    /// 出处：oklch 的明度、色度、色相（度）。
    let l: Double

    let c: Double

    let h: Double

    var r: Int { Int(hex >> 16) & 0xFF }

    var g: Int { Int(hex >> 8) & 0xFF }

    var b: Int { Int(hex) & 0xFF }

    var color: Color { Color(uiColor) }

    var uiColor: UIColor {
        UIColor(red: CGFloat(r) / 255, green: CGFloat(g) / 255, blue: CGFloat(b) / 255, alpha: 1)
    }
}
```

Create `cloud/ios/Shared/Theme.swift`：

```swift
// 素笺的强调色：灰绿，只用在和对象有关的地方（对象牌子、提示行、对象卡与选择面板）。工作 / 日常场景完全不用。
// 设计稿 theme.css 的 oklch 值，已用 `OKLCH.srgb`（Tests/ThemeTests 核对）换算成 sRGB：
//   --accent      oklch(0.89  0.06  150)  #C0E7C6  圆点、头像底、选中描边
//   --accent-ink  oklch(0.38  0.05  150)  #2E4A34  强调色上的文字、按钮文字
//   --accent-soft oklch(0.965 0.025 150)  #E8F9EB  提示行与牌子的底色

import SwiftUI

enum Theme {
    static let accent = ThemeSwatch(hex: 0xC0E7C6, l: 0.89, c: 0.06, h: 150)

    static let accentInk = ThemeSwatch(hex: 0x2E4A34, l: 0.38, c: 0.05, h: 150)

    static let accentSoft = ThemeSwatch(hex: 0xE8F9EB, l: 0.965, c: 0.025, h: 150)
}
```

Create `cloud/ios/Shared/Memory/HintText.swift`：

```swift
// 提示行的文字：命中的词加粗（桥的 Hint 不带命中词，由 Swift 侧按卡片关键词找），以及提示来源的小标签（手动卡显示「你写的」，灰字不可点）。

import Foundation

enum HintText {
    /// 在 `text` 里把 `emphasis` 中第一个出现的词加粗；一个都没出现、词为空时原样返回。
    static func attributed(text: String, emphasis: [String]) -> AttributedString {
        var result = AttributedString(text)
        for word in emphasis where !word.isEmpty {
            guard let found = text.range(of: word),
                  let range = Range(found, in: result)
            else { continue }
            result[range].inlinePresentationIntent = .stronglyEmphasized
            break
        }
        return result
    }

    /// 要加粗的词：日子 / 约定提醒取卡片文字（「明天是她的**生日**」），匹配提示取卡片关键词。
    static func emphasis(for hint: MemoryHint, card: MemoryCard?) -> [String] {
        guard let card else { return [] }
        return hint.reason == .today ? [card.text] : card.keywords
    }

    /// 提示右边的来源小字：手动卡是「你写的」（2A 的卡都是手动卡），云端卡不写。
    static func sourceLabel(for source: String?) -> String? {
        (source ?? "manual") == "manual" ? "你写的" : nil
    }
}
```

Create `cloud/ios/Keyboard/Sources/NoteBar.swift`：

```swift
// 「记一笔」的确认条：占提示行的位置（选了对象才能记，那一行一定在），牌子那一行保留。
// 左边「刚复制的」加粗加截断的原文，右边「忽略」（描边）与「记到 {对象}」（强调色底）；记下后变成一行「记下了」，2 秒后消失。

import SwiftUI

struct NoteBar: View {
    let model: KeyboardModel

    var body: some View {
        HStack(spacing: 8) {
            if let draft = model.noteDraft {
                (Text("刚复制的 ").bold() + Text(draft.replacingOccurrences(of: "\n", with: " ")))
                    .font(.system(size: 14))
                    .foregroundStyle(Theme.accentInk.color)
                    .lineLimit(1)
                    .truncationMode(.tail)
                    .padding(.leading, 12)
                    .frame(maxWidth: .infinity, alignment: .leading)
                Text("忽略")
                    .font(.system(size: 13))
                    .foregroundStyle(Theme.accentInk.color)
                    .padding(.horizontal, 10)
                    .frame(height: 26)
                    .overlay(RoundedRectangle(cornerRadius: 13).stroke(Theme.accentInk.color.opacity(0.4), lineWidth: 1))
                    .onKeyboardPress { model.cancelNote() }
                Text("记到 \(model.currentContact?.name ?? "")")
                    .font(.system(size: 13, weight: .medium))
                    .foregroundStyle(Theme.accentInk.color)
                    .padding(.horizontal, 10)
                    .frame(height: 26)
                    .background(Capsule().fill(Theme.accent.color))
                    .padding(.trailing, 12)
                    .onKeyboardPress { model.confirmNote() }
            } else {
                Text("记下了")
                    .font(.system(size: 14, weight: .medium))
                    .foregroundStyle(Theme.accentInk.color)
                    .padding(.leading, 12)
                    .frame(maxWidth: .infinity, alignment: .leading)
            }
        }
        .frame(height: KeyStyle.hintRowHeight)
        .background(Theme.accentSoft.color)
    }
}
```

Create `cloud/ios/Tests/ThemeTests.swift`：

```swift
// 主题色：oklch 换算成 sRGB 与写死的十六进制一致（容差 1/255）。

import XCTest
@testable import QingjianCloud

final class ThemeTests: XCTestCase {
    func testSwatchesMatchOKLCHSources() {
        for swatch in [Theme.accent, Theme.accentInk, Theme.accentSoft] {
            let converted = OKLCH.srgb(l: swatch.l, c: swatch.c, h: swatch.h)
            XCTAssertLessThanOrEqual(abs(converted.r - swatch.r), 1, "\(swatch.hex) r")
            XCTAssertLessThanOrEqual(abs(converted.g - swatch.g), 1, "\(swatch.hex) g")
            XCTAssertLessThanOrEqual(abs(converted.b - swatch.b), 1, "\(swatch.hex) b")
        }
    }

    func testSwatchHexValues() {
        XCTAssertEqual(Theme.accent.hex, 0xC0E7C6)
        XCTAssertEqual(Theme.accentInk.hex, 0x2E4A34)
        XCTAssertEqual(Theme.accentSoft.hex, 0xE8F9EB)
    }

    func testOKLCHKnownPoints() {
        let white = OKLCH.srgb(l: 1, c: 0, h: 0)
        XCTAssertEqual([white.r, white.g, white.b], [255, 255, 255])
        let black = OKLCH.srgb(l: 0, c: 0, h: 0)
        XCTAssertEqual([black.r, black.g, black.b], [0, 0, 0])
    }
}
```

Create `cloud/ios/Tests/HintTextTests.swift`：

```swift
// 提示行文字：关键词加粗、来源标签；对象卡与选择面板的文字（相对日子、副文字、页脚）。

import XCTest
@testable import QingjianCloud

final class HintTextTests: XCTestCase {
    private func boldRuns(_ attributed: AttributedString) -> [String] {
        attributed.runs.compactMap { run in
            run.inlinePresentationIntent?.contains(.stronglyEmphasized) == true
                ? String(attributed[run.range].characters) : nil
        }
    }

    private func date(_ text: String) throws -> Date { try XCTUnwrap(MemoryDate.parse(text)) }

    func testBoldsFirstMatchingWord() {
        let result = HintText.attributed(text: "不吃香菜，喜欢草莓蛋糕", emphasis: ["蛋糕", "香菜"])
        XCTAssertEqual(boldRuns(result), ["蛋糕"])
        XCTAssertEqual(String(result.characters), "不吃香菜，喜欢草莓蛋糕")
    }

    func testSkipsWordsThatDoNotAppear() {
        let result = HintText.attributed(text: "明天是她的生日", emphasis: ["考试", "生日"])
        XCTAssertEqual(boldRuns(result), ["生日"])
    }

    func testNoMatchMeansNoBold() {
        XCTAssertEqual(boldRuns(HintText.attributed(text: "明天是她的生日", emphasis: ["蛋糕"])), [])
        XCTAssertEqual(boldRuns(HintText.attributed(text: "明天是她的生日", emphasis: [])), [])
        XCTAssertEqual(boldRuns(HintText.attributed(text: "明天是她的生日", emphasis: [""])), [])
    }

    func testEmojiDoesNotShiftRange() {
        let result = HintText.attributed(text: "🎂她周三考科目二👩‍❤️‍👨加油", emphasis: ["科目二"])
        XCTAssertEqual(boldRuns(result), ["科目二"])
        XCTAssertEqual(String(result.characters), "🎂她周三考科目二👩‍❤️‍👨加油")
    }

    func testEmphasisForReasons() {
        var card = MemoryCard.new(kind: .date, text: "生日", when: "1998-10-05", keywords: ["生日", "蛋糕"])
        let today = MemoryHint(cardId: "c", text: "明天是她的生日", reason: .today, more: false)
        XCTAssertEqual(HintText.emphasis(for: today, card: card), ["生日"], "日子提醒加粗卡片文字里的那个词")
        let match = MemoryHint(cardId: "c", text: "x", reason: .match, more: false)
        XCTAssertEqual(HintText.emphasis(for: match, card: card), ["生日", "蛋糕"], "匹配提示加粗关键词")
        card.keywords = []
        XCTAssertEqual(HintText.emphasis(for: match, card: card), [])
        XCTAssertEqual(HintText.emphasis(for: match, card: nil), [])
    }

    func testSourceLabel() {
        XCTAssertEqual(HintText.sourceLabel(for: "manual"), "你写的")
        XCTAssertNil(HintText.sourceLabel(for: "cloud"))
        XCTAssertEqual(HintText.sourceLabel(for: nil), "你写的", "2A 里卡片都是手动卡")
    }

    func testDateLabelIsRelativeWithinThreeDays() throws {
        let now = try date("2026-10-04")
        func label(_ when: String, kind: MemoryCard.Kind = .promise) -> String? {
            MemoryCard.new(kind: kind, text: "x", when: when, keywords: []).dateLabel(now: now)
        }
        XCTAssertEqual(label("2026-10-04"), "今天")
        XCTAssertEqual(label("2026-10-05"), "明天")
        XCTAssertEqual(label("2026-10-06"), "周二")
        XCTAssertEqual(label("2026-10-07"), "周三")
        XCTAssertEqual(label("2026-10-20"), "10.20")
        XCTAssertEqual(label("2026-12-05"), "12.05")
        XCTAssertEqual(label("1998-10-05", kind: .date), "明天", "日子按年重复，取下一次")
        XCTAssertEqual(label("1998-03-09", kind: .date), "3.09")
        XCTAssertNil(MemoryCard.new(kind: .preference, text: "x", when: nil, keywords: []).dateLabel(now: now))
    }

    func testSubtitleSkipsKeywordsEqualToTitle() {
        var card = MemoryCard.new(kind: .date, text: "生日", when: nil, keywords: ["生日"])
        XCTAssertEqual(card.subtitle, "")
        card.keywords = ["生日", "蛋糕", "惊喜"]
        XCTAssertEqual(card.subtitle, "蛋糕 · 惊喜")
        card.keywords = []
        XCTAssertEqual(card.subtitle, "")
    }

    func testPanelTexts() {
        XCTAssertEqual(ScopeDisplay.cardFooter(count: 2), "只显示与今天有关的 2 条")
        XCTAssertEqual(ScopeDisplay.contactSubtitle(knownDays: 13), "认识 13 天")
        XCTAssertEqual(ScopeDisplay.newContactSubtitle(count: 3), "3 / 8")
        XCTAssertEqual(ScopeDisplay.maxContacts, 8)
        XCTAssertEqual(ScopeDisplay.noScopeSubtitle, "只用场景")
        XCTAssertEqual(ScopeDisplay.allMemoryNotice, "在素笺 App 里查看全部记忆")
        XCTAssertEqual(ScopeDisplay.enableFullAccessNotice, "在素笺 App 里按引导开启完全访问")
    }
}
```

对 `KeyboardModel.swift`、`KeyboardViewController.swift` 的改动（相对 1568829）：

```diff
diff --git a/cloud/ios/Keyboard/Sources/KeyboardModel.swift b/cloud/ios/Keyboard/Sources/KeyboardModel.swift
index 27494f4..66ed6f4 100644
--- a/cloud/ios/Keyboard/Sources/KeyboardModel.swift
+++ b/cloud/ios/Keyboard/Sources/KeyboardModel.swift
@@ -37,6 +37,15 @@ final class KeyboardModel {
     /// 「记一笔」确认条里的剪贴板文字；nil 时不显示。
     private(set) var noteDraft: String?
 
+    /// 「记一笔」刚记下：确认条换成一行「记下了」，2 秒后消失。
+    private(set) var noteDone = false
+
+    /// 面板里的一行短提示（键盘扩展打不开 App，「全部记忆」「去开启」只能这样告诉用户），2 秒后消失。
+    private(set) var notice: String?
+
+    /// 当前对象之外的所有恋爱对象的卡片，按卡片 id 查（提示行加粗关键词、来源标签用）。
+    @ObservationIgnored private var cardIndex: [String: MemoryCard] = [:]
+
     /// 对象卡面板里的卡片。
     private(set) var panelCards: [MemoryCard] = []
 
@@ -184,6 +193,8 @@ final class KeyboardModel {
     func dismiss() {
         dismissRewrite()
         noteDraft = nil
+        noteDone = false
+        notice = nil
         engine?.clear()
         engine?.flush()
         panel = .keys
@@ -379,7 +390,16 @@ final class KeyboardModel {
         guard let text = noteDraft, let id = scope.contactId else { return }
         noteDraft = nil
         // nil 即成功（含桥「已接受、稍后写入」）；写不进（App Group 不可写、对象刚被删）时不弹错，不打断打字
-        _ = engine?.memoryNote(id, text: text)
+        if engine?.memoryNote(id, text: text) == nil {
+            noteDone = true
+            noteDoneTask?.cancel()
+            noteDoneTask = Task { @MainActor [weak self] in
+                try? await Task.sleep(for: .seconds(2))
+                guard !Task.isCancelled else { return }
+                self?.noteDone = false
+            }
+        }
+        reloadContacts()
         refreshHint()
     }
 
@@ -387,6 +407,29 @@ final class KeyboardModel {
         noteDraft = nil
     }
 
+    /// 提示行里要加粗的词。
+    var hintEmphasis: [String] {
+        guard let hint else { return [] }
+        return HintText.emphasis(for: hint, card: cardIndex[hint.cardId])
+    }
+
+    /// 提示对应卡片的来源（手动卡显示「你写的」）。
+    var hintSource: String? { hint.flatMap { cardIndex[$0.cardId]?.source } }
+
+    /// 面板里显示一行 2 秒的短提示。
+    func showNotice(_ text: String) {
+        notice = text
+        noticeTask?.cancel()
+        noticeTask = Task { @MainActor [weak self] in
+            try? await Task.sleep(for: .seconds(2))
+            guard !Task.isCancelled else { return }
+            self?.notice = nil
+        }
+    }
+
+    @ObservationIgnored private var noticeTask: Task<Void, Never>?
+    @ObservationIgnored private var noteDoneTask: Task<Void, Never>?
+
     /// 换了引擎、键盘出现时：从桥取当前场景，重读名单与提示。
     private func syncScope() {
         scope = engine?.scope ?? MemoryScope()
@@ -399,13 +442,18 @@ final class KeyboardModel {
               let snapshot = MemoryFiles.read(userDirectory: directory)
         else {
             contacts = []
+            cardIndex = [:]
             return
         }
         contacts = snapshot.contacts.filter { $0.scene == MemoryScope.dating }
+        cardIndex = Dictionary(
+            snapshot.cards.values.joined().map { ($0.id, $0) }, uniquingKeysWith: { first, _ in first })
     }
 
     private func refreshHint() {
         let next = privateField ? nil : engine?.memoryHint
+        // 提示的卡是刚记下的、名单缓存里还没有时，重读一次再显示
+        if let next, cardIndex[next.cardId] == nil { reloadContacts() }
         if next != hint { hint = next }
     }
 
diff --git a/cloud/ios/Keyboard/Sources/KeyboardViewController.swift b/cloud/ios/Keyboard/Sources/KeyboardViewController.swift
index 5267301..5257794 100644
--- a/cloud/ios/Keyboard/Sources/KeyboardViewController.swift
+++ b/cloud/ios/Keyboard/Sources/KeyboardViewController.swift
@@ -153,8 +153,8 @@ final class KeyboardViewController: UIInputViewController, TextOutput {
         }
     }
 
-    /// 宿主输入框的标识。`documentIdentifier` 声明为非可选，但连上宿主之前系统返回 nil，Swift 桥接时直接崩（textDidChange 在这之前就会被调），
-    /// 所以走 KVC 取成可选值。
+    /// 宿主输入框的标识。`textDocumentProxy.documentIdentifier` 声明为非可选 UUID，但键盘刚弹出、连上宿主之前系统返回 nil，
+    /// 直接读会在 UUID 桥接处 EXC_BREAKPOINT 崩溃，所以走 KVC 取成可选值，别「简化」回去。
     private var hostDocumentIdentifier: UUID? {
         (textDocumentProxy as? NSObject)?.value(forKey: "documentIdentifier") as? UUID
     }
```

- [ ] **Step 8: 重编桥、生成工程、构建**

Run: `cd /Users/liyuqing/sproot/qingjian-mainline/cloud/ios && scripts/build-bridge.sh && xcodegen generate && xcodebuild -scheme QingjianCloud -destination 'platform=iOS Simulator,name=iPhone 17e' build 2>&1 | grep -E "error:|warning: .*Memory|BUILD" | tail -20`
Expected: 没有 `error:`，最后 `** BUILD SUCCEEDED **`。（Swift 6 并发检查若报 `UIView.animate` 闭包捕获的问题，把闭包改成 `{ [weak self] in MainActor.assumeIsolated { self?.view.layoutIfNeeded() } }` 再编。）

- [ ] **Step 9: 原有单元测试照过**

Run: `cd /Users/liyuqing/sproot/qingjian-mainline/cloud/ios && xcodebuild -scheme QingjianCloud -destination 'platform=iOS Simulator,name=iPhone 17e' test 2>&1 | grep -E "Executed|TEST (SUCCEEDED|FAILED)" | tail -3`
Expected: `** TEST SUCCEEDED **`（`AccountDecodeTests`、`AccountStoreTests` 照过，加 `MemoryModelTests` 21 个、`ThemeTests` 3 个、`HintTextTests` 9 个，共 49 个）。

- [ ] **Step 10: 模拟器里对照截图**

App 的「键盘记住的事」在 Task 6 才有，这一步直接往模拟器的 App Group 里放样例数据（合成的，不是真实聊天）：

```bash
cd /Users/liyuqing/sproot/qingjian-mainline/cloud/ios
xcrun simctl boot "iPhone 17e" 2>/dev/null || true
app=$(find ~/Library/Developer/Xcode/DerivedData -path "*Debug-iphonesimulator/QingjianCloud.app" -maxdepth 6 | head -1)
xcrun simctl install booted "$app"
xcrun simctl spawn booted defaults write .GlobalPreferences AppleKeyboards -array "app.qingjian.cloud.keyboard" "en_US@sw=QWERTY;hw=Automatic"
group=$(xcrun simctl get_app_container booted app.qingjian.cloud group.app.qingjian.cloud)
mem="$group/Library/Application Support/Qingjian/memory"
id=0123456789abcdef0123456789abcdef
mkdir -p "$mem/$id"
today=$(TZ=Asia/Shanghai date +%F); tomorrow=$(TZ=Asia/Shanghai date -v+1d +%F)
printf '[{"id":"%s","name":"小美","pronoun":"ta_f","scene":"dating","created_at":%s}]' "$id" "$(( $(date +%s) - 12*86400 ))" > "$mem/contacts.json"
printf '{"rev":1,"cards":[{"id":"fedcba9876543210fedcba9876543210","kind":"date","text":"生日","keywords":["生日"],"when":"1998-%s","source":"manual","confirmed":true,"created_at":0,"touched_at":0},{"id":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","kind":"preference","text":"不吃香菜，喜欢草莓蛋糕","keywords":["蛋糕","香菜"],"source":"manual","confirmed":true,"created_at":0,"touched_at":1}]}' "${tomorrow#*-}" > "$mem/$id/cards.json"
printf '{"scene":"dating","contact_id":"%s"}' "$id" > "$mem/state.json"
mkdir -p build/screenshots
```

然后在模拟器里手动操作并截图（「完全访问」要在 设置 → 通用 → 键盘 → 键盘 → 青简 里手动打开；每张截图用 `xcrun simctl io booted screenshot build/screenshots/<名字>.png`）：

（生日写的是 1998 年的明天这个月日，顺带验证日子按年重复。）

1. 打开 App 的试打框，弹出键盘：候选栏上方是提示行「明天是她的生日」+「知道了」，键盘比日常场景高一行 → `task5-1a-reminder.png`。
2. 点「知道了」：提示文字消失，**这一行留着（空行）、键盘高度不变**，宿主界面不跳 → `task5-1a-dismissed.png`；关掉键盘再弹出（或在 Xcode 里杀掉键盘进程），提醒当天不再出（`dismissed.json` 里有这张卡）。
3. 看候选栏左侧牌子是强调色的「小美 · 恋爱」→ 点它：键区换成选择面板（三段场景、小美 / 不指定 / ＋ 新对象、底部那句话与「收起」）→ `task5-1c-picker.png`；点「＋ 新对象」出现「在素笺 App 里新建」。
4. 选「日常」：面板收起，牌子变灰色「日常」，**提示行这一行去掉、键盘矮一行**（只有进出「恋爱 · 某人」时高度才变）→ `task5-1d-daily.png`；再选「工作」：同样没有提示行、什么记忆界面都没有，对照 01 的 1f → `task5-1f-work.png`；再切回恋爱、选「不指定」：也没有提示行；最后选小美，提示行回来。
5. 打 `dangao` 选「蛋糕」：提示行出现「不吃香菜，喜欢草莓蛋糕」+「展开」→ 点「展开」：键区换成对象卡（头像「小」、小美、认识 13 天、两张卡、底部「全部记忆在素笺 App 里」「收起」）→ `task5-1b-card.png`。
6. 在别处复制一段字，回到试打框：候选栏右侧出现「记一笔」→ 点它（模拟器会弹粘贴授权，允许）→ 确认条「记到 小美」/「忽略」→ `task5-2c-note.png`；点「记到 小美」后 `cat "$mem/$id/cards.json"` 里多了一张 `other` 卡。
7. 关掉完全访问再弹键盘、点牌子：键区只有一句「开启完全访问后才能使用记忆」与「收起」，没有场景分段、切不了场景 → `task5-1e-no-access.png`。
8. 在「备忘录」这类输入框靠近屏幕底部的应用里打字，切到「恋爱 · 小美」（键盘高一行）时宿主输入框跟着上移、没被键盘挡住；之后提示出现消失时宿主不跳 → `task5-host.png`。
9. 恋爱 · 小美下打 `dangao` 上屏「蛋糕」看到提示后，点到同一个应用的另一个输入框再打 `nihao`：不出「蛋糕」那张卡的提示（换输入框时清了最近上屏的字）。

模拟器操作的坑（落地时踩的）：开完全访问要先点 设置 → 通用 → 键盘 → 键盘 → 青简 → 允许完全访问，再在弹窗点「允许」，回试打框前确认开关是绿的；软键盘不出来时 `xcrun simctl shutdown/boot` 重启设备再试；点 App 里的试打框后要等 1–2 秒键盘才出；剪贴板有字时才有「记一笔」。
Expected: 9 张截图都在 `cloud/ios/build/screenshots/`（`build/` 已在 `.gitignore`），结构与 01 的 1a–1f、05 的 2c 对得上（结构一致即可，视觉打磨留给子项目 3）。

- [ ] **Step 11: 提交**

```bash
cd /Users/liyuqing/sproot/qingjian-mainline
git add cloud/ios/Shared/Memory cloud/ios/Keyboard/Sources
git commit -m "feat(cloud): iOS 键盘加场景牌子、记忆提示行、对象卡与「记一笔」

候选栏左侧牌子显示当前场景（恋爱时是对象名），点开在键区换成场景 / 对象选择；对象只能自己切，名单读 App Group 里的 memory/。
恋爱场景选了对象时候选栏上方一直有提示行（没提示时空着），键盘高度只在进出这个状态时变，宿主界面不跟着提示跳；
日子 / 约定提醒是「知道了」，匹配提示「展开」成对象卡；换输入框时经 qj_reset_context 清掉最近上屏的字。
没开完全访问时牌子只提示开启完全访问，不让切场景。
「记一笔」只在开了完全访问、剪贴板有字、选了对象时出现，确认后经 qj_memory_note 建卡。
记忆的模型放 Shared/Memory，App 下一步共用。

```

## Task 6：iOS App「键盘记住的事」

**Files:** Create `cloud/ios/App/Memory/*`（见文件结构）；Modify `SetupView.swift`（入口改为首页 Tab：记住的 / 本周 / 我，「我」里放原有的「键盘设置」「账号」）、`project.yml`；Create `Tests/MemoryStoreTests.swift`。

- 首页（05 的 2i）：标题「键盘记住的事」、副标题「都是你写的 · 只存在这台手机上」、今日提醒卡、「人 · n / 8」列表、「加一个人」、底部灰底小卡「懒得自己写？」→ 静态说明页。
- 对象详情（02 的 1b，去掉「待确认」分组）：按 日子 / 约定 / 喜好 / 近况 / 其他 分组；右上「设置」。
- `CardEditor`（02 的 1c）：文字、是什么（五选一）、到哪天（日子与约定才显示）、关键词（逗号分隔，可空；审计修订后改为一个一个加、最多 8 个，见差异 7）、删掉这条。
- `ContactEditor`（05 的 2g）：名字或代号、称呼四选一（缺省 TA）、可选的已知的事（生日、喜欢 / 不喜欢、再写一条）。
- `ContactSettingsView`（02 的 1d）：名字、称呼、打字时提示开关、日子提醒开关、导出为文本（分享面板）、忘掉这个人（二次确认，文案「{名字}的所有记忆会从这台手机上删除，无法恢复」）。
- `WeekView`：7 天内的日子与约定，按日期排。
- `MemoryStore.swift` 经 `qj_memory_read` / `qj_memory_write` 读写；写失败按 `code` 提示（`contact_limit`→「恋爱场景最多 8 个人」）。目录设 `FileProtectionType.completeUntilFirstUserAuthentication`（开机后第一次解锁前读不了；用 `complete` 的话锁屏时键盘读不到卡片，锁屏通知里直接回复时提示行会失效）。
- 品牌：App 显示名「素笺」，图标用 `brand-sujian/icon/ios-1024-*.png`（亮、暗、着色三套进 Asset Catalog）。

测试（`MemoryStoreTests`）：JSON 解码往返；`contact_limit` 映射；称呼文案四种。验收：模拟器截图对照 02 的 1a–1d、05 的 2g、2i。

### 与大纲的差异

1. **多一个 `App/Memory/CloudIntroView.swift`**：「懒得自己写？」指向的静态说明页（一个视图一个文件）。
2. **模型在 `Shared/Memory/`（Task 5 已建）**，App 侧只有 `MemoryStore` 与页面。
3. **两个提示开关按人设置**（决定点 3）：`ContactSettingsView` 的「打字时提示」「日子提醒」写到这个人的 `hintOn` / `remindOn` 上，脚注「只对 {名字} 生效」。
4. **新建对象的「生日」就叫「生日」**（决定点 2：日子按年重复，填出生日期也会每年提醒）。
5. **图标源文件在主检出 `/Users/liyuqing/sproot/qingjian/brand-sujian/icon/`**（本检出没有 `brand-sujian/`），拷进 `cloud/ios/App/Assets.xcassets/AppIcon.appiconset/`；`project.yml` 只加显示名与 `ASSETCATALOG_COMPILER_APPICON_NAME`，`.xcassets` 在 `App/` 目录里会被自动收录。
6. **键盘扩展的显示名也改成「素笺」**（决定点 7）：`project.yml` 两个 target 的 `CFBundleDisplayName`、xcodegen 重写的两份 `Info.plist`、App 里「添加新键盘…选『素笺』」都同步改。
7. **审计修订（阻断项的 App 一侧、建议）：**
   - **写回冲突：** `MemoryStore.update` 遇到 `conflict` 就重读，用新文件 `App/Memory/MemoryMerge.swift` 三方合并（base = 改之前读的快照，local = 改完的，remote = 重读的）后再写，最多三轮；合并以 id 为键：local 新加的加、local 删掉的删、local 改过的用 local 的，其余照 remote（键盘新记的卡就这样留下）；同一张卡两边都改了 `touchedAt` 新的赢；对象两边都改了用 local；修订号用 remote 的。写成功后重读一遍拿新的修订号。
   - **回到前台重读：** `SetupView` 观察 `scenePhase`，变成 `.active` 时 `memory.reload()`。
   - **数据保护：** `protect()` 第一次（目录还不是 `.completeUntilFirstUserAuthentication`）对整个 `memory/` 递归设一遍，之后新建的文件继承目录属性。
   - 截图验收补 02 的 1a（首页）与写回冲突、按人关提示两项。
   - **手写卡上限（卡片契约）：** `CardEditor` 文字超过 200 字截掉、下面显示「137 / 200」；关键词改成一个一个加（每个 2–8 字），满 8 个「添加」不可点、标题显示「关键词 n / 8」；`ContactEditor` 生成的卡也截到 200 字。纯函数在 `MemoryLimits`（Task 5），单测 `testCardLimitsCountUnicodeScalars`；`MemoryCard` 带 `faded` / `seq` / `updatedAt` 往返不丢（`testCardKeepsCloudFieldsOnRoundTrip`）。

8. **执行时补的（2026-10-04，审计会话确认）：**
   - 首页「今天」与「本周」跳过关了「日子提醒」的人（`MemoryStore.upcoming` 只看 `remindOn` 为真的人）；对象设置里「日子提醒」下面一行小字「关掉后，今天和本周里不再提{称呼}的日子」，称呼用 `MemoryPronoun.label`。
   - 读写不占主线程：`MemoryWorker` 是跑在自己串行队列上的 actor，同一时刻只有一个读或写；保存中界面显示「正在保存」、按钮置灰，再提交的保存直接拒掉；结果回主线程再更新界面、弹提示。启动时读一次，从后台回到前台再读（不在 inactive → active 时重复读）。
   - 任何读写失败都给中文提示：容器拿不到、读不出时首页、详情、设置页顶上常驻原因；写失败弹「没存上：原因」；冲突合并时重读到坏文件也提示已备份；「忘掉」只做了一半（目录删了、名单没写进去）时提示再点一次。
   - 「忘掉」用 `.alert`（忘掉 / 再想想），等确认框收起后再写，免得失败提示撞上收起动画弹不出来。
   - 每次保存记一条系统日志（subsystem `app.qingjian.cloud`，category `memory`，public）：序号、写入前后的修订号、写之前锁是否被占、写入耗时（含等锁）、冲突轮数、成败。Debug 包的对象详情有「连续保存 20 次」给真机验并发。
   - App 的开关照设计稿 `.toggle` 用 `Theme.accent`，是「控件中性色」的例外（那条只管键盘面板），记在 `ColorUsage.appToggle`。

### 步骤

**Files:**
- Create: `cloud/ios/App/Memory/{MemoryStore,MemoryMerge,MemoryHomeView,WeekView,ContactDetailView,CardEditor,ContactEditor,ContactSettingsView,CloudIntroView}.swift`
- Create: `cloud/ios/App/Assets.xcassets/Contents.json`、`cloud/ios/App/Assets.xcassets/AppIcon.appiconset/{Contents.json,ios-1024-light.png,ios-1024-dark.png,ios-1024-tinted.png}`
- Create: `cloud/ios/Tests/MemoryStoreTests.swift`
- Modify: `cloud/ios/App/SetupView.swift`（整份替换）、`cloud/ios/project.yml:35-41,66`、`cloud/ios/App/Info.plist`、`cloud/ios/Keyboard/Info.plist`（xcodegen 重写）

- [ ] **Step 1: 写失败的测试**

Create `cloud/ios/Tests/MemoryStoreTests.swift`：

```swift
// 记忆的 JSON 往返（含旧文件缺字段）、桥的失败码映射与「NULL 即成功」的约定、称呼与按种类的提醒文案、日子按年重复、
// 写回冲突时的三方合并，以及 App 侧的「今天 / 本周」挑卡。

import Foundation
import XCTest
@testable import QingjianCloud

@MainActor
final class MemoryStoreTests: XCTestCase {
    private let contactId = "0123456789abcdef0123456789abcdef"

    private var sample: String {
        """
        {"contacts":[{"id":"\(contactId)","name":"小美","pronoun":"ta_f","scene":"dating","created_at":1791043200,
           "hint_on":true,"remind_on":false}],
         "cards":{"\(contactId)":[{"id":"fedcba9876543210fedcba9876543210","kind":"date","text":"生日","keywords":["生日"],
           "when":"2026-10-05","source":"manual","confirmed":true,"created_at":1791043200,"touched_at":1791043200}]},
         "revs":{"\(contactId)":3},
         "state":{"scene":"dating","contact_id":"\(contactId)"},
         "broken":[]}
        """
    }

    private func card(_ id: String, _ text: String, touched: Int64) -> MemoryCard {
        MemoryCard(
            id: id, kind: .other, text: text, keywords: [], when: nil, source: "manual", confirmed: true,
            createdAt: 0, touchedAt: touched)
    }

    private func person(_ name: String = "小美") -> MemoryContact {
        MemoryContact(id: contactId, name: name, pronoun: .ta, scene: MemoryScope.dating, createdAt: 0)
    }

    func testSnapshotRoundTrips() throws {
        let decoded = try JSONDecoder().decode(MemorySnapshot.self, from: Data(sample.utf8))
        XCTAssertEqual(decoded.contacts.first?.pronoun, .taF)
        XCTAssertEqual(decoded.contacts.first?.createdAt, 1_791_043_200)
        XCTAssertEqual(decoded.contacts.first?.remindOn, false)
        XCTAssertEqual(decoded.cards[contactId]?.first?.kind, .date)
        XCTAssertEqual(decoded.cards[contactId]?.first?.when, "2026-10-05")
        XCTAssertEqual(decoded.revs[contactId], 3)
        XCTAssertEqual(decoded.state.contactId, contactId)
        let again = try JSONDecoder().decode(MemorySnapshot.self, from: JSONEncoder().encode(decoded))
        XCTAssertEqual(again, decoded)
    }

    func testMissingFieldsFallBackToDefaults() throws {
        let scope = try JSONDecoder().decode(MemoryScope.self, from: Data(#"{"scene":"work","contact_id":null}"#.utf8))
        XCTAssertEqual(scope.scene, MemoryScope.work)
        let old = #"{"id":"\#(contactId)","name":"小美","pronoun":"ta","scene":"dating","created_at":0}"#
        let contact = try JSONDecoder().decode(MemoryContact.self, from: Data(old.utf8))
        XCTAssertTrue(contact.hintOn && contact.remindOn, "旧文件没有两个开关时按开")
        let empty = try JSONDecoder().decode(MemorySnapshot.self, from: Data("{}".utf8))
        XCTAssertEqual(empty, MemorySnapshot())
    }

    func testHintDecodes() throws {
        let json = #"{"card_id":"fedcba9876543210fedcba9876543210","text":"明天是她的生日","reason":"today","more":true}"#
        let hint = try JSONDecoder().decode(MemoryHint.self, from: Data(json.utf8))
        XCTAssertEqual(hint.reason, .today)
        XCTAssertTrue(hint.more)
    }

    func testFailureCodes() {
        let limit = MemoryFailure.decode(#"{"code":"contact_limit","message":"whatever"}"#)
        XCTAssertEqual(limit?.code, .contactLimit)
        XCTAssertEqual(limit?.userMessage, "恋爱场景最多 8 个人")
        XCTAssertEqual(MemoryFailure.decode(#"{"code":"conflict","message":"x"}"#)?.code, .conflict)
        let io = MemoryFailure.decode(#"{"code":"io","message":"记忆文件读写不了"}"#)
        XCTAssertEqual(io?.userMessage, "记忆文件读写不了")
        XCTAssertEqual(MemoryFailure.decode(#"{"code":"new_code","message":"x"}"#)?.code, .other)
        XCTAssertEqual(MemoryFailure.decode("不是 JSON")?.message, "不是 JSON")
    }

    /// 桥的写操作（qj_memory_note / qj_memory_write）成功时返回 NULL，Swift 侧拿到 nil 就是成功。
    func testNoteNullMeansSuccess() {
        XCTAssertNil(MemoryFailure.decode(nil))
    }

    func testPronounLabels() {
        XCTAssertEqual(MemoryPronoun.ta.label(name: "小美"), "TA")
        XCTAssertEqual(MemoryPronoun.taM.label(name: "小美"), "他")
        XCTAssertEqual(MemoryPronoun.taF.label(name: "小美"), "她")
        XCTAssertEqual(MemoryPronoun.name.label(name: "小美"), "小美")
    }

    func testReminderTextMatchesBridgeTemplates() {
        var contact = person()
        contact.pronoun = .taF
        let birthday = MemoryCard.new(kind: .date, text: "生日", when: nil, keywords: [])
        XCTAssertEqual(birthday.reminderText(days: 0, contact: contact), "今天是她的生日")
        XCTAssertEqual(birthday.reminderText(days: 1, contact: contact), "明天是她的生日")
        XCTAssertEqual(birthday.reminderText(days: 3, contact: contact), "3 天后是她的生日")
        let movie = MemoryCard.new(kind: .promise, text: "看电影", when: nil, keywords: [])
        XCTAssertEqual(movie.reminderText(days: 1, contact: contact), "明天：看电影")
        XCTAssertEqual(movie.reminderText(days: 0, contact: contact), "今天：看电影")
    }

    func testDatesRepeatYearlyButPromisesDoNot() {
        let now = MemoryDate.parse("2026-12-30")!
        XCTAssertEqual(MemoryCard.new(kind: .date, text: "生日", when: "1998-01-02", keywords: []).daysAway(now: now), 3)
        XCTAssertEqual(MemoryCard.new(kind: .promise, text: "看电影", when: "2025-12-31", keywords: []).daysAway(now: now), -364)
        let leap = MemoryCard.new(kind: .date, text: "生日", when: "2024-02-29", keywords: [])
        XCTAssertEqual(leap.daysAway(now: MemoryDate.parse("2026-02-27")!), 1, "平年按 2 月 28 日")
        XCTAssertEqual(leap.daysAway(now: MemoryDate.parse("2028-02-28")!), 1, "闰年是 2 月 29 日")
    }

    func testUpcomingPicksDatesWithinRange() {
        let store = MemoryStore()
        let now = MemoryDate.parse("2026-10-04")!
        var snapshot = MemorySnapshot()
        snapshot.contacts = [person()]
        snapshot.cards[contactId] = [
            MemoryCard.new(kind: .promise, text: "看电影", when: "2026-10-09", keywords: []),
            MemoryCard.new(kind: .date, text: "生日", when: "1998-10-05", keywords: []),
            MemoryCard.new(kind: .date, text: "纪念日", when: "2026-10-12", keywords: []),
            MemoryCard.new(kind: .preference, text: "草莓", when: "2026-10-04", keywords: []),
        ]
        store.replace(with: snapshot)
        XCTAssertEqual(store.upcoming(within: 3, now: now).map(\.card.text), ["生日"])
        XCTAssertEqual(store.upcoming(within: 6, now: now).map(\.card.text), ["生日", "看电影"])
        XCTAssertEqual(store.upcoming(within: 6, now: now).first?.text, "明天是TA的生日")
    }

    func testMergeKeepsKeyboardNotesAndAppEdits() {
        var base = MemorySnapshot()
        base.contacts = [person()]
        base.cards[contactId] = [card("a", "原来的", touched: 1), card("b", "要删的", touched: 1)]
        var local = base
        local.cards[contactId] = [card("a", "App 改的", touched: 5), card("c", "App 新加的", touched: 5)]
        var remote = base
        remote.cards[contactId] = base.cards[contactId]! + [card("k", "键盘记的", touched: 3)]
        remote.revs[contactId] = 4
        let merged = MemoryMerge.merge(base: base, local: local, remote: remote)
        XCTAssertEqual(merged.cards[contactId]?.map(\.text), ["App 改的", "键盘记的", "App 新加的"])
        XCTAssertEqual(merged.revs[contactId], 4, "修订号用重读的")
    }

    func testMergeBothChangedNewerWinsAndContactsUnion() {
        var base = MemorySnapshot()
        base.contacts = [person()]
        base.cards[contactId] = [card("a", "原来的", touched: 1)]
        var local = base
        local.cards[contactId] = [card("a", "App 改的", touched: 2)]
        let other = MemoryContact(id: "11111111111111111111111111111111", name: "阿杰", pronoun: .taM, scene: MemoryScope.dating, createdAt: 0)
        local.contacts.append(other)
        var remote = base
        remote.cards[contactId] = [card("a", "键盘那边更新的", touched: 9)]
        let merged = MemoryMerge.merge(base: base, local: local, remote: remote)
        XCTAssertEqual(merged.cards[contactId]?.first?.text, "键盘那边更新的", "两边都改了，touchedAt 新的赢")
        XCTAssertEqual(merged.contacts.map(\.name), ["小美", "阿杰"])
        var forgot = base
        forgot.contacts = []
        forgot.cards = [:]
        XCTAssertTrue(MemoryMerge.merge(base: base, local: forgot, remote: remote).contacts.isEmpty, "App 删掉的人照样删")
    }

    func testCardLimitsCountUnicodeScalars() {
        XCTAssertEqual(MemoryLimits.clampText(String(repeating: "字", count: 200)).count, 200)
        XCTAssertEqual(MemoryLimits.count(MemoryLimits.clampText(String(repeating: "字", count: 201))), 200)
        XCTAssertEqual(MemoryLimits.count(String(repeating: "😀", count: 3)), 3, "单个 emoji 算一个")
        XCTAssertEqual(MemoryLimits.counter(String(repeating: "a", count: 137)), "137 / 200")
        let eight = (0..<8).map { "关键词\($0)" }
        XCTAssertTrue(MemoryLimits.canAdd("海边", to: Array(eight.prefix(7))))
        XCTAssertFalse(MemoryLimits.canAdd("海边", to: eight), "满 8 个不能再加")
        XCTAssertFalse(MemoryLimits.canAdd("海", to: []), "至少 2 字")
        XCTAssertFalse(MemoryLimits.canAdd("九个字的关键词呀呀", to: []), "至多 8 字")
        XCTAssertFalse(MemoryLimits.canAdd("海边", to: ["海边"]), "不重复")
    }

    func testCardKeepsCloudFieldsOnRoundTrip() throws {
        let json = #"{"id":"fedcba9876543210fedcba9876543210","kind":"other","text":"x","keywords":[],"source":"cloud","confirmed":false,"faded":true,"seq":42,"updated_at":1791043200123,"created_at":0,"touched_at":0}"#
        let card = try JSONDecoder().decode(MemoryCard.self, from: Data(json.utf8))
        XCTAssertEqual(card.seq, 42)
        XCTAssertEqual(card.updatedAt, 1_791_043_200_123)
        XCTAssertTrue(card.faded)
        XCTAssertEqual(try JSONDecoder().decode(MemoryCard.self, from: JSONEncoder().encode(card)), card)
    }

    func testMemoryIDsAreBridgeFormat() {
        let id = MemoryID.make()
        XCTAssertEqual(id.count, 32)
        XCTAssertTrue(id.allSatisfy { $0.isHexDigit && !$0.isUppercase })
    }
}
```

- [ ] **Step 2: 跑测试看它失败**

Run: `cd /Users/liyuqing/sproot/qingjian-mainline/cloud/ios && xcodegen generate && xcodebuild -scheme QingjianCloud -destination 'platform=iOS Simulator,name=iPhone 17e' test 2>&1 | grep -E "error:|TEST (SUCCEEDED|FAILED)" | head -5`
Expected: 编译失败，`error: cannot find 'MemoryStore' in scope`、`cannot find 'MemoryMerge' in scope`（其余用到的 Shared 模型在 Task 5 已有）。

- [ ] **Step 3: `MemoryStore`**

Create `cloud/ios/App/Memory/MemoryStore.swift`：

```swift
// 「键盘记住的事」的数据：经桥整份读写 App Group 里的 memory/（qj_memory_read / qj_memory_write），改一处写一次。
// 校验（恋爱场景最多 8 个人、日期格式）在桥里，写失败按 code 提示；conflict（键盘这期间「记一笔」改过）时重读、
// 用 MemoryMerge 把这次的改动合并上去再写，最多三轮。App 回到前台时重读（SetupView）。目录设数据保护 completeUntilFirstUserAuthentication：开机后第一次解锁前谁都读不了，锁屏通知里回复时键盘照常能读。

import Foundation
import Observation

@MainActor
@Observable
final class MemoryStore {
    /// 「今天」「本周」里的一条：哪个人的哪张卡、还有几天。
    struct Upcoming: Identifiable, Equatable {
        let contact: MemoryContact

        let card: MemoryCard

        let days: Int

        var id: String { card.id }

        /// 「明天是她的生日」。
        var text: String { card.reminderText(days: days, contact: contact) }

        /// 「今天」「明天」「3 天后 · 10-07」。
        var dayLabel: String {
            switch days {
            case 0: return "今天"
            case 1: return "明天"
            default: return "\(days) 天后 · \(card.when?.suffix(5) ?? "")"
            }
        }
    }

    static let contactLimit = 8

    private(set) var snapshot = MemorySnapshot()

    /// 要弹给用户的话（写失败、文件损坏）。
    var message: String?

    /// 记忆所在的学习数据目录（App Group 的 Qingjian/）；测试里换成临时目录。
    @ObservationIgnored var directoryProvider: () -> URL? = { SharedStore.directory }

    /// 恋爱场景的人。
    var people: [MemoryContact] { snapshot.contacts.filter { $0.scene == MemoryScope.dating } }

    func contact(_ id: String) -> MemoryContact? { snapshot.contacts.first { $0.id == id } }

    func cards(of id: String) -> [MemoryCard] { snapshot.cards[id] ?? [] }

    func reload() {
        guard let directory = directoryProvider() else {
            message = "这个安装包没有开通 App Group，记忆用不了"
            return
        }
        Self.protect(directory.appendingPathComponent("memory", isDirectory: true))
        guard let next = MemoryFiles.read(userDirectory: directory) else {
            message = "记忆读不出来"
            return
        }
        snapshot = next
        if !next.broken.isEmpty { message = "这个人的记忆文件损坏，已备份" }
    }

    /// 测试用：不经桥直接换数据。
    func replace(with snapshot: MemorySnapshot) {
        self.snapshot = snapshot
    }

    /// 改一份再整份写回。冲突时重读、三方合并后再写（最多三轮）；写成功后重读一遍拿新的修订号。
    /// 别的失败不改内存里的，弹原因。
    @discardableResult
    func update(_ change: (inout MemorySnapshot) -> Void) -> Bool {
        guard let directory = directoryProvider() else { return false }
        var base = snapshot
        var next = snapshot
        change(&next)
        for _ in 0..<3 {
            next.broken = []
            guard let failure = MemoryFiles.write(next, userDirectory: directory) else {
                snapshot = next
                reload()
                return true
            }
            guard failure.code == .conflict, let remote = MemoryFiles.read(userDirectory: directory) else {
                message = failure.userMessage
                return false
            }
            next = MemoryMerge.merge(base: base, local: next, remote: remote)
            base = remote
        }
        message = "记忆刚在键盘里改过好几次，请稍后再试"
        return false
    }

    @discardableResult
    func addContact(_ contact: MemoryContact, cards: [MemoryCard]) -> Bool {
        update {
            $0.contacts.append(contact)
            $0.cards[contact.id] = cards
        }
    }

    @discardableResult
    func saveContact(_ contact: MemoryContact) -> Bool {
        update { snapshot in
            if let index = snapshot.contacts.firstIndex(where: { $0.id == contact.id }) {
                snapshot.contacts[index] = contact
            }
        }
    }

    /// 忘掉这个人：从名单去掉，桥连目录（卡片与分区学习）一起删。
    func forget(_ id: String) {
        update {
            $0.contacts.removeAll { $0.id == id }
            $0.cards[id] = nil
        }
    }

    @discardableResult
    func saveCard(_ card: MemoryCard, for id: String) -> Bool {
        update { snapshot in
            var list = snapshot.cards[id] ?? []
            if let index = list.firstIndex(where: { $0.id == card.id }) {
                list[index] = card
            } else {
                list.append(card)
            }
            snapshot.cards[id] = list
        }
    }

    func deleteCard(_ cardId: String, for id: String) {
        update { $0.cards[id]?.removeAll { $0.id == cardId } }
    }

    /// 所有人今天到 `within` 天后的日子与约定，近的在前。
    func upcoming(within days: Int, now: Date = Date()) -> [Upcoming] {
        people.flatMap { contact in
            cards(of: contact.id).compactMap { card -> Upcoming? in
                guard let away = card.daysAway(now: now), (0...days).contains(away) else { return nil }
                return Upcoming(contact: contact, card: card, days: away)
            }
        }
        .sorted { ($0.days, $0.card.text) < ($1.days, $1.card.text) }
    }

    /// 导出为文本：名字、认识几天，按类分组的卡片。
    func exportText(_ id: String) -> String {
        guard let contact = contact(id) else { return "" }
        var lines = ["\(contact.name)（认识 \(contact.knownDays()) 天）"]
        for kind in MemoryCard.Kind.allCases {
            let list = cards(of: id).filter { $0.kind == kind }
            guard !list.isEmpty else { continue }
            lines.append("")
            lines.append(kind.title)
            for card in list {
                lines.append("· \(card.text)" + (card.when.map { " \($0)" } ?? ""))
            }
        }
        return lines.joined(separator: "\n")
    }

    /// 记忆目录：开机后第一次解锁前谁都读不了（不用 complete：锁屏通知里回复时键盘要读卡片）。第一次（目录还不是这一档时）把整个目录树递归设一遍，
    /// 之后新建的文件继承目录的属性，不用每次设。
    private static func protect(_ directory: URL) {
        let manager = FileManager.default
        try? manager.createDirectory(at: directory, withIntermediateDirectories: true)
        let current = (try? manager.attributesOfItem(atPath: directory.path))?[.protectionKey] as? FileProtectionType
        guard current != .completeUntilFirstUserAuthentication else { return }
        let items = manager.enumerator(at: directory, includingPropertiesForKeys: nil)?
            .compactMap { $0 as? URL } ?? []
        for url in [directory] + items {
            try? manager.setAttributes([.protectionKey: FileProtectionType.completeUntilFirstUserAuthentication], ofItemAtPath: url.path)
        }
    }
}
```

Create `cloud/ios/App/Memory/MemoryMerge.swift`：

```swift
// App 写回遇到 conflict（键盘这期间「记一笔」改过）时的三方合并：base 是 App 改之前读的快照，local 是 App 改完的，
// remote 是刚重读的。对象与卡片都以 id 为键：local 新加的加上、local 删掉的删掉、local 改过的用 local 的，其余照 remote
// （键盘新记的卡就这样留下来）；同一张卡两边都改了，touchedAt 新的赢（一样新用 local）；对象两边都改了用 local。修订号用 remote 的。

enum MemoryMerge {
    static func merge(base: MemorySnapshot, local: MemorySnapshot, remote: MemorySnapshot) -> MemorySnapshot {
        var result = remote
        result.broken = []
        result.contacts = mergeList(base: base.contacts, local: local.contacts, remote: remote.contacts) { mine, _ in mine }
        result.cards = [:]
        for contact in result.contacts {
            let id = contact.id
            result.cards[id] = mergeList(
                base: base.cards[id] ?? [], local: local.cards[id] ?? [], remote: remote.cards[id] ?? []
            ) { mine, theirs in mine.touchedAt >= theirs.touchedAt ? mine : theirs }
        }
        return result
    }

    /// 以 id 为键的三方合并，顺序按 remote，local 新加的接在后面。
    private static func mergeList<T: Identifiable & Equatable>(
        base: [T], local: [T], remote: [T], bothChanged: (T, T) -> T
    ) -> [T] where T.ID == String {
        let original = Dictionary(base.map { ($0.id, $0) }, uniquingKeysWith: { first, _ in first })
        let mine = Dictionary(local.map { ($0.id, $0) }, uniquingKeysWith: { first, _ in first })
        var result: [T] = []
        for theirs in remote {
            switch (original[theirs.id], mine[theirs.id]) {
            case (.some, nil):
                continue
            case (.some(let before), .some(let edited)) where edited != before:
                result.append(theirs == before ? edited : bothChanged(edited, theirs))
            case (nil, .some(let edited)):
                result.append(theirs == edited ? edited : bothChanged(edited, theirs))
            default:
                result.append(theirs)
            }
        }
        let remoteIDs = Set(remote.map(\.id))
        result += local.filter { original[$0.id] == nil && !remoteIDs.contains($0.id) }
        return result
    }
}
```

- [ ] **Step 4: 跑测试看它通过**

Run: `cd /Users/liyuqing/sproot/qingjian-mainline/cloud/ios && xcodebuild -scheme QingjianCloud -destination 'platform=iOS Simulator,name=iPhone 17e' test 2>&1 | grep -E "MemoryStoreTests|Executed|TEST (SUCCEEDED|FAILED)" | tail -12`
Expected: `MemoryStoreTests` 14 个 `passed`，`** TEST SUCCEEDED **`。

- [ ] **Step 5: 首页、本周、说明页**

Create `cloud/ios/App/Memory/MemoryHomeView.swift`：

```swift
// 「键盘记住的事」首页（05 的 2i）：今天的提醒、恋爱场景的人（n / 8）、加一个人、「懒得自己写？」。

import SwiftUI

struct MemoryHomeView: View {
    let store: MemoryStore

    @State private var addingContact = false

    var body: some View {
        NavigationStack {
            List {
                Section {
                    ForEach(store.upcoming(within: 3)) { item in
                        NavigationLink(value: item.contact.id) {
                            VStack(alignment: .leading, spacing: 2) {
                                Text(item.text)
                                Text(item.contact.name).font(.caption).foregroundStyle(.secondary)
                            }
                        }
                    }
                } header: {
                    Text("都是你写的 · 只存在这台手机上")
                } footer: {
                    if store.upcoming(within: 3).isEmpty { Text("3 天内没有要记着的日子") }
                }
                Section {
                    ForEach(store.people) { contact in
                        NavigationLink(value: contact.id) {
                            HStack(spacing: 12) {
                                MemoryAvatar(name: contact.name, size: 36)
                                VStack(alignment: .leading, spacing: 2) {
                                    Text(contact.name)
                                    Text("\(store.cards(of: contact.id).count) 条 · 认识 \(contact.knownDays()) 天")
                                        .font(.caption)
                                        .foregroundStyle(.secondary)
                                }
                            }
                        }
                    }
                    if store.people.count < MemoryStore.contactLimit {
                        Button {
                            addingContact = true
                        } label: {
                            Label("加一个人", systemImage: "plus")
                        }
                    }
                } header: {
                    Text("人 · \(store.people.count) / \(MemoryStore.contactLimit)")
                }
                Section {
                    NavigationLink {
                        CloudIntroView()
                    } label: {
                        VStack(alignment: .leading, spacing: 4) {
                            Text("懒得自己写？").font(.subheadline.weight(.medium))
                            Text("以后可以让素笺从你发出的话里自动整理").font(.caption).foregroundStyle(.secondary)
                        }
                    }
                    .listRowBackground(Color(.secondarySystemBackground))
                }
            }
            .navigationTitle("键盘记住的事")
            .navigationDestination(for: String.self) { id in
                ContactDetailView(store: store, contactId: id)
            }
            .sheet(isPresented: $addingContact) { ContactEditor(store: store) }
            .task { store.reload() }
            .alert(
                store.message ?? "",
                isPresented: Binding(get: { store.message != nil }, set: { if !$0 { store.message = nil } })
            ) {
                Button("好", role: .cancel) {}
            }
        }
    }
}
```

Create `cloud/ios/App/Memory/WeekView.swift`：

```swift
// 「本周」：所有人 7 天内（今天到 6 天后）的日子与约定，按日期排。

import SwiftUI

struct WeekView: View {
    let store: MemoryStore

    var body: some View {
        NavigationStack {
            List {
                let items = store.upcoming(within: 6)
                if items.isEmpty {
                    Text("这 7 天没有记下的日子和约定").foregroundStyle(.secondary)
                }
                ForEach(items) { item in
                    HStack {
                        VStack(alignment: .leading, spacing: 2) {
                            Text(item.card.text)
                            Text("\(item.contact.name) · \(item.card.kind.title)")
                                .font(.caption)
                                .foregroundStyle(.secondary)
                        }
                        Spacer()
                        Text(item.dayLabel).font(.caption).foregroundStyle(.secondary)
                    }
                }
            }
            .navigationTitle("本周")
            .task { store.reload() }
        }
    }
}
```

Create `cloud/ios/App/Memory/CloudIntroView.swift`：

```swift
// 「懒得自己写？」点进来的说明页：云端记忆还没上线，这里只说明它会是什么样、现在的记忆在哪。

import SwiftUI

struct CloudIntroView: View {
    var body: some View {
        List {
            Section("现在") {
                Text("「键盘记住的事」全部是你自己写的，只存在这台手机上，不联网、不用登录。")
            }
            Section("以后") {
                Text("开了云端记忆后，素笺会把你在选定场景里发出的话去掉手机号、地址这类信息后上传，每天整理成记忆卡，等你确认了才生效。")
                Text("不开就永远不会上传；开了也随时可以停。")
            }
        }
        .navigationTitle("云端记忆")
        .navigationBarTitleDisplayMode(.inline)
    }
}
```

- [ ] **Step 6: 对象详情与卡片编辑**

Create `cloud/ios/App/Memory/ContactDetailView.swift`：

```swift
// 对象详情（02 的 1b，没有「待确认」）：头像字、名字、认识几天，卡片按日子 / 约定 / 喜好 / 近况 / 其他分组；右上「设置」。

import SwiftUI

struct ContactDetailView: View {
    let store: MemoryStore

    let contactId: String

    @Environment(\.dismiss) private var dismiss

    @State private var editing: MemoryCard?

    @State private var adding = false

    var body: some View {
        List {
            if let contact = store.contact(contactId) {
                Section {
                    HStack(spacing: 14) {
                        MemoryAvatar(name: contact.name, size: 56)
                        VStack(alignment: .leading, spacing: 4) {
                            Text(contact.name).font(.title2.weight(.semibold))
                            Text("认识 \(contact.knownDays()) 天").foregroundStyle(.secondary)
                        }
                    }
                }
                ForEach(MemoryCard.Kind.allCases, id: \.self) { kind in
                    let cards = store.cards(of: contactId).filter { $0.kind == kind }
                    if !cards.isEmpty {
                        Section(kind.title) {
                            ForEach(cards) { card in
                                Button {
                                    editing = card
                                } label: {
                                    VStack(alignment: .leading, spacing: 2) {
                                        Text(card.text).foregroundStyle(.primary)
                                        if let when = card.when {
                                            Text(when).font(.caption).foregroundStyle(.secondary)
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                Section {
                    Button {
                        adding = true
                    } label: {
                        Label("记一条", systemImage: "plus")
                    }
                }
            }
        }
        .navigationTitle(store.contact(contactId)?.name ?? "")
        .navigationBarTitleDisplayMode(.inline)
        .toolbar {
            ToolbarItem(placement: .topBarTrailing) {
                NavigationLink("设置") { ContactSettingsView(store: store, contactId: contactId) }
            }
        }
        .sheet(item: $editing) { card in CardEditor(store: store, contactId: contactId, card: card) }
        .sheet(isPresented: $adding) { CardEditor(store: store, contactId: contactId, card: nil) }
        // 在设置页里忘掉了这个人：回到首页
        .onChange(of: store.contact(contactId) == nil) { _, gone in
            if gone { dismiss() }
        }
    }
}
```

Create `cloud/ios/App/Memory/CardEditor.swift`：

```swift
// 新建 / 编辑一张卡（02 的 1c）：写下来（最多 200 字，带计数）、是什么（五选一）、到哪天（日子与约定才有）、
// 关键词（一个一个加，最多 8 个、每个 2–8 字，满了「添加」不可点）、删掉这条。上限见 MemoryLimits，桥写入前也会校验。

import SwiftUI

struct CardEditor: View {
    let store: MemoryStore

    let contactId: String

    /// nil 是新建。
    let card: MemoryCard?

    @Environment(\.dismiss) private var dismiss

    @State private var text = ""

    @State private var kind = MemoryCard.Kind.other

    @State private var when = Date()

    @State private var keywords: [String] = []

    @State private var newKeyword = ""

    var body: some View {
        NavigationStack {
            Form {
                Section {
                    TextField("比如：她不吃香菜", text: $text, axis: .vertical)
                        .lineLimit(2...5)
                        .onChange(of: text) { _, value in
                            let clamped = MemoryLimits.clampText(value)
                            if clamped != value { text = clamped }
                        }
                } header: {
                    Text("写下来")
                } footer: {
                    HStack {
                        Spacer()
                        Text(MemoryLimits.counter(text)).monospacedDigit()
                    }
                }
                Section("是什么") {
                    Picker("是什么", selection: $kind) {
                        ForEach(MemoryCard.Kind.allCases, id: \.self) { Text($0.title).tag($0) }
                    }
                    .pickerStyle(.segmented)
                }
                if kind.hasDate {
                    Section("到哪天") {
                        DatePicker("日期", selection: $when, displayedComponents: .date)
                            .environment(\.timeZone, MemoryDate.timeZone)
                    }
                }
                Section {
                    ForEach(keywords, id: \.self) { keyword in Text(keyword) }
                        .onDelete { keywords.remove(atOffsets: $0) }
                    HStack {
                        TextField("2–8 个字", text: $newKeyword)
                        Button("添加") {
                            keywords.append(newKeyword.trimmingCharacters(in: .whitespacesAndNewlines))
                            newKeyword = ""
                        }
                        .disabled(!MemoryLimits.canAdd(newKeyword, to: keywords))
                    }
                } header: {
                    Text("关键词 \(keywords.count) / \(MemoryLimits.maxKeywords)")
                } footer: {
                    Text("打字时出现这些词，键盘会提示这一条；不写就按这条的内容自动找。")
                }
                if let card {
                    Section {
                        Button("删掉这条", role: .destructive) {
                            store.deleteCard(card.id, for: contactId)
                            dismiss()
                        }
                    }
                }
            }
            .navigationTitle(card == nil ? "记一条" : "改一条")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) { Button("取消") { dismiss() } }
                ToolbarItem(placement: .confirmationAction) {
                    Button("完成") { save() }
                        .disabled(text.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
                }
            }
            .onAppear(perform: load)
        }
    }

    private func load() {
        guard let card else { return }
        text = card.text
        kind = card.kind
        when = card.when.flatMap(MemoryDate.parse) ?? Date()
        keywords = card.keywords
    }

    private func save() {
        var next = card ?? MemoryCard.new(kind: kind, text: "", when: nil, keywords: [])
        next.kind = kind
        next.text = MemoryLimits.clampText(text.trimmingCharacters(in: .whitespacesAndNewlines))
        next.when = kind.hasDate ? MemoryDate.format(when) : nil
        next.keywords = keywords
        next.touchedAt = Int64(Date().timeIntervalSince1970)
        if store.saveCard(next, for: contactId) { dismiss() }
    }
}
```

- [ ] **Step 7: 新建对象与对象设置**

Create `cloud/ios/App/Memory/ContactEditor.swift`：

```swift
// 加一个人（05 的 2g）：名字或代号、称呼（他 / 她 / TA / 直接用名字，缺省 TA）、可以不填的几条已知的事。

import SwiftUI

struct ContactEditor: View {
    let store: MemoryStore

    @Environment(\.dismiss) private var dismiss

    @State private var name = ""

    @State private var pronoun = MemoryPronoun.ta

    @State private var hasBirthday = false

    @State private var birthday = Date()

    @State private var likes = ""

    @State private var dislikes = ""

    @State private var extra = ""

    var body: some View {
        NavigationStack {
            Form {
                Section("名字或代号") {
                    TextField("只存在这台手机上", text: $name)
                }
                Section("称呼") {
                    Picker("称呼", selection: $pronoun) {
                        ForEach(MemoryPronoun.choices, id: \.self) { Text($0.title).tag($0) }
                    }
                    .pickerStyle(.segmented)
                }
                Section {
                    Toggle("生日", isOn: $hasBirthday)
                    if hasBirthday {
                        DatePicker("日期", selection: $birthday, displayedComponents: .date)
                            .environment(\.timeZone, MemoryDate.timeZone)
                    }
                    TextField("喜欢", text: $likes)
                    TextField("不喜欢", text: $dislikes)
                    TextField("再写一条", text: $extra)
                } header: {
                    Text("已经知道的事（可以不填）")
                }
            }
            .navigationTitle("加一个人")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) { Button("取消") { dismiss() } }
                ToolbarItem(placement: .confirmationAction) {
                    Button("完成") { save() }.disabled(trimmed(name).isEmpty)
                }
            }
        }
    }

    private func trimmed(_ text: String) -> String {
        text.trimmingCharacters(in: .whitespacesAndNewlines)
    }

    private func save() {
        let contact = MemoryContact.new(name: trimmed(name), pronoun: pronoun)
        var cards: [MemoryCard] = []
        if hasBirthday {
            cards.append(.new(kind: .date, text: "生日", when: MemoryDate.format(birthday), keywords: ["生日"]))
        }
        if !trimmed(likes).isEmpty {
            cards.append(.new(kind: .preference, text: MemoryLimits.clampText("喜欢\(trimmed(likes))"), when: nil, keywords: []))
        }
        if !trimmed(dislikes).isEmpty {
            cards.append(.new(kind: .preference, text: MemoryLimits.clampText("不喜欢\(trimmed(dislikes))"), when: nil, keywords: []))
        }
        if !trimmed(extra).isEmpty {
            cards.append(.new(kind: .other, text: MemoryLimits.clampText(trimmed(extra)), when: nil, keywords: []))
        }
        if store.addContact(contact, cards: cards) { dismiss() }
    }
}
```

Create `cloud/ios/App/Memory/ContactSettingsView.swift`：

```swift
// 对象设置（02 的 1d）：名字、称呼、这个人的两个提示开关、导出为文本、忘掉这个人（二次确认，桥连对象目录一起删）。

import SwiftUI

struct ContactSettingsView: View {
    let store: MemoryStore

    let contactId: String

    @Environment(\.dismiss) private var dismiss

    @State private var name = ""

    @State private var confirmingForget = false

    var body: some View {
        Form {
            if let contact = store.contact(contactId) {
                Section("名字") {
                    TextField("名字或代号", text: $name)
                        .onSubmit { saveName(contact) }
                }
                Section("称呼") {
                    Picker(
                        "称呼",
                        selection: Binding(
                            get: { contact.pronoun },
                            set: { pronoun in
                                var next = contact
                                next.pronoun = pronoun
                                store.saveContact(next)
                            })
                    ) {
                        ForEach(MemoryPronoun.choices, id: \.self) { Text($0.title).tag($0) }
                    }
                    .pickerStyle(.segmented)
                }
                Section {
                    Toggle(
                        "打字时提示",
                        isOn: Binding(
                            get: { contact.hintOn },
                            set: { on in
                                var next = contact
                                next.hintOn = on
                                store.saveContact(next)
                            }))
                    Toggle(
                        "日子提醒",
                        isOn: Binding(
                            get: { contact.remindOn },
                            set: { on in
                                var next = contact
                                next.remindOn = on
                                store.saveContact(next)
                            }))
                } footer: {
                    Text("只对\(contact.name)生效。")
                }
                Section {
                    ShareLink(item: store.exportText(contactId)) {
                        Label("导出为文本", systemImage: "square.and.arrow.up")
                    }
                }
                Section {
                    Button("忘掉这个人", role: .destructive) { confirmingForget = true }
                }
                .confirmationDialog(
                    "忘掉\(contact.name)？", isPresented: $confirmingForget, titleVisibility: .visible
                ) {
                    Button("忘掉", role: .destructive) {
                        store.forget(contactId)
                        dismiss()
                    }
                } message: {
                    Text("\(contact.name)的所有记忆会从这台手机上删除，无法恢复")
                }
                .onDisappear { saveName(contact) }
            }
        }
        .navigationTitle("设置")
        .onAppear { name = store.contact(contactId)?.name ?? "" }
    }

    private func saveName(_ contact: MemoryContact) {
        let trimmed = name.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty, trimmed != contact.name, store.contact(contactId) != nil else { return }
        var next = contact
        next.name = trimmed
        store.saveContact(next)
    }
}
```

- [ ] **Step 8: 首页 Tab 化**

Replace `cloud/ios/App/SetupView.swift`：

```swift
// 主 App 首页：三个 Tab。「记住的」与「本周」是键盘记住的事，「我」是原来的启用步骤、键盘设置与账号入口、试打框。
// 回到前台时重读记忆（键盘可能在这期间记过一笔）。

import SwiftUI
import UIKit

struct SetupView: View {
    @Environment(\.scenePhase) private var scenePhase

    @State private var draft = ""

    @State private var store = SettingsStore()

    @State private var account = AccountStore()

    @State private var memory = MemoryStore()

    var body: some View {
        TabView {
            MemoryHomeView(store: memory)
                .tabItem { Label("记住的", systemImage: "heart.text.square") }
            WeekView(store: memory)
                .tabItem { Label("本周", systemImage: "calendar") }
            me
                .tabItem { Label("我", systemImage: "person.crop.circle") }
        }
        // 回到前台时重读：App 在后台期间键盘可能记过一笔、切过对象
        .onChange(of: scenePhase) { _, phase in
            if phase == .active { memory.reload() }
        }
    }

    private var me: some View {
        NavigationStack {
            Form {
                Section("启用键盘") {
                    Label("打开「设置 → 通用 → 键盘 → 键盘」", systemImage: "1.circle")
                    Label("点「添加新键盘…」，选「素笺」", systemImage: "2.circle")
                    Label("打字时长按地球键切到素笺", systemImage: "3.circle")
                    Button("打开设置") {
                        if let url = URL(string: UIApplication.openSettingsURLString) {
                            UIApplication.shared.open(url)
                        }
                    }
                }
                Section {
                    if store.available {
                        NavigationLink("键盘设置") { KeyboardSettingsView(store: store) }
                        NavigationLink("账号") { AccountView(store: account) }
                    } else {
                        Text("这个安装包没有开通 App Group，设置改不到键盘上。").foregroundStyle(.secondary)
                    }
                } header: {
                    Text("设置")
                } footer: {
                    Text("与 Mac 版偏好设置是同一份，登录并打开同步后两边互通。")
                }
                Section {
                    TextField("在这里试打", text: $draft, axis: .vertical)
                        .lineLimit(3...8)
                } header: {
                    Text("试一试")
                } footer: {
                    Text("「完全访问」用于按键震动、键盘读你在这里记下的人与事，以及登录后连接服务器（大模型润色、剪贴板与学习数据同步）。不开也能正常打字；没登录时键盘不联网。")
                }
            }
            .navigationTitle("我")
        }
    }
}
```

- [ ] **Step 9: 图标与显示名**

Run:

```bash
cd /Users/liyuqing/sproot/qingjian-mainline/cloud/ios
mkdir -p App/Assets.xcassets/AppIcon.appiconset
cp /Users/liyuqing/sproot/qingjian/brand-sujian/icon/ios-1024-{light,dark,tinted}.png App/Assets.xcassets/AppIcon.appiconset/
cat > App/Assets.xcassets/Contents.json <<'JSON'
{
  "info" : { "author" : "xcode", "version" : 1 }
}
JSON
cat > App/Assets.xcassets/AppIcon.appiconset/Contents.json <<'JSON'
{
  "images" : [
    { "filename" : "ios-1024-light.png", "idiom" : "universal", "platform" : "ios", "size" : "1024x1024" },
    {
      "appearances" : [ { "appearance" : "luminosity", "value" : "dark" } ],
      "filename" : "ios-1024-dark.png", "idiom" : "universal", "platform" : "ios", "size" : "1024x1024"
    },
    {
      "appearances" : [ { "appearance" : "luminosity", "value" : "tinted" } ],
      "filename" : "ios-1024-tinted.png", "idiom" : "universal", "platform" : "ios", "size" : "1024x1024"
    }
  ],
  "info" : { "author" : "xcode", "version" : 1 }
}
JSON
file App/Assets.xcassets/AppIcon.appiconset/*.png
```

Expected: 三行 `PNG image data, 1024 x 1024, 8-bit/color RGB`（没有透明通道，App Store 要求）。

Modify `cloud/ios/project.yml`：`QingjianCloud` target 里

```yaml
    info:
      path: App/Info.plist
      properties:
        CFBundleDisplayName: 青简
```

的 `CFBundleDisplayName: 青简` 改成 `CFBundleDisplayName: 素笺`；`Keyboard` target 里同样的那一行（`info: path: Keyboard/Info.plist` 下，约第 66 行）也改成 `CFBundleDisplayName: 素笺`（决定点 7：键盘与容器 App 都叫素笺）；同一 `QingjianCloud` target 的

```yaml
    settings:
      base:
        PRODUCT_BUNDLE_IDENTIFIER: app.qingjian.cloud
```

下面加一行 `ASSETCATALOG_COMPILER_APPICON_NAME: AppIcon`（与 `PRODUCT_BUNDLE_IDENTIFIER` 同缩进）。

`App/Info.plist` 与 `Keyboard/Info.plist` 在仓库里，由 xcodegen 按 `project.yml` 的 `info.properties` 重写：跑完 `xcodegen generate` 后确认两份里都是 `<key>CFBundleDisplayName</key>` 下一行 `<string>素笺</string>`，一起提交。

Run: `cd /Users/liyuqing/sproot/qingjian-mainline/cloud/ios && xcodegen generate && grep -A1 CFBundleDisplayName App/Info.plist Keyboard/Info.plist`
Expected: 两个文件都输出 `<string>素笺</string>`。

- [ ] **Step 10: 构建与全部测试**

Run: `cd /Users/liyuqing/sproot/qingjian-mainline/cloud/ios && scripts/build-bridge.sh && xcodegen generate && xcodebuild -scheme QingjianCloud -destination 'platform=iOS Simulator,name=iPhone 17e' test 2>&1 | grep -E "error:|Executed|TEST (SUCCEEDED|FAILED)" | tail -5`
Expected: 没有 `error:`，`** TEST SUCCEEDED **`（`AccountDecodeTests`、`AccountStoreTests`、`MemoryStoreTests` 全过）。

- [ ] **Step 11: 模拟器里对照截图**

```bash
cd /Users/liyuqing/sproot/qingjian-mainline/cloud/ios
app=$(find ~/Library/Developer/Xcode/DerivedData -path "*Debug-iphonesimulator/QingjianCloud.app" -maxdepth 6 | head -1)
xcrun simctl install booted "$app" && xcrun simctl launch booted app.qingjian.cloud
mkdir -p build/screenshots
```

手动操作，每步 `xcrun simctl io booted screenshot build/screenshots/<名字>.png`：

1. 主屏图标是素笺的白纸折角、名字「素笺」；设置 → 键盘 → 键盘 里的键盘也叫「素笺」→ `task6-icon.png`。
2. 首页「键盘记住的事」：副标题、今天的提醒（Task 5 放的「明天是她的生日」）、「人 · 1 / 8」、加一个人、灰底「懒得自己写？」，对照 05 的 2i 与 02 的 1a → `task6-2i-home.png`、`task6-1a-home.png`；点「懒得自己写？」看说明页。
3. 点「加一个人」：名字、称呼四选一（缺省 TA）、生日 / 喜欢 / 不喜欢 / 再写一条 → `task6-2g-new.png`；填「阿杰」、称呼「他」、喜欢「篮球」，完成后列表变「人 · 2 / 8」。
4. 点小美：按 日子 / 喜好 分组的卡片 → `task6-1b-detail.png`；点一张卡进编辑（02 的 1c）→ `task6-1c-editor.png`；把「是什么」切到「其他」时「到哪天」消失。
5. 右上「设置」（02 的 1d）→ `task6-1d-settings.png`；点「导出为文本」出分享面板；点「忘掉这个人」出二次确认，文案「小美的所有记忆会从这台手机上删除，无法恢复」→ `task6-1d-forget.png`（点取消）。
6. 「本周」Tab：7 天内的日子与约定 → `task6-week.png`。
7. 用 `printf` 往 contacts.json 再塞到 8 个恋爱场景的人后回到首页，「加一个人」消失；再手动改文件塞第 9 个、在 App 里改任意一张卡，弹「恋爱场景最多 8 个人」→ `task6-limit.png`。
8. 把某人的 `cards.json` 改成 `[{`，回到首页：弹「这个人的记忆文件损坏，已备份」，`memory/<id>/` 下有 `cards.json.broken-…` → `task6-broken.png`。
9. 「我」Tab：原来的启用步骤（「选『素笺』」）、键盘设置、账号、试打框都在 → `task6-me.png`。
10. 写回冲突：App 停在小美的详情页；去「我」Tab 的试打框用键盘「记一笔」（剪贴板放一段字，记到小美）；回到小美的详情页，改一张卡点完成：键盘记的那张还在，改动也在（App 回前台时已重读；就算没重读，写回返回 conflict 后也会重读合并）→ `task6-merge.png`；`cat "$mem/<id>/cards.json"` 看 `rev` 比改之前大 2。
11. 对象设置里关掉小美的「打字时提示」：键盘里打 `dangao` 不再出提示，「日子提醒」照常（按人设置）。

Expected: 截图都在 `cloud/ios/build/screenshots/`，结构与 02 的 1a–1d、05 的 2g、2i 对得上（视觉打磨留给子项目 3）。

- [ ] **Step 12: 提交**

```bash
cd /Users/liyuqing/sproot/qingjian-mainline
git add cloud/ios/App cloud/ios/Keyboard/Info.plist cloud/ios/Tests/MemoryStoreTests.swift cloud/ios/project.yml
git commit -m "feat(cloud): iOS App 加「键盘记住的事」，显示名改素笺

首页分三个 Tab：记住的（今天的提醒、恋爱场景的人 n / 8、加一个人、云端记忆说明）、本周（7 天内的日子与约定）、我（原来的设置与账号）。
对象详情按日子 / 约定 / 喜好 / 近况 / 其他分组，卡片与对象的增删改经 qj_memory_write 整份写回，上限与日期由桥校验，失败按 code 提示。
写回返回 conflict（键盘这期间记过一笔）时重读、以 id 为键三方合并后再写；App 回到前台时重读。提示开关按人设置。
记忆目录第一次递归设 FileProtectionType.completeUntilFirstUserAuthentication；键盘与 App 都叫素笺，图标换成素笺定稿（亮、暗、着色）。

```

## Task 7：回放调参与文档

- 用 `apps/cli` 回放一份输入日志，比较 `OVERLAY_WEIGHT` = 2 / 4 / 8 时恋爱场景对象常用词的首选命中，选定后写进常数与 `cloud/docs/design.md` 新增的「本地记忆」一节。
- 更新 `cloud/docs/design.md`、`cloud/README.md`（功能清单）；`fork-patch.md` 写明本功能没有新增上游补丁。

### 与大纲的差异

1. **不用 `apps/cli`，改在桥里写回放 example**：`apps/cli` 的 `--replay`（`apps/cli/src/replay/mod.rs`）拿的是 CLI 自己装配的 `Engine`，学习器只能是 `--user-dict` 指定文件的 `FrequencyLearner`；`--tune` 能调的只有个人 n-gram 与纠错的常数（`lambda/k/cap/discount/transpose/…`），没有叠加倍数，也装不进 `ScopedLearner`（它在 `cloud/` 的独立 workspace 里，CLI 依赖它就是上游补丁）。
   所以在 `cloud/crates/qingjian-cloud-bridge/examples/overlay_replay.rs` 里照 `apps/cli` 回放的做法（`set_input` → `query` → 看首选 → `commit` → `clear` / `break_chain`）自己跑，引擎装 `ScopedLearner::open_with_weight`（Task 1 已留的构造参数，只用于调参，不进产品配置）。
2. **样本不用输入日志，用合成的选词序列**：仓库里没有可用的恋爱场景输入日志，也不能用真实用户数据。example 内置 24 个常见拼音探针，冷启动下每个探针的首选当「通用词」、同字数的下一个候选当「对象常用词」，序列完全确定、可复现：
   日常场景每个通用词先选 W 次（全局层的底子，W 取 5 / 10 / 20 / 40 各跑一遍），切到恋爱 · 对象 A 后每轮把每个探针的对象词选一次、共 6 轮（每轮先看它是不是首选再上屏，所以第 r 轮量的是选过 r−1 次之后），最后切到对象 B 看通用词还剩多少是首选（场景层外溢）。
3. **量化指标与实测结果**：A 列是第 1–6 轮「对象词就是首选」的比例；B 列是对象 B 下「通用词仍是首选」的比例（越高说明对象之间越不串）。
   在临时副本里（不动本检出）用 `data/generated` 的产品数据跑过，24 个探针全部可用。**审阅后修订（恋爱场景不记词序列转移，决定点 5）之后重跑**的结果（封顶 88%：有三个探针的对象词在恋爱场景也压不过别的因素）：

   | W | k=2：A 第 1–6 轮，B | k=4：A 第 1–6 轮，B | k=8：A 第 1–6 轮，B |
   |---|---|---|---|
   | 5 | 0 / 0 / 0 / 88 / 88 / 88 %，B 12% | 0 / 0 / 88 / 88 / 88 / 88 %，B 12% | 0 / 88 / 88 / 88 / 88 / 88 %，B 12% |
   | 10 | 0 / 0 / 0 / 0 / 0 / 0 %，B 12% | 0 / 0 / 0 / 88 / 88 / 88 %，B 12% | 0 / 0 / 88 / 88 / 88 / 88 %，B 12% |
   | 20 | 0 / 0 / 0 / 0 / 0 / 0 %，B 100% | 0 / 0 / 0 / 0 / 0 / 0 %，B 12% | 0 / 0 / 0 / 88 / 88 / 88 %，B 12% |
   | 40 | 0 / 0 / 0 / 0 / 0 / 0 %，B 100% | 0 / 0 / 0 / 0 / 0 / 0 %，B 100% | 0 / 0 / 0 / 0 / 0 / 0 %，B 12% |

   结论：对象词在对象下选过 n 次后成为首选的条件约是 **`k·n > W`**（W 是全局里选通用词的次数）；场景层外溢到对象 B 的条件约是 `k·6 > W`。
   修订前（恋爱场景也记转移）第一次展开时测的是 `2k·n > W`：日常里记下的个人 n-gram 把通用词往前推，恋爱场景不再记转移后对象词抵不掉这一份，**同样的权重下要多选一倍次数**（W=20、k=4 时从第 4 轮变成 6 轮内都不行）。
   k=4 的含义：在一个对象下选 1 次大约抵全局 4 次；k=8 恢复到修订前 k=4 的速度。权重本身没有「对」的值。
4. **维持 spec 的 4，不改常数**（审计会话已定，文末决定点 6）；但上面的重跑说明决定点 5 让对象词变慢了一倍，是否改成 8 列进「新增的需要审计会话再决定的点」。以后要改只改 `OVERLAY_WEIGHT` 一处，测试里写的是 `K = ScopedLearner::OVERLAY_WEIGHT`，不用改测试。
5. 多改一个 `cloud/ios/README.md`（加「本地记忆」一节）：它是 iOS 壳的使用说明，功能变了不改就过时。

### 步骤

**Files:**
- Create: `cloud/crates/qingjian-cloud-bridge/examples/overlay_replay.rs`
- Modify: `cloud/docs/design.md:262`（「## 分期」之前插一节）、`cloud/README.md:3`、`cloud/ios/README.md`（「## 已知问题」之前）、`cloud/docs/fork-patch.md:4`

- [ ] **Step 1: 写回放 example**

Create `cloud/crates/qingjian-cloud-bridge/examples/overlay_replay.rs`：

```rust
//! 叠加权重回放：用合成的选词序列比较 `ScopedLearner` 的叠加倍数取 2 / 4 / 8 时，恋爱场景里对象常用词多快成为首选（对象 A），
//! 以及换到另一个对象时通用词还剩多少首选（场景层外溢，对象 B）。只在临时目录里学习，不读任何真实输入日志。
//! 用法：`cargo run --release -p qingjian-cloud-bridge --example overlay_replay -- <含 dict.qj 与 lm.qj 的目录>`

use std::path::{Path, PathBuf};

use qingjian_cloud_bridge::ScopedLearner;
use qingjian_cloud_proto::Scene;
use qingjian_core::{Candidate, Engine};
use qingjian_dictionary::Dictionary;
use qingjian_lm::BigramModel;

/// 探针：常见的、首选与次选都是常用词的拼音。冷启动下首选当「通用词」，同字数的下一个候选当「对象常用词」。
const PROBES: [&str; 24] = [
    "shishi",
    "jiyi",
    "shiyan",
    "gongshi",
    "jieshi",
    "yuanyi",
    "xiangxiang",
    "liwu",
    "shouji",
    "dianying",
    "jiankang",
    "gongzuo",
    "yiyi",
    "shijian",
    "zhuyi",
    "chengshi",
    "xinli",
    "tiqian",
    "baobei",
    "xiexie",
    "jinzhang",
    "xiaoxin",
    "shengqi",
    "wanan",
];

/// 日常场景里每个通用词先选几次（全局层的底子），各跑一遍。
const WARMUPS: [usize; 4] = [5, 10, 20, 40];

/// 恋爱场景对象 A 下把对象词选几轮。
const ROUNDS: usize = 6;

const CONTACT_A: &str = "0123456789abcdef0123456789abcdef";

const CONTACT_B: &str = "fedcba9876543210fedcba9876543210";

/// 一个探针：拼音、通用词、对象词。
type Probe = (String, String, String);

fn main() {
    let Some(data) = std::env::args().nth(1).map(PathBuf::from) else {
        eprintln!("用法：overlay_replay <含 dict.qj 与 lm.qj 的目录>");
        std::process::exit(2);
    };
    for warmup in WARMUPS {
        println!();
        println!("日常先选通用词 {warmup} 次，恋爱 · 对象 A 下选对象词 {ROUNDS} 轮，再看对象 B");
        println!("| 权重 | 探针 | A 第 1–{ROUNDS} 轮对象词首选命中 | B 下通用词仍是首选 |");
        println!("|---|---|---|---|");
        for weight in [2, 4, 8] {
            let (probes, rounds, kept) = run(&data, weight, warmup);
            let rounds: Vec<String> = rounds
                .iter()
                .map(|rate| format!("{:.0}%", rate * 100.0))
                .collect();
            println!(
                "| {weight} | {probes} | {} | {:.0}% |",
                rounds.join(" / "),
                kept * 100.0
            );
        }
    }
}

/// 返回可用的探针数、A 每轮的命中率、B 的通用词保持率。
fn run(data: &Path, weight: u32, warmup: usize) -> (usize, Vec<f64>, f64) {
    let user =
        std::env::temp_dir().join(format!("qj-overlay-replay-{weight}-{}", std::process::id()));
    std::fs::remove_dir_all(&user).ok();
    std::fs::create_dir_all(&user).expect("临时目录建不了");
    let memory = user.join("memory");
    let learner = ScopedLearner::open_with_weight(&user, &memory, Scene::Daily, None, weight);
    let handle = learner.handle();
    let dictionary = Dictionary::from_path(data.join("dict.qj")).expect("dict.qj 读不了");
    let model = BigramModel::from_path(&data.join("lm.qj")).expect("lm.qj 读不了");
    let mut engine = Engine::new(dictionary)
        .with_learner(Box::new(learner))
        .with_language_model(Box::new(model));

    let probes: Vec<Probe> = PROBES
        .iter()
        .filter_map(|keys| probe(&mut engine, keys))
        .collect();
    for _ in 0..warmup {
        for (keys, generic, _) in &probes {
            choose(&mut engine, keys, generic);
        }
    }

    handle.switch(Scene::Dating, Some(CONTACT_A));
    engine.learner_mut();
    let mut rounds = Vec::new();
    for _ in 0..ROUNDS {
        let hits = probes
            .iter()
            .filter(|(keys, _, personal)| choose(&mut engine, keys, personal))
            .count();
        rounds.push(hits as f64 / probes.len() as f64);
    }

    handle.switch(Scene::Dating, Some(CONTACT_B));
    engine.learner_mut();
    let kept = probes
        .iter()
        .filter(|(keys, generic, _)| top(&mut engine, keys).as_deref() == Some(generic.as_str()))
        .count();
    std::fs::remove_dir_all(&user).ok();
    (probes.len(), rounds, kept as f64 / probes.len() as f64)
}

/// 冷启动下的首选与同字数的下一个候选；凑不出一对就不用这个探针。
fn probe(engine: &mut Engine, keys: &str) -> Option<Probe> {
    let items = candidates(engine, keys);
    let generic = items.first()?.text.clone();
    let len = generic.chars().count();
    let personal = items
        .iter()
        .skip(1)
        .find(|candidate| candidate.text.chars().count() == len)?
        .text
        .clone();
    Some((keys.to_owned(), generic, personal))
}

fn candidates(engine: &mut Engine, keys: &str) -> Vec<Candidate> {
    engine.set_input(keys);
    let items = engine
        .query()
        .map(|query| query.candidates.items)
        .unwrap_or_default();
    engine.clear();
    items
}

fn top(engine: &mut Engine, keys: &str) -> Option<String> {
    candidates(engine, keys)
        .into_iter()
        .next()
        .map(|candidate| candidate.text)
}

/// 输入 `keys`，看 `text` 是不是首选，再把它上屏（与 `apps/cli` 回放一样，上屏后断开上文）。
fn choose(engine: &mut Engine, keys: &str, text: &str) -> bool {
    engine.set_input(keys);
    let items = engine
        .query()
        .map(|query| query.candidates.items)
        .unwrap_or_default();
    let hit = items
        .first()
        .is_some_and(|candidate| candidate.text == text);
    if let Some(candidate) = items.iter().find(|candidate| candidate.text == text) {
        engine.commit(candidate);
    }
    engine.clear();
    engine.break_chain();
    hit
}
```

Run: `cd /Users/liyuqing/sproot/qingjian-mainline/cloud && cargo clippy -p qingjian-cloud-bridge --all-targets -- -D warnings 2>&1 | tail -3`
Expected: 末行 `Finished`，没有告警。

- [ ] **Step 2: 跑回放**

产品数据在 `/Users/liyuqing/sproot/qingjian-mainline/data/generated/`（没有就在仓库根跑 `tools/release/data-fetch.sh`）。

Run: `cd /Users/liyuqing/sproot/qingjian-mainline/cloud && cargo run --release -p qingjian-cloud-bridge --example overlay_replay -- /Users/liyuqing/sproot/qingjian-mainline/data/generated 2>/dev/null`
Expected: 四张表（W = 5 / 10 / 20 / 40），每张三行、探针列是 24；数字应与「与大纲的差异」第 3 条（修订后重跑）的表一致（词库或语言模型换过版本时会略有出入）。整段输出存下来，下一步贴进文档、发给审计会话。

- [ ] **Step 3: 权重维持 4**

审计会话已定（决定点 6）：维持 `OVERLAY_WEIGHT = 4`，常数不动。把 Step 2 的输出与「与大纲的差异」第 3 条的表对一下（数字一致即可），进 Step 4。

- [ ] **Step 4: `cloud/docs/design.md` 加「本地记忆」一节**

Modify `cloud/docs/design.md`：在第 262 行 `## 分期` 之前插入下面这段（`<回放表>` 换成 Step 2 打印的四张表，原样贴）：

```markdown
## Task 9：每个场景各一组人，键盘照新稿 01（2026-10-04 定，进行中）

设计稿「01 键盘」改版（etag 1791132191916621）：日常、恋爱、工作各有自己的一组人，牌子拆成场景 / 人两半。审计会话定的规则：

- **对象按场景分组**：`Contact.scene` 本来就有，旧数据都是 dating，不用迁移。每个场景各自最多 8 个，互不相通（`check_limit` 按 scene 分组；`contact_limit` 文案带场景）。对象建好后不能换场景，要换就忘掉再建；对象设置页显示所在场景、不可改，下面一行小字「换场景需要忘掉后重新加」。
- **各场景记住上次选的人**：`ScopeState.last: BTreeMap<Scene, String>`（serde default）；切场景不传人时取 `last[scene]`；旧 state.json 没有 `last` 时，当前有效的 dating 对象回填进去；忘掉的人从 `last` 清掉；对象的 scene 与当前场景不符时 sanitize 成不指定。
- **分区学习**：恋爱照旧（场景层 + 对象层，只写叠加层、不写全局 n-gram）；日常、工作选了人时只开对象层、不开场景层，读时叠加，写时全局与对象层都写。
- **提示**：恋爱、日常出提示行、对象卡与日子提醒；工作三样都不出（工作的人只做分区学习，卡片只在 App 里看）。记一笔三个场景都能用。
- **配色**：恋爱、日常牌子的人那半边用 accent（代表一个人）；工作两半都用中性色。
- **键盘界面**：牌子左半 `.cs` 场景名（ink-2，右侧竖分隔线，点开 1d），右半 `.cp` 圆点加人名（key 色底，点了在工具栏横列本场景其他人的胶囊，点一个切过去并收起）；工具栏右侧依次「记一笔」「改写」（只在配了素笺云时）和收起键盘的向下箭头（`dismissKeyboard`）；提示行「展开」换成 28pt 圆形箭头 `.arw`（ink-2，展开后变向下），日子提醒仍是「知道了」`.more`（ink-3、11.5pt），文字 accent-ink、底 accent-soft、下有 1px 分隔线；对象卡顶部只放牌子与向下箭头；1d 面板场景分段下列该场景的人、「不指定」「新对象 n/8」，右上「完成」。
- **App**：首页按场景分组，顺序恋爱、日常、工作，各自 n/8，工作组中性色；加一个人从哪组点进去就默认哪个场景。
- **顺序**：桥 → 共享层 → 键盘 → App → 截图（对照 01 的 1a–1e、1h），每步带测试。
- **对 2B / 2C 的影响**：上传按用户开了记录的场景走（设计稿 05 的 2f：恋爱缺省开，日常、工作要用户自己开），素材带 scene，整理出的卡挂在对应场景的人下面；工作场景记待办与约好的时间，卡不出提示、只在 App 里看。

## 6. 本地记忆（素笺 2A）

免费、不登录、不联网，先做 iOS。spec 在 `synon-ime` 仓库 `docs/superpowers/specs/2026-10-04-memory-design.md` 的「2A」；代码在 `qingjian-cloud-bridge` 的 `scope/`、`memory/`、`session/memory/`。

- **数据**：学习数据目录下的 `memory/`（iOS 开了完全访问时就是 App Group 的 `Qingjian/memory/`）：`state.json`（当前场景与对象，键盘写）、`contacts.json`（含每个人的两个提示开关）、
  `dismissed.json`（「知道了」，键盘写）、`<对象 id>/cards.json`（`{"rev","cards"}`），以及与全局层同构的分区学习 `<对象 id>/learning/user*.tsv`、`scene-<场景>/learning/user*.tsv`。
  对象 id 是 16 字节随机数的十六进制，名字只在 `contacts.json`。
- **两个进程的读写**：App 与键盘的每个操作（读也算）都在 `memory/.lock` 的 flock 里做（`File::try_lock` 加重试；App 等 2 秒、键盘只等 200 毫秒，键盘拿不到锁就把这次写入放进内存待办、下次 refresh 重试，见 Task 4）；写走同目录临时文件加改名，不建父目录（对象目录只在建对象时创建，忘掉的人不会被写卡片重新建出来）。
  `cards.json` 每写一次修订号加一；App 整份写回时带着读时的修订号，磁盘上更新（键盘这期间记过一笔）就整份不写、返回 `conflict`，App 重读、以 id 为键三方合并后再写；只重写有变化的对象。
  解析不了的文件改名 `.broken-<秒>` 后按空处理；读不了的（锁屏时数据保护）不改名，读-改-写直接报错，键盘内存里的名单与卡片保留原来的。iOS 上 `memory/` 第一次递归设数据保护 `completeUntilFirstUserAuthentication`（不用 `complete`：锁屏通知里回复时键盘要读卡片）。
  键盘切场景、「记一笔」都在锁里按磁盘上的 `state` 与名单读-改-写；App 改了按修改时间重载，App 回到前台时也重读。
- **分区学习（`ScopedLearner`）**：恋爱场景读「全局 + k×场景 + k×对象」、写只进场景与对象层；日常与工作只用全局。用户词、个人 n-gram、英文词表返回引用没法叠加，一律读全局，
  恋爱场景里新造的词、个人英文词会进全局（排序仍由叠加的计数管住）；**恋爱场景不记词序列转移**（个人 n-gram），暧昧的话不会在工作场景的整句里冒出来，代价是恋爱场景的句子不帮整句学习。
  删词连当前打开的场景层与对象层一起删；「忘掉这个人」删整个对象目录（含它的分区学习）。叠加层在 `Arc<Mutex<_>>` 里由会话的 `ScopeHandle` 换：
  `Engine::learner_mut()` 只给 `&mut dyn Learner`，不加上游补丁就只能这样；换完调 `learner_mut()` 作废格子缓存。学习数据同步只认学习数据目录顶层的六个文件，分区层不上云。
- **k = 4**（审计会话 2026-10-04 定）：`examples/overlay_replay.rs` 用合成的选词序列回放（不读真实日志）。排序基本就是计数比大小：在一个对象下选 n 次的词，超过全局里选过 W 次的词的条件约是 `k·n > W`
  （恋爱场景不记转移，日常里记下的个人 n-gram 照样向通用词倾斜，所以不是 `2k·n`），场景层外溢到别的对象约是 `k·（场景里选的次数）> W`；
  权重没有「对」的值，取决于想让「对象下选几次」压过「全局里选过多少次」，维持 spec 的 4（在一个对象下选 1 次约抵全局 4 次）：

  <回放表>

- **提示**：每次 refresh 后拿最近上屏的 24 字加当前首选，去碰当前对象卡片的匹配词（关键词加语言模型切出的两字以上的词，去掉 100 个停用词）；命中词多、新改过的优先，
  一直命中时接着显示，消失后 10 分钟内同一张不再出，「知道了」当天不出（记进 `dismissed.json`，键盘重启也记得；重建索引时带上节流与「知道了」）。
  切到对象时，日子（按年重复，2 月 29 日平年按 28 日）与约定（只一次）在今天到 3 天后的给一条提醒（北京时间），优先于匹配提示；文案日子「明天是她的生日」、约定「明天：看电影」。
  两类提示各按这个人的开关（`hint_on` / `remind_on`）。键盘收起与换输入框时清掉最近上屏的字；私密输入时上屏的字不进缓冲、不出提示；没有 `lm.qj` 时只靠关键词。
  键盘上，恋爱场景选了对象时提示行一直占一行（没提示时空着），高度只在进出这个状态时变。
- **C 接口**：带会话的 `qj_scope_set/get`、`qj_reset_context`、`qj_memory_hint/dismiss/cards/note`，App 用的 `qj_memory_read/write`；JSON 与失败码（含 `conflict`）见 `qingjian_bridge.h`。
- **已知限制**：没开完全访问时键盘读不到 App Group，对象与卡片用不了，键盘里也不让切场景；恋爱场景的句子不帮整句学习；日期按北京时间（UTC+8）写死，海外时区的「今天」会差几个小时；
  App 与键盘同时改同一张卡时以 `touchedAt` 新者为准，另一边的改动丢掉。

```

- [ ] **Step 5: `cloud/README.md`、`cloud/ios/README.md`、`fork-patch.md`**

Modify `cloud/README.md` 第 3 行，在句末「支持 macOS 与 iOS。」之后接一句：

```markdown
iOS 上另有不登录、不联网也能用的本地记忆（素笺 2A）：按日常 / 恋爱 / 工作分开学习，恋爱场景再按对象分开，打字时按你写下的记忆卡提示，数据只在手机上。
```

Modify `cloud/ios/README.md`：在 `## 已知问题` 之前插入：

```markdown
## 本地记忆

主 App 首页是「键盘记住的事」：恋爱场景最多 8 个人，每个人一组记忆卡（日子 / 约定 / 喜好 / 近况 / 其他），「本周」列出 7 天内的日子与约定，「我」里是原来的键盘设置与账号。
数据在 App Group 的 `Qingjian/memory/`，经桥的 `qj_memory_read/write` 整份读写，目录设数据保护 `completeUntilFirstUserAuthentication`；不登录、不联网。

键盘：候选栏左侧的牌子是当前场景（恋爱时是对象名），点开在键区换成场景 / 对象选择——iOS 拿不到宿主应用，**对象只能自己切**。
恋爱场景选了对象后，候选栏上方一直有一行提示行（键盘因此高一行，提示出现消失时高度不变）：打字碰上卡片里的词、日子或约定快到时显示在这里，「展开」看对象卡，「知道了」当天不再提醒；
键盘收起或换了输入框，之前打的字不再算。开了完全访问且剪贴板有字时有「记一笔」，把剪贴板的文字记到当前对象。没开完全访问时键盘读不到 App Group，记忆用不了，牌子也不让切场景。
App 与键盘同时改记忆不会互相覆盖（文件锁加修订号，App 写回时发现键盘记过一笔会重读合并）。
设计见 `cloud/docs/design.md` 的「本地记忆」一节。

```

`cloud/ios/README.md` 的「启用」一节里写的键盘名「青简」改成「素笺」（键盘与容器 App 都改名了）：

Run: `cd /Users/liyuqing/sproot/qingjian-mainline/cloud/ios && sed -i '' 's/添加新键盘 → 青简/添加新键盘 → 素笺/; s/再点进「青简」/再点进「素笺」/' README.md && grep -n "素笺" README.md | head`
Expected: 「启用」一节那一行变成「添加新键盘 → 素笺；要震动就再点进「素笺」…」。

Modify `cloud/docs/fork-patch.md`：第 4 行（「原则：新代码放新文件……」）之后加一段：

```markdown

素笺 2A 本地记忆（场景 / 对象分区学习、打字提示、记忆卡）全部在 `cloud/` 下，**没有新增上游补丁**：分区学习是桥里包着 `FrequencyLearner` 的 `ScopedLearner`，
换层靠桥自己持有的 `ScopeHandle`（不改 `Learner` trait），提示挂在桥的上屏路径上；用到的 `qingjian_core::sentence::{Context, UserNgram, segment_text}` 与 `Engine::learner_mut()` 都是上游已公开的接口。
```

- [ ] **Step 6: 全量检查**

Run: `cd /Users/liyuqing/sproot/qingjian-mainline/cloud && cargo fmt --all -- --check && cargo test -p qingjian-cloud-bridge 2>&1 | grep "test result" && cargo clippy --all-targets -- -D warnings 2>&1 | tail -2`
Expected: fmt 没有输出；全部 `ok`；clippy 末行 `Finished`。

- [ ] **Step 7: 提交**

```bash
cd /Users/liyuqing/sproot/qingjian-mainline
git add cloud/crates/qingjian-cloud-bridge/examples/overlay_replay.rs cloud/docs/design.md cloud/README.md \
  cloud/ios/README.md cloud/docs/fork-patch.md
git commit -m "docs(cloud): 本地记忆的设计与叠加权重回放

apps/cli 的回放装不进 ScopedLearner（它在 cloud 的独立 workspace，CLI 依赖它就是上游补丁），在桥里写 overlay_replay example，用合成的选词序列比较 k=2/4/8。
排序基本就是计数比大小（恋爱场景不记转移后，对象下选 n 次压过全局 W 次约需 k·n > W），权重维持 4，写进 design.md；README 与 iOS README 补本地记忆，fork-patch 写明没有新增上游补丁。

```

## 提交与审计

- 每个任务一个或几个提交，提交信息按仓库约定（`feat(cloud): …`）。
- 每完成一个任务，把提交哈希、测试结果发给审计会话「素笺输入法」；Task 5、6 附模拟器截图路径。
- 发现做不了或与 spec 冲突，先告诉审计会话，不自行改 spec。

## 审计会话已决定的点（2026-10-04，审阅 b2aa168 后）

1. **删词：** `forget` / `forget_english` 先删全局，再删当前打开的场景层与对象层；「忘掉这个人」时整个对象目录（`memory/<id>/` 含 `learning/`）一起删。→ Task 1 Step 5（`ScopedLearner::forget*`）、测试 `forget_clears_the_open_overlay_layers`；Task 2 `forget_contact` / `write_snapshot`。
2. **日子按年重复：** `date` 类按月日匹配今年（过了看明年），2 月 29 日平年按 2 月 28 日；`promise` 只提醒一次。界面文案用「生日」。→ Task 2 `LocalDate::next_anniversary` 与测试；Task 3 `days_away`、`today`、`panel_cards` 与测试（闰年、12 月 30 日看 1 月 2 日）；Task 5 `MemoryCard.daysAway`、`MemoryDate.nextAnniversary`；Task 6 `ContactEditor` 与测试。
3. **提示开关按人：** `Contact.hint_on` / `remind_on`（缺省开、旧文件兼容）；`state` 里不再有开关。→ Task 1 `ScopeState`；Task 2 `Contact`；Task 4 `memory_hint` / `update_hint`；Task 5 `MemoryContact`；Task 6 `ContactSettingsView`。
4. **「知道了」持久化：** 键盘写 `memory/dismissed.json`（`{"cards":{"<card_id>":"<YYYY-MM-DD>"}}`），加载时清 30 天前的与已不存在的卡。→ Task 2 `DismissedFile`、`MemoryStore::dismissed` / `put_dismissed` 与测试；Task 4 `dismiss_hint` 落盘、`LiveMemory::open` 读回，FFI 测试「重开键盘仍记得」。
5. **恋爱场景不写全局 n-gram：** `record_transition` / `unrecord_transition` 在恋爱场景不记，读照常。→ Task 1 Step 5 与测试 `dating_does_not_write_transitions`；已知限制写进 Task 7 的 design.md。
6. **叠加权重维持 4。** → Task 7 Step 3（决定点 5 生效后重跑，结果变慢一倍，见「新增的需要审计会话再决定的点」第 5 条）。
7. **键盘与容器 App 都叫「素笺」**；图标从主检出 `/Users/liyuqing/sproot/qingjian/brand-sujian/icon/` 拷（那个目录没进 git，只在主检出里），拷进 `cloud/ios/App/Assets.xcassets/AppIcon.appiconset/` 并提交。→ Task 6 Step 8（`SetupView` 文案）、Step 9（拷贝命令、`project.yml` 两处、两份 `Info.plist`）；Task 7 Step 5（iOS README）。
8. **节流语义认可；`more` = 除了正在显示的这张还有别的卡**（只算确认过的卡）。→ Task 3 `HintIndex::has_other` 与测试 `more_means_another_card_besides_this_one`。

## 需要同步进 spec 的内容（建议，由审计会话改 spec）

- 「2A 本地记忆 · 分区学习」的表：恋爱场景**不写**个人 n-gram（`record_transition`），写只进场景层与对象层的计数；删词连当前打开的叠加层一起删。已知限制改为「恋爱场景里新造的词、个人英文词会进全局；恋爱场景的句子不帮整句学习」。
- 「数据」一节：`cards.json` 是 `{"rev","cards"}`（旧的数组按 rev 0 读）；新增 `memory/dismissed.json`（只键盘写）与 `memory/.lock`（文件锁）；`state.json` 只有当前场景与对象；提示开关在 `contacts.json` 的每个人上（`hint_on` / `remind_on`）。
- 「数据」一节的写入规则：两个进程的读-改-写都在文件锁里；App 整份写回带修订号，冲突时重读合并；写卡片不建对象目录。
- 「提示」一节：日子按年重复、约定一次；约定的文案是「明天：{text}」；`more` 的定义；键盘收起与换输入框时清最近 24 字。
- 「iOS 界面 · 键盘」：恋爱场景选了对象时提示行常驻（没提示时空行），高度只在进出这个状态时变；没开完全访问时牌子不让切场景。
- 「2A 的错误与边界」：锁屏读不了时键盘保留内存里的名单与卡片、读-改-写报 `io` 不写；日期按北京时间写死。

## 新增的需要审计会话再决定的点

1. **（已解决）`tests/session.rs::logs_only_when_connected`**：它靠 `QINGJIAN_DATA` 才会真正运行（没设就静默跳过并显示 ok），设上数据后确实失败，原因是账号改造把桥的开关缺省关、测试的 `cloud.toml` 没写 `logs = true`；已由单独提交修好（测试配置补上 `logs = true`），Task 4 Step 9 的预期是 4 个全过。**教训：凡是跑桥的会话测试都要带 `QINGJIAN_DATA`，否则「全过」可能是被跳过。**
2. **合并时「App 删卡、键盘同时改了这张卡」**：现在是 App 的删除赢（`MemoryMerge` 里 local 删掉的一律删）。键盘不改已有的卡（「记一笔」只加新卡），2A 里碰不到；2C 云端改卡后可能要重新定。
3. **`dismissed.json` 里别的对象的卡**：重建索引时「知道了」整份带过去，不按当前对象的卡丢（否则切对象再切回来，记录就没了）；过期与已删的卡在加载时按 30 天与「是否还存在」清。审计原话是「卡已不存在的丢弃」，这里把「不存在」理解为「哪个对象下都没有」，请确认。
4. **（已决定）锁的超时**：App 等 `memory/.lock` 最多 2 秒；键盘只等 200 毫秒，超时返回独立错误 `MemoryError::LockTimeout`（不再套在 `io` 里），键盘把这次写入（记一笔、切场景）放进**内存待办、下次 `refresh` 重试**，主线程不会卡 2 秒（待办与重试在 Task 4；Task 2 已提供可配置超时与 `LockTimeout`）。
5. **叠加权重要不要因决定点 5 改成 8。** 恋爱场景不记转移后重跑回放（Task 7「与大纲的差异」第 3 条）：对象词成为首选的条件从 `2k·n > W` 变成约 `k·n > W`，同样 k=4 要多选一倍次数（全局里选过 20 次的词，6 轮内都换不过来）。
   选项：A 维持 4（慢一些，对象之间外溢也少）；B 改成 8（恢复第一次展开时 k=4 的速度，外溢条件变成 `8·n > W`）。我倾向 A：真实用户的全局计数分布与合成序列不同，先上线看真机日志再调；若审计更看重「一个对象下很快学会」就选 B，改常数一处、测试不用改。

## 不确定的地方（执行时留意）

- **Swift 部分（Task 5、6）仍没有编译验证**，以下几处最可能出问题：`UIView.animate` 的闭包在 Swift 6 严格并发下能否直接捕获 `[weak self]`（Task 5 Step 8 给了退路写法）；`MemoryCard.reminderText` 里的 `switch` 表达式赋值（Swift 5.9 起支持）；`MemoryMerge.mergeList` 的 `where T.ID == String` 约束与 `Dictionary(uniquingKeysWith:)`；`textDocumentProxy.documentIdentifier` 在个别宿主里是否每个输入框都不同（只能真机看）。
- 键盘扩展可见时改高度约束，系统多数情况下会跟着动画，个别宿主应用（全屏视频、游戏）可能不响应；只能真机看。
- **Rust 部分已验证**：本次修订后在临时副本（同步到 `sujian` 最新 22086d7 加本计划的全部改动）里按计划原文重新落了 Task 1–4 与 Task 7 的代码，`cargo build`、`cargo test -p qingjian-cloud-bridge`（桥的单元测试 66 个，其中 scope 11、memory 35；`memory_ffi` 9 个；其余原有测试照过）、`cargo clippy -p qingjian-cloud-bridge --all-targets -- -D warnings`、`rustfmt --check` 全过，头文件与导出符号逐个一致；另外按 Task 1、Task 1+2、Task 1+2+3 三个中间状态各编过一次（clippy 无告警、测试全过），保证每个提交单独可编。双进程测试 `two_processes_lose_no_notes` 在本机实测有写回冲突发生并都合并成功。
- 语言模型切卡片文字的效果（`segment_text` 对短句的切分）没量过；切得差时匹配主要靠关键词，App 的卡片编辑页已经提示「不写关键词就按内容自动找」。
- Asset Catalog 里着色图标的 `appearances` 写法依赖 Xcode 16 以上；低版本 Xcode 会忽略暗色 / 着色两张。
- `xcodebuild -scheme QingjianCloud … test` 依赖 Xcode 自动生成的 scheme 带上测试 target（账号计划里只用过 `build`）；若报「scheme 没有测试动作」，在 `project.yml` 顶层加 `schemes: QingjianCloud: { build: { targets: { QingjianCloud: all } }, test: { targets: [QingjianCloudTests] } }` 再 `xcodegen generate`。
- flock 在 iOS App Group 容器上的行为：同一台设备上两个进程对同一个文件 `flock` 是标准 BSD 语义，按理没问题；锁文件 `.lock` 也受数据保护，锁屏时打不开就按 `io` 报错（与「锁屏读不了」一致）。

## 自查

- **大纲每条都有步骤：**
  - Task 1：`Scene` / `ScopedLearner` / `OVERLAY_WEIGHT` / `open` / 全部 22 个方法（Step 5，与「展开前核实」第 2 条的表一一对应，`forget*` 与转移按决定点 1、5 改）/ 五个指定测试（Step 1，外加计数类、恋爱不记转移、删词连叠加层、对象目录不重建、坏 id、`ScopeState` 六个）/ `Context`、`UserNgram` 的核实（展开前核实第 1 条）/ clippy（Step 8）。
  - Task 2：手写卡上限（200 字、8 个关键词、每个 2–8 字）与 `Card` 的 `faded` / `seq` / `updated_at`（Step 5、6，测试 `card_limits_count_characters`）/ `MemoryStore` 大纲的方法（`put_state` 换成 `update_scope`，Step 6）/ serde 字段名与 spec 一致（Step 5）/ `getrandom` 16 字节 id（Step 1、5）/ 复用 `cloud_config` 的原子写（Step 2）/ 坏文件改名 `.broken-<秒>` 加 `tracing::warn!`（Step 6 `read_with`）/ 数据保护在 Swift（Task 6 Step 3）/ 五类测试（Step 3：往返、8 个上限且日常不计、forget 删目录、坏 JSON、两个线程各 50 次），外加审计要的文件锁、修订号与冲突、双进程、读不了不覆盖、忘掉的人不复活、「知道了」记录。
  - Task 3：`Hint` / `HintReason` / `HintIndex::build` / `match_text` / `today` / `dismiss`（Step 5）/ 停用词表 100 个随代码提交（Step 3）/ 两字以上、排序、10 分钟、当天（Step 5）/ 0–3 天与四种称呼（Step 5）/ 北京时间（`LocalDate`，Task 2）/ 六类测试（Step 1），外加按年重复、约定模板、`more`、重建带状态。
  - Task 4：`Session::open` 用 `ScopedLearner`（Step 5）/ 四个上屏路径喂 24 字缓冲（Step 5）/ 9 个 C 函数（大纲 8 个加 `qj_reset_context`，Step 6）与头文件（Step 7）/ `catch_unwind` 与空指针、非法 UTF-8（`with` 与 `path_arg`）/ `learner_mut()` 作废缓存、重建索引、写 `state.json`（Step 4 `set_scope` / `switch_layers`）/ 私密恒 NULL（`memory_hint`）/ 按修改时间重载（`poll_memory`）/ 五类测试加头文件核对（Step 1），外加清最近的字、按人开关、忘掉的人不复活、「知道了」重开仍在。
  - Task 5：牌子 / 选择面板 / 提示行（常驻一行，高度只在进出「恋爱 · 某人」时变）/ `ContactCardPanel` / 「记一笔」/ 没开完全访问的提示与不让切场景（Step 1–7）/ 换输入框清最近的字（Step 7 (h)）/ 构建与截图对照（Step 8–10，含 01 的 1f）。
  - Task 6：首页、详情、`CardEditor`、`ContactEditor`、`ContactSettingsView`、`WeekView`、`MemoryStore`、`MemoryMerge`（Step 3–7）/ Tab 化与回前台重读（Step 8）/ 品牌名与三套图标（Step 9）/ 三类测试（Step 1，外加缺省字段、冲突码、NULL 即成功、按年重复、约定模板、合并两例、本周挑卡、id 格式）/ 截图对照（Step 11，含 02 的 1a、写回冲突）。
  - Task 7：回放比较 2 / 4 / 8（Step 1–3，展开时已实测一遍，权重维持 4）/ design.md（Step 4）/ README、iOS README（含改名）与 fork-patch（Step 5）。
- **名字前后一致：** `ScopedLearner::{open, open_with_weight, handle, OVERLAY_WEIGHT}`、`ScopeHandle::switch`、`ScopeState { scene, contact_id }`、`scene_name` / `parse_scene` / `is_contact_id` / `scene_learning_dir` / `contact_learning_dir` / `load_layer`；
  `MemoryStore::{open, root, contacts, try_contacts, cards, try_cards, state, try_state, put_contact, forget_contact, put_cards, add_note, update_scope, snapshot, write_snapshot, dismissed, put_dismissed, stamp}`、`CardsFile { rev, cards }`、`DismissedFile { cards }`、
  `MemorySnapshot { contacts, cards, revs, state, broken }`、`Card { …, faded, seq, updated_at }`、`Contact { …, hint_on, remind_on }`、`MemoryError::{ContactLimit, Invalid, Conflict, Io}` 与 `code / message / to_json`、`sanitized_scope`、`has_date`、`LocalDate::{from_unix, today, from_ymd, parse, days_until, add_days, ymd, next_anniversary}`；
  `HintIndex::{build, rebuild, dismissed, set_dismissed, match_text, today, dismiss}`、`Hint { card_id, text, reason, more }`、`RecentText::{push_str, text, clear}`、`days_away`、`panel_cards`、`reminder_text(kind, days, pronoun, name, text)`；
  `Session::{set_scope, scope, memory_hint, dismiss_hint, memory_cards, memory_note, reset_context}` 与内部的 `note_committed / update_hint / rebuild_hints / poll_memory / switch_layers`，`LiveMemory::{open, has_contact, contact, reload_contacts, reload_cards, forget_context}`；
  Swift 的 `Engine.{scope, setScope, memoryHint, dismissHint, memoryCards, memoryNote, resetContext}`、`KeyboardModel.{hint, scope, contacts, noteDraft, panelCards, clipboardHasText, fullAccess, hasHintRow, hostChanged, currentContact, canNote, openScopePicker, chooseScope, openContactCard, acknowledgeHint, startNote, confirmNote, cancelNote}`、
  `MemoryStore.{snapshot, message, people, contact, cards(of:), reload, replace, update, addContact, saveContact, forget, saveCard, deleteCard, upcoming(within:now:), exportText}`、`MemoryMerge.merge(base:local:remote:)`、`MemoryLimits.{maxTextChars, maxKeywords, count, clampText, counter, canAdd}`、`MemoryCard.{faded, seq, updatedAt}`、`MemoryContact.{hintOn, remindOn}`、`MemoryDate.nextAnniversary(of:from:)`——各任务里的用法都按这里。
- **没有占位：** 全文没有 TBD / TODO / 「类似 Task N」；Task 7 Step 4 的 `<回放表>` 是执行时由 Step 2 的输出原样贴的数据，不是待补的设计。

## 审计会话审阅后的修订（b2aa168 → 本次）

行号指本文件当前版本。各任务的「与大纲的差异」里有对应条目（Task 1 第 7 条、Task 2 第 2–13 条、Task 3 第 6–7 条、Task 4 第 8 条、Task 5 第 8 条、Task 6 第 3、4、6、7 条、Task 7 第 3–4 条）。

| 条目 | 改在哪 | 行号 |
|---|---|---|
| **阻断：两个进程写同一份数据** ①文件锁 | Task 2 差异 3；Step 6 `store.rs` 的 `lock()`（`File::try_lock`，5 毫秒重试；超时是构造参数：App 2 秒、键盘 200 毫秒，超时返回 `LockTimeout`，键盘侧待办在 Task 4），所有读与读-改-写都在锁里 | 939–960、2197–2670 |
| ②修订号与 `conflict` | Task 2 Step 5 `cards_file.rs`、`snapshot.rs`（`revs`）、`error.rs`（`Conflict`）；Step 6 `write_snapshot` 先比修订号、只写有变化的对象；Task 4 头文件（`conflict`）；Task 6 `MemoryStore.update` 重读重试、`MemoryMerge` 三方合并规则 | 1708–2196、2199–2670、4887–4921、6632–6862 |
| ③回前台重读 | Task 6 Step 8 `SetupView` 的 `scenePhase` | 7391–7467 |
| 双进程测试 | Task 2 Step 3 `tests/sync.rs`：`stale_snapshot_conflicts_and_merges`、`two_processes_lose_no_notes`（两个线程各持一个 `MemoryStore`，键盘 40 次 `add_note`、App 40 次整份写回旧快照，冲突重读合并，断言一条不丢） | 1506–1691 |
| **重要 1：读失败当空后写** | Task 2 `try_contacts/try_cards/try_state`，`add_note`/`put_contact`/`put_cards`/`update_scope`/`write_snapshot` 读失败返回 `io` 不写，`snapshot()` 整份报错；Task 4 会话改用存储的读-改-写，`poll_memory`/换层读不了保留原值；测试 `unreadable_files_abort_writes`（chmod 000） | 2199–2670、1506–1691、4080–4440 |
| **重要 2：忘掉的人复活** | `write_atomic` 加 `create_parent`（Task 2 Step 2），记忆文件一律不建父目录、对象目录只在 `put_contact`/`write_snapshot` 建；`add_note`/`put_cards` 在锁里按磁盘名单判断；`Overlay::open` 对象目录不在就不开（Task 1 Step 4）；测试 `forgotten_contact_is_not_recreated`、`missing_contact_dir_is_not_recreated`、FFI `forgotten_contact_stays_forgotten` | 979–1015、2199–2670、565–667、232–466、3646–4074 |
| **重要 3：切场景覆盖 App 改的** | Task 2 `update_scope`（锁里重读 `state.json` 与名单，只改场景与对象）；Task 1 `ScopeState` 只剩 `scene`、`contact_id`；Task 4 `set_scope` | 848–874、2199–2670、4213–4440 |
| **重要 4：最近 24 字只在换对象时清** | Task 4 `Session::flush` 末尾清、新 C 函数 `qj_reset_context`（头文件与导出核对同步）；Task 5 Step 7 (h) `textDidChange` 比较 `documentIdentifier` 调 `hostChanged()`；FFI 测试 `flush_and_new_field_clear_recent_text` | 4213–4440、4441–4699、4700–4921、6307–6330、3646–4074 |
| **重要 5：重建丢节流与「知道了」** | Task 3 `HintIndex::rebuild` / `dismissed` / `set_dismissed`；Task 4 `rebuild_hints` 用它；测试 `rebuild_keeps_throttle_and_dismissals` | 3203–3375、4213–4440、2758–3072 |
| **重要 6：提示行让宿主界面跳**（2026-10-04 真机后作废，改为没提示不占行，见 Task 5 差异） | Task 5 `KeyboardModel.hasHintRow`、`HintRow` 接受空提示、`KeyboardView` 与控制器 `syncHintRow`/`syncTouchView`/`hintInset` 改看 `hasHintRow`；截图验收 1、2、4、8 改写 | 5615–5818、5821–5901、6067–6216、6217–6330、6341–6377 |
| **决定 1：删词连叠加层** | Task 1 Step 5 `forget`/`forget_english`；测试 `forget_clears_the_open_overlay_layers` | 668–874、232–466 |
| **决定 2：日子按年重复** | Task 2 `LocalDate::next_anniversary` 与测试 `anniversaries_repeat_every_year`；Task 3 `days_away`、`today`、`panel_cards` 与测试（闰年、跨年）；Task 5 `MemoryCard.daysAway`、`MemoryDate.nextAnniversary`；Task 6 `ContactEditor`「生日」与测试 | 1990–2161、1083–1164、3376–3475、5093–5190、7208–7293、6435–6624 |
| **决定 3：提示开关按人** | Task 2 `Contact.hint_on/remind_on`；Task 4 `memory_hint`/`update_hint`；Task 5 `MemoryContact`；Task 6 `ContactSettingsView`；FFI 测试 `hint_switch_is_per_contact` | 1824–1862、4213–4440、5033–5092、7294–7390 |
| **决定 4：「知道了」持久化** | Task 2 `dismissed_file.rs`、`MemoryStore::dismissed/put_dismissed` 与测试；Task 4 `dismiss_hint` 落盘、`LiveMemory::open` 读回，FFI 提示测试末尾「重开仍记得」 | 2180–2196、2199–2670、4080–4440 |
| **决定 5：恋爱场景不写 n-gram** | Task 1 Step 5 `record_transition`/`unrecord_transition`；测试 `dating_does_not_write_transitions`；Task 7 design.md 已知限制；文末「需要同步进 spec 的内容」 | 668–874、232–466、7802–7844、7907–7915 |
| **决定 6：权重维持 4** | Task 7 Step 3；修订后重跑回放变慢一倍，列为新增决定点 5 | 7576–7603、7793–7796 |
| **决定 7：都叫素笺、图标** | Task 6 Step 8 文案、Step 9 拷贝命令（源目录只在主检出、没进 git）与 `project.yml` 两处、两份 `Info.plist`；Task 7 Step 5 iOS README | 7391–7525、7834–7870 |
| **决定 8：`more` 定义** | Task 3 `HintIndex::has_other`、只收确认过的卡；测试 `more_means_another_card_besides_this_one` | 3203–3375、2758–3072 |
| 建议：数据保护递归设一遍 | Task 6 `MemoryStore.protect()` | 6632–6862 |
| 建议：flush 写失败记日志 | `FrequencyLearner::flush` 本来就 `warn`；本计划新加的写（`state.json`、`dismissed.json`）失败都 `tracing::warn!`（Task 4 `set_scope`/`dismiss_hint`） | 4213–4440 |
| 建议：锁屏读名单失败保留原名单 | Task 4 `LiveMemory::reload_contacts/reload_cards`、`poll_memory` | 4080–4440 |
| 建议：约定类文案「明天：{text}」 | Task 3 `reminder_text(kind, …)`；Task 5 `MemoryCard.reminderText`；测试 `promises_remind_once_with_their_own_template`、`testReminderTextMatchesBridgeTemplates` | 3376–3475、5093–5190 |
| 建议：没开完全访问不让切场景 | Task 5 `ScopePicker`（Swift 侧门，桥不知道完全访问） | 5902–6066 |
| 建议：截图补 01 的 1f、02 的 1a | Task 5 Step 10 第 4 项；Task 6 Step 11 第 2 项 | 6341–6377、7531–7555 |
| 建议：`qj_memory_note` NULL 即成功 | Task 5 `MemoryBridge` 注释；Task 6 测试 `testNoteNullMeansSuccess`；头文件注释 | 5528–5583、6435–6624、4887–4921 |
| 建议：UTC+8 写死 | Task 7 design.md 已知限制 | 7802–7844 |
| **补充：手写卡上限（卡片契约）** | Task 2 差异 12–13；`store.rs` 的 `validate_cards`（200 字、8 个关键词、每个 2–8 字，用 proto 4b60e13 的常量，开工前确认存在）；`card.rs` 加 `faded`/`seq`/`updated_at` 注释与语义；测试 `card_limits_count_characters`；Task 5 `MemoryLimits.swift`、「记一笔」截到 200 字、`MemoryCard` 带三个云端字段；Task 6 `CardEditor` 计数与关键词添加、单测 `testCardLimitsCountUnicodeScalars`、`testCardKeepsCloudFieldsOnRoundTrip` | 939–973、1863–1989、2199–2670、1165–1505、5093–5190、5495–5527、7090–7205、6435–6624 |

**新增或改动的接口：**
- Rust：`write_atomic(path, bytes, create_parent)`；`MemoryStore::{try_contacts, try_cards, try_state, add_note, update_scope, dismissed, put_dismissed}`，`snapshot()` 改返回 `Result`，`put_state` 去掉；`CardsFile`、`DismissedFile`；`MemorySnapshot.revs`；`MemoryError::Conflict`（`code = "conflict"`）；`Contact.{hint_on, remind_on}`；`Card.{faded, seq, updated_at}`；`ScopeState` 去掉 `hints`、`reminders`；`sanitized_scope`；`LocalDate::next_anniversary`；`HintIndex::{rebuild, dismissed, set_dismissed}`；`days_away`；`reminder_text` 多一个 `kind` 参数；`Session::reset_context`；`LiveMemory::{reload_contacts, forget_context}`。
- C：新增 `void qj_reset_context(QjSession *session)`；`qj_memory_read` 多 `revs`、读不全返回 NULL；`qj_memory_write` 多 `conflict`、不再采纳 `state`。
- Swift：`Engine.resetContext()`；`KeyboardModel.{hasHintRow, hostChanged()}`；`MemoryContact.{hintOn, remindOn}`、`MemoryCard.{faded, seq, updatedAt}`、`MemorySnapshot.revs`、`MemoryFailure.Code.conflict`、`MemoryDate.nextAnniversary(of:from:)`、`MemoryLimits`、`MemoryMerge.merge(base:local:remote:)`；`MemoryStore` 去掉 `setHints/setReminders`。

**新增或改动的文件：**
- 新增：`src/memory/cards_file.rs`、`src/memory/dismissed_file.rs`、`src/memory/tests/sync.rs`、`cloud/ios/Shared/Memory/MemoryLimits.swift`、`cloud/ios/App/Memory/MemoryMerge.swift`。
- 新改到的现有文件：`cloud/ios/Keyboard/Info.plist`、`cloud/ios/App/Info.plist`（显示名，xcodegen 重写）、`cloud/ios/README.md` 的「启用」一节（键盘名）。
- `cloud_config.rs` 的 `lock()` 不再改成 `pub(crate)`（记忆不用它了）。
