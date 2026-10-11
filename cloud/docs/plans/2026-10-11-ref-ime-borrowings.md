# 借鉴参考输入法的五项改动：总览与执行顺序

> **For agentic workers:** 本页只定顺序、依赖与开工前检查；每一项的具体步骤在各自的计划里，用 superpowers:subagent-driven-development 或 superpowers:executing-plans 逐任务执行。

**来源**：[docs/notes/ref-ime-impl/README.md](../../../docs/notes/ref-ime-impl/README.md) 的审计（2026-10-11），对照上游 qingjian-team/qingjian 的 issue / PR 选出「先做」四项与「按分期」的 B4。
五份计划的代码都已在 `origin/sujian`（dd1ce657）的临时工作树里写过、测过；开工时 sujian 若已前进，先按「开工前检查」核对。

## 五份计划

| 顺序 | 计划 | 解决的问题 | 任务数 | 工作树 / 分支 | 主要改动面 |
| --- | --- | --- | --- | --- | --- |
| 1 | [2026-10-11-auto-space.md](2026-10-11-auto-space.md) | #227、#181 中英自动空格（cherry-pick 上游 PR #372 + 退格 / `#` 两条边界 + iOS） | 7 | `../qingjian-auto-space` | core `engine/commit`、`composing.rs`、`engine/mod.rs`、platform config、macOS 设置、iOS 桥与键盘 |
| 2 | [2026-10-11-fuzzy-sources.md](2026-10-11-fuzzy-sources.md) | #319、#306 模糊音对不合法拼写不生效 | 8 | `../qingjian-fuzzy` | core `fuzzy/`、`parser`、`shuangpin`、`engine/{mod,setup,composing,correcting}.rs` |
| 3 | [2026-10-11-english-learning-gate.md](2026-10-11-english-learning-gate.md) | #545 残片学进英文词表、英文词表无衰减 | 9 | `../qingjian-english-gate` | core `engine/learning`、`composing.rs`、`setup.rs`、`prediction/mod.rs`；learning；cloud-client 同步解析 |
| 4 | [2026-10-11-cloud-request-throttle.md](2026-10-11-cloud-request-throttle.md) | #177 云联想请求过多、#341 云整句排首位 | 18（Task 17 Windows 单独 PR） | `../qingjian-cloud-throttle` | predict、core `engine/prediction`、新 `engine/late/`、cli、macOS / iOS / Windows 壳 |
| 任意 | [2026-10-11-context-kv-cache.md](2026-10-11-context-kv-cache.md) | iOS 通变每键整段重算、上屏后首次重排在关键路径 | 13 | `../qingjian-kv-cache` | neural、core `rescoring`、iOS 桥 |

## 为什么是这个顺序

五份计划两两比对 Create / Modify 清单，代码文件的重叠：

| 文件 | 涉及 |
| --- | --- |
| `crates/qingjian-core/src/engine/composing.rs` | 自动空格、模糊音、英文闸门 |
| `crates/qingjian-core/src/engine/mod.rs` | 自动空格、模糊音 |
| `crates/qingjian-core/src/engine/setup.rs` | 模糊音、英文闸门 |
| `crates/qingjian-core/src/engine/prediction/mod.rs` | 英文闸门、云请求（#372 也改它） |
| `cloud/docs/fork-patch.md`、`docs/notes/crate-notes.md` | 几乎全部（只是各加一节，rebase 时按段合并即可） |

- **B4 上文缓存与其他四项没有代码重叠**，可以随时并行做。
- 其余四项都改 Core 的 engine，**同一时间只做一项**。合并后下一项 rebase 到新的 `origin/sujian` 再开工。
- **自动空格第一**：它是上游提交的原样 cherry-pick（`-x`），越早落地，后面三项越是在「含 #372 的代码」上写，将来同步上游时冲突越少。
- **模糊音第二**：改动最小、原型已验证，修的是用户能直接复现的错。
- **英文闸门第三**：涉及云同步文件格式（`user-english.tsv` 加第三列），要和 cloud-client 一起发版，放在纯 Core 的模糊音之后。
- **云请求最后**：最大（18 个任务，跨四个壳），而且 `engine/late/` 是新子系统，前三项合入后再做，rebase 面最小。

## 每项开工前检查

在新工作树里、写第一行代码之前做：

1. `git -C /Users/liyuqing/sproot/qingjian fetch origin`，工作树从最新 `origin/sujian` 开。
2. `git log --oneline dd1ce657..origin/sujian` 看计划写成后 sujian 进了什么。若含本页前序项，打开那份计划的「文件结构」，与本计划的 Modify 清单比对；重叠文件里，计划给出的「改前片段」与当前文件不一致时，以当前文件为准改写那一步的代码，并在 PR 描述里注明。
3. 按各计划 Task 0 建基线（回放、评测、`gate-pass.sh` 的改前输出）。基线必须在新的 `origin/sujian` 上重跑，不能沿用计划里写的数字。
4. 模型相关测试要把 `data/models`、`data/generated` 软链进工作树（计划里写了）。没链上时模型测试会静默跳过，测试全绿也不算数。

## 待用户拍板（计划按默认值写，改一处即可）

| 项 | 默认 | 改在哪 |
| --- | --- | --- |
| 超时晚到的云端词：丢弃（RefIME 缺省）还是从第 4 位插 | 第 4 位插 | 云请求计划 Task 2 `prediction/defaults.rs` 的 `LATE_FROZEN` 等常数 |
| `sentence_first`（云整句排首位，#341） | 关 | 同上 |
| 429 后锁 30 秒、之后抬高防抖（R9 / R10）是否也做开关 | 始终开 | 云请求计划 Task 8 |
| 模糊音全开时回放汇总里 `serv` 那一行（「现在仍纠」2 → 1）允许不同 | 允许，只此一行 | 模糊音计划 Task 5 的判据 |
| iOS 组句外退格报给 Core，附带「删掉刚上屏的词重打会撤回那次学习」 | 接受 | 自动空格计划 Task 4 |
| 升级后一次性清理英文词表：英文模式下有意学的 ≤ 2 字母词，写法与随包表不同的也会被清 | 接受 | 英文闸门计划 Task 2 |
| 自动空格：中英 / 中数是否拆两个开关 | 不拆（与 #372 一致） | 自动空格计划末尾「可选后续」 |

## 不在这五份里的后续

- 整段纠错「换一个字母」不限邻键（模糊音计划发现的第二层根因，影响所有用户），单独立项。
- 用户常用词不被云整句抢首选（RefIME `IsProtectLocalFirst`）、神经重排晚到后的守卫：云请求计划已记入 todo。
- 搁置项（B1 / B2 / B3 / B6 / C1 / C2）与按分期的 A1 / A2 / A3：见审计页。
