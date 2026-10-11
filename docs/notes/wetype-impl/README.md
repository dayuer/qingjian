# 微信输入法功能的复刻规格（索引与审计）

[wetype-feature-prompts.md](../wetype-feature-prompts.md) 的 12 条任务，加上对照上游 issue 后新挖的 3 条，每条一份规格：
「WeType 怎么做的 → 证据表 → 青简落点 → 复刻步骤 → 未知」。接任务的会话先读这一页的审计结论，再读对应规格，按「复刻步骤」做；
落点里的类型签名、文件位置、常数初值都已定，不需要再设计。

取证方法：WeType 2.2.3 macOS arm64 片纯静态反汇编（objdump 全量 + 符号表 + Swift 字段描述符），逐函数读控制流，地址都是 vmaddr。
REA / Ghidra 对 88 MB 主二进制全量分析超时，没有用。没有运行、调试、抓包 WeType，没有导出词条、模型权重或超参值。
置信度：**[确]** 有指令 / 符号 / 字符串直接证据，**[推]** 多证据推出，**[猜]** 只有名字线索。WeType 的常数只是参考，青简要用自己的回放重扫。
规格里提到的 `/tmp/...` 工作文件是临时的；留得住的原型 diff、测试和脚本在 [proto/](proto/)。

## 审计（2026-10-11）

对照对象：上游 qingjian-team/qingjian 的 363 个 issue、168 个 PR，与素笺 `sujian` 尚未合入的上游提交；素笺分叉关了 issue，分叉侧需求按
[relationship-memory.md](../../design/relationship-memory.md) 的分期与 [todo.md](../../plan/todo.md) 判断。判据：有没有真实用户问题或素笺分期要它，
上游是不是已经在做，WeType 的机制是不是对症。

### 先做：有上游 issue、素笺能复现、原型已验证

| 项 | 对应问题 | 规格 | 结论 |
| --- | --- | --- | --- |
| **D1 模糊音按键级展开** | #319（`tinwosuo` 被纠成「请我说」）、#306 / PR #330（双拼 `suang`） | [d1-fuzzy-as-augmenter.md](d1-fuzzy-as-augmenter.md) | 根因：切分只认合法音节，`tin` / `suang` 这类「套一次模糊规则才合法」的拼写（全开 60 个）模糊音根本没参与；整段纠错的换字母又不限邻键，于是 t→q 接管。原型：模糊源拼写当完整音节 + 有模糊读法时整段纠错须高出一次编辑代价才接管；`tinwosuo` → 听我说、`suang` / 自然码 `sd` → 双，#330 的 3 个测试原样通过；三种模糊配置下整句评测与 clean2 回放逐条零差异（评测集是模糊音关着采的，只证明不伤合法输入）。[proto/d1-fuzzy-sources.diff](proto/d1-fuzzy-sources.diff) |
| **D3 学习闸门（英文词）** | #545（残片 `IA` / `Sm` 学进英文词表恶性循环）、todo「个人英文词表没有衰减」 | [d3-learning-gate.md](d3-learning-gate.md) | 素笺复现两条路：分段上屏后剩下的非拼音残段（`wolai` 选「我啦」剩 `i`）被当英文学；短串回车（`sm`）学成英文后压过「什么」。WeType 中文模式下回车原样串与独立英文候选都不学。改法：残段不学；回车串 < 3 字母不学、< 5 字母且不在随包词表不学；退格删净撤销那次学习；中文选得比英文多 3 次以上英文让到第 5 位；旧词表加载时按新规则清一次。回放：词首选 920/1057 不变，英文首选 16/21 → 15/21（少的是回车学进去的 `ac`）。[proto/d3-english-gate.diff](proto/d3-english-gate.diff) |
| **D2 云请求触发** | #177（停顿就发，几百字 100+ 次请求）、#341（云整句排首位）、#441（已有 `[predict] enabled` / `slots = 0`，不用做） | [d2-cloud-request-strategy.md](d2-cloud-request-strategy.md) | WeType 不靠停顿防抖，靠门槛：无前文时 ≥ 4 键、> 6 键必发、4–6 键且整串是一个合法音节不发；同请求不重发；退格不发。青简 11 条规则（R1–R11，各标出处）；近似回放本机日志请求数降 62–71%（501–1150 → 188–336，上限估计，精确数待 CLI `--predict-sim`）。#341 做成缺省关的 `sentence_first`，与 B5 统一。[proto/d2-request-sim.py](proto/d2-request-sim.py) |
| **C3 中英自动空格** | #227、#181 | [c1-c3-cleanup-offline-space.md](c1-c3-cleanup-offline-space.md) C3 节 | **不自己写，合上游 PR #372**（Core + macOS，缺省关，未合）。合入后按 WeType 补两条：上一键是退格不补、光标前文含 `#` 不补；可选：`% ‰ ° ℃` 归数字类、中英 / 中数拆两个开关。再接 iOS bridge。尾部空格（要发方向键）不做。 |

### 素笺自己要，按分期做（上游没有对应 issue）

| 项 | 规格 | 结论 |
| --- | --- | --- |
| B4 上文缓存 | [b4-b6-context-cache-and-tiers.md](b4-b6-context-cache-and-tiers.md) | iOS 唯一模型通变每键整段重算拼音前缀、上屏后第一次重排整段重算在关键路径上，是真延迟；照规格做。验收的「逐位一致」改为两档容差。 |
| A1 崩溃隔离 | [a1-a2-pack-crash-and-patch.md](a1-a2-pack-crash-and-patch.md) | 素笺键盘有过 jetsam 与泄漏（`fork-patch.md`、#20），先用在现有词库 / 模型加载上，人设包与用户包落地后复用。拉黑与回退上一版是青简自加的。 |
| A3 叠加层 | [a3-session-layers.md](a3-session-layers.md) | P0 的 `ScopedLearner` 已在 cloud bridge 实现；剩 `PersonaSpec` / `HintIndex` 时按规格的层序与「单入口旁路私密」组织，WeType 只借组织规矩。 |
| A2 增量同步 | 同 A1 | P2（打包与增量同步）才需要；「对密文打补丁」不成立，改为明文求补丁、补丁加密下发。到 P2 再做。 |

### 搁置

| 项 | 规格 | 理由 |
| --- | --- | --- |
| B1 变换标记重构 | [b1-augmenter-marks.md](b1-augmenter-marks.md) | 用户能感知的部分被 D1 吸收；整体重构行为不变、收益靠以后调权重，敲错代价 2026-09-12 已扫过在峰上（todo）。D1 落地后再评估。 |
| B3 beam 衰减 | [b2-b3-touch-and-beam.md](b2-b3-touch-and-beam.md) | 整句不准的根因是格子准入（每格只留 6 词，素笺已撤回的上游 PR #405 的分析），不是 beam；#436 的延迟在 IMK / Electron 侧，不在引擎。 |
| B2 触点个性化 | 同上 | 没有用户反馈支撑；要在 iOS 键盘按键路径加坐标，先等 iOS 敲错率有数据。 |
| B6 设备分档 | [b4-b6-context-cache-and-tiers.md](b4-b6-context-cache-and-tiers.md) | 等 B4 做完、老机型实测超标再做；Core 只需把 `BEAM_WIDTH` / `RESCORE_PATHS` 改成可设。 |
| B5 晚到插位 | [b5-late-merge.md](b5-late-merge.md) | 规则本身成立，但上游用户要的是相反方向（#341 云整句排首位），与 D2 一起做：`engine/late/` 落地时同时带 `sentence_first`。 |
| C1 用户词清理 | [c1-c3-cleanup-offline-space.md](c1-c3-cleanup-offline-space.md) C1 节 | 英文词部分并入 D3；中文用户词没有容量问题的报告，分区学习表打包时（P2）再用同一套判据。 |
| C2 完全离线 | 同上 | 素笺设计已保证断网可用（`cloud/docs/design.md`「离线照样可用」），总开关只是锦上添花；做时在 `Client::url()` 集中守卫。 |

### 审计中顺带确认的

- #425 / #475（选了还排不上第一）在素笺不复现：选一次就排第一、重启仍在。风险反过来是误选一次就翻掉高频字；照 WeType「单字母首次选择不算」试过，冻结日志单字母选词 24/24 → 23/24，**不改**。
- WeType 的 `UserSingleCnSelectDict`（最近 4 次 ≥ 3 次选同字置顶）只在五笔 / 笔画启用，拼音不用；单字母简拼时 WeType 不查用户词库。
- 整段纠错「换一个字母」不限邻键（D1 发现的第二层根因）会影响所有用户，没在 D1 原型里改，单独立项。

### 要先拍板的

1. B5 / D2：超时到的云结果丢（WeType 缺省）还是从第 4 位插（规格建议）；`sentence_first` 缺省关。
2. A2 补丁库 `qbsdiff` 的许可（须与 GPL-3.0 兼容），到 P2 再核。
3. C3 合 #372 时是否拆成中英 / 中数两个开关（#372 是一个）。
4. D1、D3 都改 Core 上游文件：按 [fork-patch.md](../../../cloud/docs/fork-patch.md) 记「分叉补丁」；上游同步时注意 #330 若先合入会和 D1 冲突。

## 规格一览

| 任务 | 规格 | 一句话 |
| --- | --- | --- |
| A1 / A2 | [a1-a2-pack-crash-and-patch.md](a1-a2-pack-crash-and-patch.md) | 加载前写整批 md5 合成哈希、5 秒后清，残留则整批回退随包版；补丁是 BSDIFF40 + bzip2，任何一本失败整批作废 |
| A3 | [a3-session-layers.md](a3-session-layers.md) | 105 层由里到外（拼音核心在最里）；命令向内透传、结果走信号外推；重置逐层下传；无运行时私密总闸 |
| B1 | [b1-augmenter-marks.md](b1-augmenter-marks.md) | 每音节纠错预算 61：邻键 28、换位 31、多敲 31、漏敲 / 查表 / 重复 61，模糊音不扣 |
| B2 / B3 | [b2-b3-touch-and-beam.md](b2-b3-touch-and-beam.md) | 每键偏移 EMA + 高斯打分只作用于邻键；六种 beam 调度公式，简拼 / 用户词 / 英文是保留名额 |
| B4 / B6 | [b4-b6-context-cache-and-tiers.md](b4-b6-context-cache-and-tiers.md) | 上文变化后台预填、按公共前缀截断复用 KV；静态档 + 本会话耗时高水位降级 |
| B5 | [b5-late-merge.md](b5-late-merge.md) | 200 ms 内最多插 1 条；超时缺省丢弃；常用且最近用过的本地首选受保护 |
| C1–C3 | [c1-c3-cleanup-offline-space.md](c1-c3-cleanup-offline-space.md) | 删词：频次 < 5+级 且闲置 > 30−4×级 天；离线开关散落约 60 处；空格四类字符对称真值表 |
| D1 | [d1-fuzzy-as-augmenter.md](d1-fuzzy-as-augmenter.md) | 五个模糊增强器都是按键级、不扣预算；另有掩码禁止模糊紧跟敲错 |
| D2 | [d2-cloud-request-strategy.md](d2-cloud-request-strategy.md) | 云请求靠长度与合法音节门槛、同请求去重、服务端预填缓存，不靠防抖 |
| D3 | [d3-learning-gate.md](d3-learning-gate.md) | 中文模式回车串与独立英文候选不学；英文词 < 5 字母且不在词表不展示；上屏后立即删除才撤销学习 |

## 和任务原文 / 旧笔记不一致的地方

改任务时以规格为准，旧笔记 [wetype-reverse.md](../wetype-reverse.md) 已订正。

- A1「拉黑 + 回退上一版 + 上报」、A2「保留上一版、补丁失败退整包」不是 WeType 的做法，是青简自加的。
- A2「补丁对密文做」不成立（随机 nonce 的 AES-GCM），改为明文求补丁、补丁加密下发。
- A3：WeType 的层是代码里先写的在外层，拼音核心在最里。
- B1：`InsertAtMiddle` 是漏敲、`Skip` 是多敲；掩码 0x271ffa08 是 `NextInputMustBeSep`（这几类记录之后音节必须结束），禁止模糊紧跟敲错的是另一组 0x0fdfff0f（D1 订正）。
- B2 / B3：`EmitInputEvent+0x74` 不是 learnable，是内嵌 `KeyBoardLayout` 的 valid 字段（D3 订正）。
- B4「逐位一致」做不到，改两档容差。
- B5：超时后从第 4 位插是缺省关的 AB 分支，缺省丢弃。
- C1：不是「次数 × 衰减」；`…_roll_back` 与 `user_dict_clean_up.bin` 都和清理无关。
