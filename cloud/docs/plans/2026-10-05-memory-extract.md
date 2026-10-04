# 素笺 2A Task 10：记一笔的本地抽取、草稿卡与冲突（客户端实施计划）

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.
> 审计会话「素笺输入法」2026-10-05 定的方案：本地规则、放在桥里（Rust，平台无关，可单测），免费版离线可用，不上传。本文件先发审计会话审，审过再动手；排在 2A Task 9 之后。

**Goal:** 复制对方的话点「记一笔」时，桥按本地规则抽出时间、地点、喜好、计划；一项都抽不出就不出确认条（1e）；抽得出就在键区里摆成可改的草稿卡（1e-2），和这个人已有的卡冲突时并排给用户选（1e-3）。手写记一笔不受「抽不出不出条」限制。

**Architecture:** 桥新增 `extract/` 模块（纯函数：文本 + 今天 + 地名表 → 抽取结果），`memory/` 里加冲突比对与「按草稿存卡」；两个新 C 接口给键盘。键盘新增草稿卡面板与冲突面板，字段编辑复用手写记一笔的 `OutputRouter` 改道（字只进字段、不进宿主）。

**Tech Stack:** Rust（qingjian-cloud-bridge、qingjian-dictionary 读 places.qj）、Swift / SwiftUI（cloud/ios）。

**设计稿:** Claude Design「关系记忆 · 移动端 UI」01 键盘的 1e、1e-2、1e-3（etag 1791133829624639）。

---

## 规则（审计会话定）

| 类 | 触发 | 结果 | 拿不准 |
|---|---|---|---|
| 时间 | 今晚、明天、后天、周X、下周X、X号、X月X日、下个月、月底 | 按今天（北京时间）换算成 `YYYY-MM-DD`；「下个月」「月底」给月末或月份 | 相对说法（下周X、下个月、月底、X号跨月）一律标拿不准，附依据，如「『下个月』按今天（10 月）算的」 |
| 地点 | places.qj（约 4 万条）最长匹配；「去 / 在 / 到 + 名词（2–6 字）」 | 地名字符串 → 卡片 keywords | 只靠「去/在/到」模式、不在地名表里的标拿不准 |
| 喜好 | 喜欢、爱吃、爱喝、不吃、不喝、讨厌、过敏、受不了 + 宾语 | 极性（正 / 负）+ 宾语 | 宾语超过 6 字或含代词（这个、那个）标拿不准 |
| 计划 | 想去、打算、准备、要去、约了 + 宾语 | 宾语（去哪 / 做什么），与同句的时间、地点合并 | — |

- **只抽对方的事**：输入是用户复制的对方原话；不跟用户自己说过的话做前后一致性比对（产品原则）。
- **出条规则（1e）**：一项都抽不出 → 不出确认条，也不提示「未识别」；剪贴板仍记为「处理过」，同一段不再弹。手写记一笔不经抽取，一律能记（存成一张 other 卡，保持现状）。
- **否定与疑问**：「不想去」「要不要去」「你喜欢吗」这类否定计划、疑问句不抽计划 / 喜好（「不吃」「不喝」是喜好的负极性，例外）。
- **宾语截断**：宾语到标点、「了」「吧」「呢」「啊」、连词（和、跟、然后）为止，最长 8 字。

## 草稿卡怎么存（方案）

一句话里的东西按「一件事一张卡」存，不是「一项一张卡」：

- **计划**连同同句里的时间、地点合成一张卡：有时间且是答应 / 约定口吻（约了、答应、说好）→ `promise`，否则 `other`；`text` 是规整后的一句（「去厦门（沙坡尾）」），`when` 是换算出的日期，`keywords` 是地点与计划宾语。理由：提示行靠 keywords 命中、靠 when 出日子提醒，同一件事的时间地点拆开反而两张卡各缺一半。
- **每条喜好**单独一张 `preference` 卡：`text` 是「不吃香菜」「喜欢冰美式」，`keywords` 是宾语。
- **只有时间或地点、没有计划和喜好**（「周六在沙坡尾」）→ 一张 `other` 卡，带 when / keywords。
- 草稿卡上显示的是「项」（计划 / 时间 / 地点 / 喜好），按上面的规则在「记下 n 条」时合成卡；按钮上的 n 是**将要存的卡数**，不是项数。

## 冲突（1e-3）

和这个人已有的卡比：

- 喜好：同一宾语、极性相反（不吃香菜 ↔ 爱吃香菜）。
- 计划：同一计划宾语（或同一地点）、日期不同。
- 「更新为新的」= 改那张旧卡（text、when、keywords、`updated_at`），修订号加一；「两条都留」= 新增一张，两张按时间排；「不记」= 什么都不写。一次只处理第一条冲突，其余草稿项照常存。

## C 接口

```c
// 抽取：返回 {"items":[…],"conflicts":[…]}；一项都没有返回 NULL（键盘据此不出确认条）。
// item: {"id","kind":"plan|time|place|like","value","polarity"?,"date"?,"unsure":bool,"why"?}
// conflict: {"item_id","card_id","old_text","old_date","new_text","new_date"}
char *qj_memory_extract(QjSession *session, const char *contact_id, const char *text);
// 按（用户改过的）草稿存卡：draft 是 extract 的 items（可改 value、删项），resolution 取 "update" / "both" / "skip"（没有冲突时传 NULL）。
// 成功返回 {"saved":n}，失败返回 {"code","message"}；锁拿不到同 qj_memory_note 进待办。
char *qj_memory_save_draft(QjSession *session, const char *contact_id, const char *draft, const char *resolution);
```

## 文件结构

- 桥（`cloud/crates/qingjian-cloud-bridge/src/`）：
  - `extract/mod.rs`（`extract(text, today, places) -> Extraction`）、`extract/time.rs`、`extract/place.rs`（地名表：首次用时从 `dicts/places.qj` 经 `Dictionary::open_qj(..).entries()` 建 `HashSet<String>` 与最长词长，Session 里懒加载缓存，约 2 MB）、`extract/like.rs`、`extract/plan.rs`、`extract/item.rs`（`DraftItem`）、`extract/tests/`。
  - `memory/conflict.rs`（草稿项 vs 已有卡）、`memory/draft.rs`（草稿项 → 卡的合成规则）、`memory/store.rs` 加 `save_draft`（锁内读卡、更新或追加、写回）。
  - `memory/ffi.rs` 两个接口，`include/qingjian_bridge.h` 同步。
- 评测：`cloud/crates/qingjian-cloud-bridge/tests/extract_eval.rs` + `tests/data/extract-eval.tsv`。
- iOS：
  - `Shared/Memory/Extraction.swift`（解码）、`Shared/Memory/DraftEditor.swift`（纯值：删项、改值、要存几张卡的预估、冲突选择）。
  - `Keyboard/Sources/ClipOfferBar`（已有的确认条改 1e 样式）、`DraftPanel.swift`（1e-2：原话一行可展开、字段列表、拿不准的虚线加问号与依据、×删项、「不记」「记下 n 条」）、`ConflictPanel.swift`（1e-3：旧卡划线、新卡 accent 描边、「不记」「两条都留」「更新为新的」）。
  - 字段编辑：点一个字段进入改道（同手写记一笔的 `OutputRouter`，字只进这个字段），完成时写回草稿。
  - `KeyboardModel`：`startNote` 读到剪贴板后先调 `qj_memory_extract`，NULL 就标处理过、不出条；有结果出 1e 条，点「记到」进 1e-2，有冲突时「记下」前进 1e-3。

## 任务

### Task 10.1：评测集与时间抽取
- [ ] 写 `tests/data/extract-eval.tsv`：至少 60 条合成聊天句子，列：句子、今天日期、期望项（kind=value[!]，`!` 表示拿不准）、期望出不出条。组成：时间 15、地点 10、喜好 12、计划 10、混合 8、**中性句（什么都不该抽）不少于 15**；口语、错别字（「后天」写成「后填」这类不强求命中，标期望为空）、反问、否定都要有。句子全是合成的，不用真实聊天。
- [ ] `extract/time.rs` + 单测（换算表：今天 2026-10-05 周一时，明天、后天、周六、下周三、15 号、11 月 2 日、下个月、月底、今晚各给出什么、哪些拿不准、依据文案）。
- [ ] `tests/extract_eval.rs`：读评测集，统计每类的命中率（期望项被抽中的比例）与误报率（抽出但不在期望里的项 / 抽出的项），以及「出条」的误报率（中性句里出了条的比例）；打印成表，断言出条误报率 ≤ 10%。

### Task 10.2：地点、喜好、计划
- [ ] `extract/place.rs`：地名表懒加载；最长匹配；「去/在/到 + 名词」模式；单测（沙坡尾、厦门、「去公司」这种不该当地点的常见词要排除：维护一份小的排除表）。
- [ ] `extract/like.rs`、`extract/plan.rs`：触发词 + 宾语截断 + 否定 / 疑问过滤；单测。
- [ ] `extract/mod.rs` 合并同句的计划、时间、地点；跑评测集，误报率超过 10% 先收紧规则（加排除词、缩短宾语、要求触发词前后有边界），把每轮的数字记进本文件「评测记录」。

### Task 10.3：草稿合成、冲突、存卡
- [ ] `memory/draft.rs`：项 → 卡（上面「草稿卡怎么存」），单测覆盖四种组合。
- [ ] `memory/conflict.rs`：单测（极性相反、计划同宾语不同日期、同宾语同极性不算冲突、自己的话不比）。
- [ ] `MemoryStore::save_draft`：update / both / skip 三种，修订号加一，锁拿不到进待办（复用 `PendingNote` 的队列，加一个变体）。
- [ ] 两个 C 接口 + 头文件 + `tests/memory_ffi.rs`（extract 返回 NULL 的中性句、有冲突时的 JSON、save_draft 三种 resolution 的结果）。

### Task 10.4：键盘 1e / 1e-2 / 1e-3
- [ ] `KeyboardModel.startNote` 接抽取；`ClipOfferBar` 改 1e 样式（「刚复制的」小字 + 原文、「忽略」`.btn.ghost`、「记到 X」`.btn.acc`）。
- [ ] `DraftPanel`、`ConflictPanel` 按设计稿；字段编辑走 `OutputRouter` 改道，测试「编辑字段时宿主一次都没被写」沿用 Task 6 那条守法。
- [ ] 存好后 `.toast`「记下了 n 条」；「不记」后同一段剪贴板不再提示。
- [ ] 截图对照 1e、1e-2（含拿不准的项）、1e-3，交 UI 审计员。

## 评测记录

（执行时填：每轮规则改动后的命中率 / 误报率表。）
