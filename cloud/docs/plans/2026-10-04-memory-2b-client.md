# 素笺 2B 素材上传：客户端实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.
> 本文件是审计会话给出的**任务大纲**：接口、文件、测试与验收已定死。执行前由客户端会话用 writing-plans 把每个任务展开成逐步的代码与命令，展开后先发审计会话审一遍再动手。

**Goal:** 在用户开了记录的场景里，把发出的话与「记一笔」规则脱敏后上传；键盘始终看得见「记录中」，可暂停；App 有分场景开关与单独同意页。

**Architecture:** proto 加 2B 类型；新 crate `qingjian-cloud-redact`（规则层脱敏，客户端与服务端共用同一份代码与测试向量）；`qingjian-cloud-client` 加 `MemorySync`（与 `InputLogSync` 同构的离线队列）；桥在上屏路径聚合「一次发送」交给 `MemorySync`；iOS 加标记、暂停、同意页、场景开关。

**Tech Stack:** Rust（proto、client、bridge、新 crate）、Swift / SwiftUI。

**Spec:** `synon-ime` 仓库 `docs/superpowers/specs/2026-10-04-memory-design.md`「2B」（分支 `sujian-memory-docs`）。**前置：** 2A 完成。Task 1、2 先做并推送，服务端 2B 依赖它们编译。


**状态（2026-10-04 收尾）：** Task 1（proto）、Task 2（`qingjian-cloud-redact`，向量 `tests/vectors.jsonl` 与服务端共用）已在 `sujian` 完成；Task 3–7 **未开始**。接着做时从 Task 3 `MemorySync` 起，先展开、发审计会话审；Task 3 之前先补本文件 Task 3 下记的「防复活」前置检查。
---

## Task 1：proto 的 2B 类型

**Files:** Create `cloud/crates/qingjian-cloud-proto/src/{memory_item,memory_push,memory_accepted,contact_registration,processor_info}.rs`；Modify `feature.rs`（加 `Memory` → `"memory"`）、`consents.rs`（加 `memory: bool`，`#[serde(default)]`）、`lib.rs`（常量与 re-export）。

```rust
pub struct MemoryItem { pub client_id: String, pub contact_id: Option<String>, pub scene: Scene, pub kind: MemoryKind, pub text: String, pub at: i64 }
pub enum Scene { Daily, Dating, Work }          // serde: "daily" / "dating" / "work"
pub enum MemoryKind { Sent, Note }              // serde: "sent" / "note"
pub struct MemoryPush { pub items: Vec<MemoryItem> }
pub struct MemoryAccepted { pub accepted: u32 }
pub struct ContactRegistration { pub scene: Scene }
pub struct ProcessorInfo { pub name: String, pub zero_retention: bool }
pub const PATH_MEMORY_MATERIALS: &str = "/v1/memory/materials";
pub const PATH_MEMORY_CONTACTS: &str = "/v1/memory/contacts";   // 后接 /{contact_id}
pub const PATH_MEMORY_PROCESSOR: &str = "/v1/memory/processor";
pub const MAX_MEMORY_ITEMS: usize = 100;
pub const MAX_MEMORY_TEXT_BYTES: usize = 2000;
```

测试（`tests/memory.rs`）：每个类型与 spec 的 JSON 示例往返；`Consents` 缺 `memory` 字段按 false；`Feature::parse("memory")`。推送后通知服务端会话与审计会话。

## Task 2：`qingjian-cloud-redact`（规则层脱敏）

**Files:** Create `cloud/crates/qingjian-cloud-redact/{Cargo.toml,src/lib.rs,src/rules/*.rs,tests/vectors.rs,tests/vectors.jsonl}`；Modify `cloud/Cargo.toml`（members）。

```rust
pub struct RuleCounts { pub phone: u32, pub landline: u32, pub id_card: u32, pub bank_card: u32, pub email: u32, pub url: u32, pub address: u32 }
pub fn redact_rules(text: &str) -> (String, RuleCounts);
```

- 规则：大陆手机号（1[3-9]\d{9}，可带 +86、空格或连字符分组）→〔手机号〕；座机（0\d{2,3}-?\d{7,8}）→〔电话〕；身份证（18 位含校验位、15 位）→〔证件号〕；银行卡（13–19 位且 Luhn 通过）→〔卡号〕；邮箱→〔邮箱〕；URL→〔链接〕；详细地址（省 / 市 / 区 / 县 + 路 / 街 / 道 / 号 / 栋 / 室 组合，至少到「号」）→〔地址〕。
- 只依赖 `regex`，无网络、无状态；`no_std` 不要求。
- 测试向量 `vectors.jsonl`（≥80 条：每类正例、反例与边界，如 11 位非手机号、Luhn 不过的 16 位数字、日期不被当成号码）；服务端用同一份文件跑测试（路径依赖）。

## Task 3：`MemorySync`

**Files:** Create `cloud/crates/qingjian-cloud-client/src/memory_sync/{mod,queue,worker,tests}.rs`；Modify `client/`（加 `push_materials`、`register_contact`、`delete_contact`、`processor` 四个方法，账号类错误映射；409 `contact_limit` → 新变体 `ClientError::ContactLimit`，400 `unknown_contact` → `Rejected`）。

```rust
pub struct MemorySync { /* 后台线程、落盘队列 <state_dir>/memory-outbox.jsonl */ }
impl MemorySync {
    pub fn start(client: Client, state_dir: &Path) -> Self;
    pub fn enqueue(&self, item: MemoryItem);   // 先 redact_rules 再入队；Disabled 期间直接丢弃
    pub fn status(&self) -> MemoryStatus;      // Online / Offline / Disabled / Unauthorized
    pub fn stop(self);                          // join 后台线程
}
```

- 每 5 秒或攒满 100 条推一批；成功按 `client_id` 出队。
- 403 → 清空队列、状态 Disabled、之后 `enqueue` 丢弃（与剪贴板一致），300 秒后轮询一次；401 → 保留队列、Unauthorized；429 → 退避（沿用 `retry_delay`）；网络错误 → 指数退避，最长 5 分钟。
- 测试（假服务）：离线入队、恢复后补传且顺序不变；403 清队列且之后不入队；401 保留；429 退避；重启后从落盘队列续传；入队文字已脱敏（手机号变〔手机号〕）。

- **前置（2A Task 6 审查记下，2026-10-04）：** 桥的 `write_snapshot`（`cloud/crates/qingjian-cloud-bridge/src/memory/store.rs`）只在磁盘 rev 大于快照 rev 时报冲突；
  对象目录被「忘掉」删除后 rev 回到 0、`contacts.json` 本身也没有 rev，拿着旧快照的写入方（云同步回写、以后 iPad 多窗口）会把已忘掉的人连目录建回来。
  接云同步之前先补：rev 倒退也算冲突，或给名单加一个 rev；配回归测试「忘掉后用旧快照写回被拒」。

## Task 4：桥接入「一次发送」与对象登记

**Files:** Create `cloud/crates/qingjian-cloud-bridge/src/memory/sent.rs`；Modify `session/mod.rs`（上屏路径）、`memory/store.rs`（新建 / 删除对象时调 `register_contact` / `delete_contact`，失败排进 `MemorySync` 的待办）、`memory/ffi.rs`、`cloud_config.rs`（`[memory] record_daily / record_dating / record_work`，缺省 false / false / false；`paused_until`）、头文件。

- 「一次发送」：上屏的文字累积到缓冲，遇到回车上屏（`take_raw` 的换行或宿主发送）或 30 秒无新上屏时封口，生成一条 `MemoryItem{kind: Sent}`。
- 上传条件（全部满足）：已登录；`consents.memory`；当前场景的 `record_*` 为真；`paused_until` 已过；不在私密输入；恋爱场景选了对象时带 `contact_id`，日常场景 `contact_id = null`，恋爱「不指定」不上传。
- `qj_memory_note` 在满足条件时额外 `enqueue` 一条 `kind: Note`。
- 新 C 接口：`qj_memory_pause(minutes)`、`qj_memory_recording() -> char*`（`{"recording":bool,"paused_until":…|null,"scene":…}`，键盘画标记用）。
- 2A 的对象新建遇到 `ContactLimit`（云端 30 天内删过的仍占名额）：App 提示「恋爱场景最多 8 个人（最近 30 天删除的也算）」。
- 测试：封口的两种边界；五个条件逐一不满足时不入队；暂停期间不入队；`note` 双写；对象新建离线时登记进待办、恢复后补登记。

## Task 5：iOS 键盘「记录中」

**Files:** Create `cloud/ios/Keyboard/Sources/RecordingBadge.swift`、`PausedBanner.swift`；Modify `ScopeChip` 所在工具栏、`KeyboardModel.swift`（refresh 时取 `qj_memory_recording`）。

- 标记：在对象牌子之后，「● 记录中」，圆点用 `--accent-ink` 等价色；暂停时空心圈「已暂停」。点「记录中」→ `qj_memory_pause(60)`，出提示条「已暂停记录，提示照常。1 小时后恢复」+「一直暂停」（调 `qj_memory_pause` 为极大值并把当前场景的 `record_*` 置 false，经 App 同步）。
- 私密输入、工作场景（未开）、未登录时不显示标记。
- 验收：截图对照 05 的 2a、2b。

## Task 6：iOS App 同意页与场景开关

**Files:** Create `cloud/ios/App/Memory/RecordingConsentView.swift`、`RecordingSettingsSection.swift`；Modify 「我」页（Account 区）。

- 同意页（05 的 2f）：标题「在哪些场景里记？」；日常 / 恋爱 / 工作三个开关（缺省只开恋爱）；「交给谁处理」一节显示 `processor().name`，带单独的勾选框「同意交给 {name} 整理」，说明「先抹去姓名、电话、地址等再发送；对方不保存、不拿来训练。整理完原文即删」；「永远不记」「随时可以停」两段；勾选前「同意并开始记录」不可用。`name` 为空时整页显示「云端记忆还在准备中」，不可开启。
- 同意后：`put_consent(memory, true)` 成功才写本地 `record_*`；失败按错误代号提示。
- 「我」页：「在哪些场景里记」三个开关；「交给谁处理：{name} · 已同意」；「撤回同意并删除云端数据」→ `put_consent(memory, false)` 并清本地队列与 `record_*`。
- 测试（Swift）：同意按钮可用性；`name` 为空的分支；撤回后本地开关全关。验收：截图对照 05 的 2f、2j。

## Task 7：文档

- `cloud/docs/design.md`「本地记忆」后加「素材上传」一节；`cloud/README.md` 功能清单；隐私说明里写明素材存新加坡、7 天删除、规则脱敏在本机先做。

## 提交与审计

- 每个任务完成后把提交哈希、测试结果发给审计会话「素笺输入法」；Task 5、6 附模拟器截图。
- Task 1、2 推送后立即通知服务端会话（它的 2B 依赖这两个 crate）。
