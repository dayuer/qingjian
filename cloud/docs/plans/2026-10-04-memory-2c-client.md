# 素笺 2C 每日整理记忆：客户端实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.
> 审计会话给出的**任务大纲**。执行前用 writing-plans 展开成逐步代码与命令，先发审计会话「素笺输入法」审过再动手。

**Goal:** 云端整理出的记忆卡下发到 iOS；用户在 App 里确认、修改、删除、补上是谁；每周确认；键盘提示只用已确认的卡。

**Architecture:** proto 加卡片类型；`MemorySync` 加卡片增量同步与本机改动队列；桥把云端卡并入 2A 的本地结构；iOS 加待确认、每周确认、还没归到人。

**Spec:** synon-ime `docs/superpowers/specs/2026-10-04-memory-2c-design.md`。**前置：** 2A、2B 客户端完成。Task 1 先做并推送（服务端 2C 依赖）。

---

## Task 1：proto 卡片类型
`MemoryCard { card_id, contact_id: Option<String>, kind: CardKind, text, keywords: Vec<String>, when: Option<String>, source: CardSource, confirmed, faded, deleted, seq: i64, updated_at: i64 }`、`CardKind`（date/promise/preference/recent/other）、`CardSource`（cloud/manual）、`CardPage { cards, latest }`、`PutCard`（可写字段，全部 Option，`#[serde(default)]`）、路径常量 `PATH_MEMORY_CARDS`。测试：与 spec 的 JSON 往返；未知字段忽略。推送后通知服务端与审计会话。

## Task 2：`MemorySync` 卡片同步
拉：启动、每 6 小时、App 回前台（桥暴露 `qj_memory_sync_now`）时按 `since` 拉，墓碑删除本地；推：本机改动（手动卡新建 / 修改 / 删除、确认、补上是谁）进落盘待办，按序推，`updated_at` 冲突以新者为准；403 清待办、401 保留。测试（假服务）：增量不漏不重、墓碑、离线排队、冲突、403 / 401。

## Task 3：桥合并到本地结构
云端卡写入 2A 的 `cards.json`（`source=cloud`），`contact_id=NULL` 的写 `memory/unassigned.json`；2A 的 `Card` 加 `faded`、`seq`、`updated_at` 字段（旧文件缺省兼容）；`HintIndex::build` 只取 `confirmed && !faded`；`qj_memory_read` 带上 `unassigned`；`qj_memory_write` 的改动同时入 `MemorySync` 待办（开了 memory 时）。测试：未确认卡不出提示；补上是谁后卡移到对象下；旧版 cards.json 兼容读取。

## Task 4：iOS 待确认与还没归到人
对象详情顶部「待确认 · 昨晚学到的」（02 的 1b）：每张卡「不对」（删除）/「记住」（confirmed）；首页（05 的 2h）：「记录中 · 恋爱」「今天 HH:mm 整理过」、「还没归到人的」列表与「补上」（选对象）；「全部」里淡出的卡灰显、可确认。截图对照。

## Task 5：iOS 每周确认
周日 21:30 本地通知（设置里可关、可选「锁屏不显示名字」）；流程（03 的 1a–1d）：一次一个对象、卡片栈、右滑记住左滑不对、「改一下」进编辑、看完小结（这周记住了几件、认识天数、临近的日子）、「看下一位」。状态机单测（全部确认、部分跳过、中途退出再进）。截图对照。

## Task 6：文档
`cloud/docs/design.md` 加「每日整理」一节；隐私说明同步（DeepSeek、存放地、未承诺零留存、7 天删素材、只用确认过的卡做提示）。

## 提交与审计
每个任务完成后发哈希、测试结果、截图给「素笺输入法」。
