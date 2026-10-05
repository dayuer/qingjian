# 素笺：「记一笔」原话存成待整理素材，交给素笺云每日整理成卡（客户端计划，待审）

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.
> 2026-10-05 用户定：复制过来的原话不在键盘里删改（名字、时间都留着），存下后交给大模型结构化成真正的记忆卡；大模型在**素笺云服务端**；**每天整理一次**；**没开云服务的人原话整段存下，开通后再整理**。本文件先发审计会话审，审过再动手。

**Goal:** 「记一笔」（剪贴板与手写）不再直接写成 200 字的卡，而是存成一条「待整理」素材（原话原样、最多 2000 字节）；开了素笺云且同意「记忆」的，素材走 2B 上传、2C 每日整理成卡下发；没开的留在本机，App 里看得到全文，开通后补传。

**Architecture:** 素材就是 2B 已定的 `MemoryItem { kind: Note }`（proto 已有 `MemoryKind::Note`、`MAX_MEMORY_TEXT_BYTES = 2000`），不新造类型。新增的是**本机素材库**（按对象存、2B 上传前后都留着直到整理成卡），以及 App 里「待整理」的展示。上传、整理、下发分别是 2B Task 3–4、2C Task 2–5，本计划只把「记一笔」接进去并补本机这一段。

**Tech Stack:** Rust（qingjian-cloud-bridge、qingjian-cloud-proto）、Swift / SwiftUI。

---

## 与现有计划的关系

- **2A Task 10（本地规则抽取、1e-2 草稿卡、1e-3 冲突）**：用户改为大模型整理，本地规则版**暂缓**；1e-2 / 1e-3 的界面改为展示云端整理出的待确认卡（2C Task 4 的「待确认」），不在键盘里即时拆。待审计会话确认后在 Task 10 计划头上标注。
- **2B**：「记一笔」素材本来就在 2B 的范围（`MemoryKind::Note`）；本计划要求 2B Task 3 的上传队列从本机素材库取（而不是只在内存里），保证没开云服务时攒下的素材开通后能补传。
- **2C**：整理出的卡挂在对应对象下，标「待确认 · 昨晚学到的」，出处写「你记的一笔」（不显示原文，原文在本机素材库里能看）。
- **e68d38b / efb3439（过渡）**：现在剪贴板按段切成多张 200 字的 other 卡、名字时间都留着；本计划落地后，「记一笔」改存素材，这段切分删掉。

## 本机素材库

- 文件：`memory/<对象 id>/materials.jsonl`，一行一条 `{client_id, kind: "note", text, at, source: "clipboard" | "typed", uploaded: bool, processed: bool}`；锁与修订号沿用 `MemoryStore`（flock）。
- 单条上限 2000 字节（与 `MAX_MEMORY_TEXT_BYTES` 一致），超过的按段切成几条（不丢字，沿用 `ClipMessages` 的分段）；每个对象最多留 200 条未整理的，超出提示「待整理的太多了，先开通素笺云整理一下」（不静默丢）。
- 「忘掉这个人」一并删掉素材；整理成卡（2C 下发并确认）后素材标 `processed`，保留 30 天后删（便于用户对照）——**原文保存多久**待审计会话定（设计文档原写「原文处理完就丢弃」，指的是云端）。

## 上传前的处理（要审计会话定）

- 2B 的规则层脱敏（手机号、身份证、银行卡、地址、账号……）照旧在上传前做。
- **对象本人的名字**：设计文档承诺「云端只知道一个随机编号」，但微信多选复制的原话里，说话人那一行常常就是对象的真名。建议上传前在本机把对象的名字与代号替换成「〔对象〕」，其余人名（第三方）保留；这样大模型仍能分清谁说的，云端也不知道对象是谁。
- 素材的时间行（「2026年10月05日 09:34」）保留，整理时用来换算「明天」「下周三」。

## 界面

- 键盘：确认条照旧（「刚复制的 · n 条」→「记到 X」），存好后的 toast 改为「记下了，明早整理」（开了云服务）或「记下了，开通素笺云后整理」（没开）。手写同理。
- App 对象详情：卡片分组下面加一节「待整理 · n 条」（ink-3 小字，展开看原文全文，按时间倒序；每条可删）；没开云服务时这一节下方一行「开通素笺云后，每天帮你整理成记忆卡」，点了进 `CloudIntroView`（T9 后换开通流程）。
- 键盘提示行只用已确认的卡，不用素材（素材是原话，不拿来匹配提示）。

## 任务

### Task M1：proto 与桥的素材库
- [ ] 桥 `memory/materials.rs`：`MaterialsFile` 读写、追加、分段、上限、标记 uploaded / processed、随对象一起删；单测（不丢字、2000 字节切分、上限提示、忘掉时删除、旧目录没有文件时为空）。
- [ ] C 接口：`qj_memory_note` 改为写素材（返回值不变：NULL 成功 / `{"code","message"}`）；新增 `qj_memory_materials(user_dir, contact_id)` 给 App 读（JSON 数组）与 `qj_memory_material_delete(user_dir, contact_id, client_id)`；头文件与 FFI 测试同步。
- [ ] `ClipMessages` 改成按 2000 字节分段（不再按 200 字），删掉过渡代码。

### Task M2：iOS 键盘与 App
- [ ] 键盘 toast 文案按是否开了素笺云（`cloud.toml` 有没有令牌，沿用 `IdleBar` 判断「改写」的办法）二选一；单测锁文案。
- [ ] App 对象详情「待整理」一节（`MaterialsSection`），展开看全文、删除；没开云服务时的开通引导一行；截图对照（设计稿没画这一节，作为有意偏离写进 UI 清单约束 7）。

### Task M3：接 2B 上传（在 2B Task 3 里做）
- [ ] `MemorySync` 的待传队列从各对象的 `materials.jsonl` 里取 `uploaded == false` 的，上传前脱敏、替换对象名字，成功后标 `uploaded`；没开云服务或没同意「记忆」时不取。

### Task M4：接 2C 下发（在 2C Task 2–4 里做）
- [ ] 下发的卡带 `material_ids`（服务端在 2C 里加），确认后把对应素材标 `processed`。

## 要审计会话定的

1. 上传前把对象本人的名字替换成「〔对象〕」（建议）——还是原样上传。
2. 本机原文在整理成卡后保留多久（建议 30 天）。
3. 每个对象未整理素材的上限（建议 200 条）与超出时的提示。
4. 2A Task 10 本地规则版是否正式暂缓。
