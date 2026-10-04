# 素笺 2A 本地记忆：客户端实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.
> 本文件是审计会话给出的**任务大纲**：接口、文件、测试与验收已定死。执行前由客户端会话用 writing-plans 把每个任务展开成逐步的代码与命令（写在本文件各任务下），展开后先发审计会话审一遍再动手。

**Goal:** 免费版的本地记忆：场景、对象、手动记忆卡、分区学习、键盘提示行与对象卡、App「键盘记住的事」，不联网、不登录。

**Architecture:** 全部在 `cloud/` 下，不加上游补丁。桥里新增 `scope/`（包装学习器 `ScopedLearner`）与 `memory/`（存储、提示匹配、C 接口）；iOS 键盘加牌子、选择面板、提示行、对象卡面板；App 加「键盘记住的事」一组页面。

**Tech Stack:** Rust（qingjian-cloud-bridge、qingjian-core 的 `Learner` trait、qingjian-learning 的 `FrequencyLearner`）、Swift / SwiftUI（cloud/ios）、XcodeGen。

**Spec:** `synon-ime` 仓库 `docs/superpowers/specs/2026-10-04-memory-design.md`「2A 本地记忆」（分支 `sujian-memory-docs`）。UI：Claude Design「关系记忆 · 移动端 UI」01、02、05（2c、2g、2i）。

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
- iOS 数据保护在 Swift 侧给目录设 `.complete`（Task 6），Rust 不管。

测试：往返；8 个上限（恋爱第 9 个报 `ContactLimit`，日常不计）；`forget_contact` 后目录不存在；坏 JSON 被改名且返回空；并发 put（两个线程各写 50 次）后文件可解析。

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

## Task 5：iOS 键盘

**Files:** Create `ScopeChip.swift`、`ScopePicker.swift`、`HintRow.swift`、`ContactCardPanel.swift`、`MemoryBridge.swift`；Modify `KeyboardView.swift`（VStack 加提示行）、`IdleBar`（左侧牌子、「记一笔」）、`KeyboardModel.swift`（refresh 后取 `qj_memory_hint`）、`project.yml`（新文件）。

- 牌子：恋爱场景显示「{对象名} · 恋爱」，强调色；日常 / 工作只显示灰色场景名。点它在键位区换成 `ScopePicker`：分段控件（日常 / 恋爱 / 工作）+ 对象格子（≤8）+「不指定」+「新对象」（提示「在素笺 App 里新建」）；底部一行「对象只能你自己切，键盘不知道你在和谁聊」。
- 提示行：有提示时在候选栏上方出现，键盘高度加一行（0.2 秒动画）；左侧强调色圆点、文字、右侧「展开」；`reason=today` 时右侧是「知道了」（调 `qj_memory_dismiss(today=true)`）。
- `ContactCardPanel`：替换键位区，头像字、名字、认识天数、最多 3 张卡，底部「全部记忆在素笺 App 里」与「收起」。
- 「记一笔」：只在开了完全访问且剪贴板有文字时显示；点后确认条「记到 {对象}」/「忽略」，确认调 `qj_memory_note`。
- 没开完全访问时牌子点开显示「开启完全访问后才能使用记忆」（App Group 读不到）。

验收：模拟器构建通过；按 01 键盘 1a–1e、05 的 2c 截图对照（结构一致即可，视觉打磨留给子项目 3）；键盘高度变化不遮挡宿主输入框。

## Task 6：iOS App「键盘记住的事」

**Files:** Create `cloud/ios/App/Memory/*`（见文件结构）；Modify `SetupView.swift`（入口改为首页 Tab：记住的 / 本周 / 我，「我」里放原有的「键盘设置」「账号」）、`project.yml`；Create `Tests/MemoryStoreTests.swift`。

- 首页（05 的 2i）：标题「键盘记住的事」、副标题「都是你写的 · 只存在这台手机上」、今日提醒卡、「人 · n / 8」列表、「加一个人」、底部灰底小卡「懒得自己写？」→ 静态说明页。
- 对象详情（02 的 1b，去掉「待确认」分组）：按 日子 / 约定 / 喜好 / 近况 / 其他 分组；右上「设置」。
- `CardEditor`（02 的 1c）：文字、是什么（五选一）、到哪天（日子与约定才显示）、关键词（逗号分隔，可空）、删掉这条。
- `ContactEditor`（05 的 2g）：名字或代号、称呼四选一（缺省 TA）、可选的已知的事（生日、喜欢 / 不喜欢、再写一条）。
- `ContactSettingsView`（02 的 1d）：名字、称呼、打字时提示开关、日子提醒开关、导出为文本（分享面板）、忘掉这个人（二次确认，文案「{名字}的所有记忆会从这台手机上删除，无法恢复」）。
- `WeekView`：7 天内的日子与约定，按日期排。
- `MemoryStore.swift` 经 `qj_memory_read` / `qj_memory_write` 读写；写失败按 `code` 提示（`contact_limit`→「恋爱场景最多 8 个人」）。目录设 `FileProtectionType.complete`。
- 品牌：App 显示名「素笺」，图标用 `brand-sujian/icon/ios-1024-*.png`（亮、暗、着色三套进 Asset Catalog）。

测试（`MemoryStoreTests`）：JSON 解码往返；`contact_limit` 映射；称呼文案四种。验收：模拟器截图对照 02 的 1a–1d、05 的 2g、2i。

## Task 7：回放调参与文档

- 用 `apps/cli` 回放一份输入日志，比较 `OVERLAY_WEIGHT` = 2 / 4 / 8 时恋爱场景对象常用词的首选命中，选定后写进常数与 `cloud/docs/design.md` 新增的「本地记忆」一节。
- 更新 `cloud/docs/design.md`、`cloud/README.md`（功能清单）；`fork-patch.md` 写明本功能没有新增上游补丁。

## 提交与审计

- 每个任务一个或几个提交，提交信息按仓库约定（`feat(cloud): …`）。
- 每完成一个任务，把提交哈希、测试结果发给审计会话「素笺输入法」；Task 5、6 附模拟器截图路径。
- 发现做不了或与 spec 冲突，先告诉审计会话，不自行改 spec。
