# 从参考输入法借鉴给素笺的能力：功能提示词

来源：[ref-ime-reverse.md](ref-ime-reverse.md)（RefIME 2.2.3 静态逆向）。每条是一份可以直接交给新会话的任务说明，
每条都附了逐函数取证后的复刻规格（索引与分歧汇总在 [ref-ime-impl/README.md](ref-ime-impl/README.md)），接任务先读规格。
只借机制，不借数据：厂商的词库、模型、参数一律不提取、不复制，常数要用我们自己的回放与评测重新扫。

筛选依据是素笺的边界（[relationship-memory.md](../design/relationship-memory.md)）：打字全本地、云端只异步收素材与下发包、
分区之间不共享、不读聊天窗口、不跨用户学习。所以下面三件 RefIME 有、素笺**不做**：

- **读取聊天应用的聊天上下文**：RefIME 的做法是和聊天应用进程合作的私有 IPC——`AppCommunicator.postNotifToChat` / `recieveFromChat`
  经 `NSDistributedNotificationCenter` 收发，先 `SayHello` 握手、`checkChatTokenStatus` 验令牌、载荷用 `chatEncryptKey` 加密，
  请求 `_fetchXChatCursorContext` 拿光标附近的聊天内容（只对 `com.vendor.xinChat` / `xChat` / 工作聊天应用生效，有 `disable_fetch_xchat_context` 开关）。
  这要宿主 App 配合，第三方做不到；即使做得到也违反「不读聊天窗口」。
- **上屏内容上报**（`CloudReportEmitInput`）与**上屏 bigram 统计上报**（`CollectAndReportBiGram`）。
- **用全体用户数据训练的排序模型**（LightGBM LTR 若用聚合日志训练）。下面第 4 条只取「特征化」的思路，权重用公开语料与自己的回放定。

优先级：A 直接服务于素笺已定的 P0 / P1；B 提升打字体验，iOS 上收益明显；C 有空再做。

---

## A1 包加载崩溃隔离（崩溃哈希拉黑）

复刻规格（反汇编取证 + 青简落点）：[ref-ime-impl/a1-a2-pack-crash-and-patch.md](ref-ime-impl/a1-a2-pack-crash-and-patch.md)。规格与下面原文冲突时以规格为准。
**审计（2026-10-11）：按分期做（先用在现有词库 / 模型加载）。** 见 [ref-ime-impl/README.md](ref-ime-impl/README.md#审计2026-10-11)。

```
背景：素笺的人设包 / 用户包由云端打好后同步到本地，iOS 键盘扩展内存紧（jetsam），一个坏包能让键盘每次调出都崩，用户只能卸载。
RefIME 的做法（maybeCrashDictsHashes）：加载某版数据前先记一个「正在加载 <哈希>」标记，加载完成并稳定运行后清掉；
下次启动若发现标记还在，说明上次加载这版时崩了，就把这个哈希列入黑名单，回退到上一版（没有上一版就不加载这个包），并在下次同步时上报版本号。

任务：给素笺的包加载路径加崩溃隔离。
- 范围：iOS 键盘加载人设包 / 用户包的地方（cloud/ios/Keyboard，先 grep 包加载入口），Mac 壳的同等路径；黑名单存在 App Group 共享目录，只存哈希与时间。
- 「稳定」的判定：加载完成后成功处理过 N 次按键或存活 T 秒（N、T 先取 20 次 / 10 秒，写成常数）。
- 黑名单的包不再加载，直到同步下来新版本；壳在状态条用一行字提示「包已回退」，不弹窗。
- 回退时保留本地队列里的素材，不丢。
验收：单测覆盖「标记残留 → 拉黑 → 回退上一版」「新版本到达 → 解除」；真机用一个故意损坏的测试包（自造，不用真实用户数据）验证键盘不再连续崩。
不做：自动删除用户数据；把崩溃堆栈连同包内容一起上传。
```

## A2 包的增量同步（bsdiff + 整包兜底）

复刻规格（反汇编取证 + 青简落点）：[ref-ime-impl/a1-a2-pack-crash-and-patch.md](ref-ime-impl/a1-a2-pack-crash-and-patch.md)。规格与下面原文冲突时以规格为准。
**审计（2026-10-11）：到 P2 再做；「对密文打补丁」不成立。** 见 [ref-ime-impl/README.md](ref-ime-impl/README.md#审计2026-10-11)。

```
背景：设计里「包带版本号，同步走增量：云端按版本差给补丁，失败退回整包」还没实现。
RefIME 的做法：index.json 给每个数据文件记 name / md5 / version / support_update；更新时先下补丁（bsdiffplus::BsPatch），
打完补丁校验 md5，不对就下整包；新文件放到带版本号的目录里，切换是改指针，旧版本留一份用于回退（和 A1 配合）。

任务：给素笺的包同步加增量补丁。
- 服务端（synon-ime）：每次打包保留最近 K 版（K=3），按 (from, to) 生成 bsdiff 补丁并缓存；接口返回 {to_version, patch_url?, full_url, sha256}。
- 客户端：有 patch 先打补丁、校验 sha256，失败或缺 from 版本就下整包；写到 <包名>/<版本>/，原子切换当前版本指针，保留上一版。
- 补丁库选纯 Rust 实现（qbsdiff 之类，许可要和 GPL-3.0 兼容），客户端打补丁放在 Rust 侧供 iOS / Mac 共用。
- 包是加密的：补丁对密文做（服务端只见密文），这样不需要在服务端解密。
验收：服务端单测（补丁可逆、sha 不符时退整包）；客户端用三版合成测试包验证 v1→v3 走两次补丁与直接整包结果字节一致；记录补丁体积占整包的比例。
不做：P2P 分发；后台定时轮询以外的推送通道改动。
```

## A3 叠加层做成会话包装层

复刻规格（反汇编取证 + 青简落点）：[ref-ime-impl/a3-session-layers.md](ref-ime-impl/a3-session-layers.md)。规格与下面原文冲突时以规格为准。
**审计（2026-10-11）：按分期做（ScopedLearner 已有）。** 见 [ref-ime-impl/README.md](ref-ime-impl/README.md#审计2026-10-11)。

```
背景：设计里 OverlayLearner / PersonaSpec / HintIndex 都要插进 Engine。RefIME 把繁简、emoji、计算器、敏感词、云结果合并等横切功能
都写成一层层包住核心拼音会话的 wrapper（SessionCreator 按顺序套，核心搜索在最底层不感知），开关一层不影响别层。

任务：在实现 P0 的 Core 接口时，评估并给出「叠加层 = 包装层」的落地方案，先写设计再动代码。
- 读 crates/qingjian-core/src/engine/（mod.rs、learning/、privacy.rs、session.rs），列出现在横切逻辑散在哪些分支里。
- 方案要回答：OverlayLearner 包 Learner（设计已定）之外，人设词库加权与 HintIndex 匹配放在哪一层；私密输入（set_private）怎样一次性旁路所有叠加层；
  切换 LearningScope 时各层如何清状态。
- 不要为了套模式重写现有 Engine；只在新增的三个能力上用这种组织，存量不动。
验收：docs/design/relationship-memory.md 的「Core 的改动」一节补上分层图与接口签名；apps/cli 的 --overlay 回放能分别开关每一层。
```

---

## B1 纠错 / 模糊音统一成带标记的词图边，标记进排序

复刻规格（反汇编取证 + 青简落点）：[ref-ime-impl/b1-augmenter-marks.md](ref-ime-impl/b1-augmenter-marks.md)。规格与下面原文冲突时以规格为准。
**审计（2026-10-11）：搁置，用户可感知部分由 D1 吸收。** 见 [ref-ime-impl/README.md](ref-ime-impl/README.md#审计2026-10-11)。

```
背景：青简现在「整段一处编辑」（correction/）和「音节级敲错边」（correction/typo）是两套机制，整段那路逐变体跑整句，长句几毫秒。
RefIME 把 28 种变换（邻键、换位、漏敲、多敲、前后鼻音、平翘舌、h/f、un/ong、补全……）写成同一个 trie 搜索模板上可组合的「增强器」，
每条路径记下用了哪些变换（AugmentRecord），排序时变成特征（exact_with_typo、lm_exact_after_typo、user_typo……），而不是每种变换一个固定代价。

任务：设计并原型验证「变换 = 词图边上的标记」。
- 先读 crates/qingjian-core/src/{parser,correction,fuzzy,sentence/viterbi}，写清楚现在三种来源（模糊音、敲错、整段编辑）各自在哪一步展开、代价怎么进 Viterbi。
- 原型：音节边带一个小的变换标记集合（bitset），代价 = 基础代价 + Σ 标记权重；权重表先用现有常数初始化，保证改造前后回放结果一致（行为不变的重构），再单独一步调权重。
- iOS 特有：邻键变换按 iOS 键盘布局（26 键）给，和 Mac 的物理键盘分开一张表。
验收：第一步重构 apps/cli 回放与门槛全部 +0.0；第二步调权重按 docs/plan/model-eval.md 写尺子，报召回与误纠率；记录 p50 / p95 逐键耗时，长句（≥ 10 音节）不得变慢。
不做：引入 LightGBM 等学习排序（需要大量标注日志，且不能用跨用户数据）。
```

## B2 触摸坐标个性化（iOS）

复刻规格（反汇编取证 + 青简落点）：[ref-ime-impl/b2-b3-touch-and-beam.md](ref-ime-impl/b2-b3-touch-and-beam.md)。规格与下面原文冲突时以规格为准。
**审计（2026-10-11）：搁置（无用户反馈）。** 见 [ref-ime-impl/README.md](ref-ime-impl/README.md#审计2026-10-11)。

```
背景：RefIME 引擎里有 MemCacheUserCoordinateDict 和 coordinate_score_abbr 特征：记录用户每个键实际点在哪，
把「点在 F 和 G 交界」的概率分给两个键，再进候选打分。手机触屏的敲错主要来自这个，物理键盘上没有。

任务：给素笺 iOS 键盘加按用户学习的触点模型。
- 壳把每次按键的触点坐标（相对键中心的偏移，归一化到键宽高）连同落定的字母交给 Core；Core 只收「这次按键对各候选字母的概率」，不关心坐标系（保持平台无关）。
- 模型：每个键一个二维高斯（均值、协方差），用户确认上屏的输入才更新（选了候选 = 确认）；冷启动用键中心 + 固定方差。
- 概率接到 B1 的邻键变换上：邻键代价 = −log P(邻键 | 触点)，没有 B1 时先接到现有敲错边。
- 数据只存本地，进私密输入时不学；属于基础学习层，不进分区、不上传。
验收：用合成触点（键中心 + 偏移噪声）的单测验证收敛；真机打 200 字对比开关前后的首选命中率与退格次数（docs/plan/model-eval.md 的写法）。
不做：上传坐标；跨设备同步触点模型。
```

## B3 整句 beam 宽按位置衰减

复刻规格（反汇编取证 + 青简落点）：[ref-ime-impl/b2-b3-touch-and-beam.md](ref-ime-impl/b2-b3-touch-and-beam.md)。规格与下面原文冲突时以规格为准。
**审计（2026-10-11）：搁置（根因是格子准入，不是 beam）。** 见 [ref-ime-impl/README.md](ref-ime-impl/README.md#审计2026-10-11)。

```
背景：RefIME 的 BeamSizeSchedulerFactory 有 CosineDecay / ExpDecay / WarmUp* 几种调度：句首保留宽 beam，越往后越窄；
简拼、中英混、用户词各自再乘一个比例。长句延迟直接受益，iOS 上更明显。

任务：给青简整句搜索（crates/qingjian-core/src/sentence/viterbi）试位置衰减的剪枝宽度。
- 先确认现有剪枝方式（固定 beam / 每节点保留 top-k / 无剪枝），找到唯一的宽度常数。
- 先只做一种线性衰减：width(i) = max(min_w, w0 − slope·i)，三个常数可配；简拼输入单独一个倍率。
- 用 apps/cli 常数扫描在冻结日志上扫，同时记 p50 / p95 / max 逐键耗时。
验收：精度门槛不掉（±0.0 以内）且长句 p95 下降才合；结果写进 docs/notes/constant-sweep.md。
```

## B4 光标前文本的模型状态跨按键缓存

复刻规格（反汇编取证 + 青简落点）：[ref-ime-impl/b4-b6-context-cache-and-tiers.md](ref-ime-impl/b4-b6-context-cache-and-tiers.md)。规格与下面原文冲突时以规格为准。
**审计（2026-10-11）：先做（iOS 通变每键整段重算）。** 见 [ref-ime-impl/README.md](ref-ime-impl/README.md#审计2026-10-11)。

```
背景：RefIME 的 StreamingNNLMModel 有 SetTextBeforeCache / LookUpLMScoreInCache，E2E 模型有 PrefillContext / CopySavedCacheToTensor：
光标前文本（上文）的 KV cache 算一次，同一输入框里后续每次按键直接复用，只算拼音部分。
素笺已经有「词级前文 + 知微重排」，上文每次重算是浪费。

任务：在 crates/qingjian-neural 里加上文 KV cache 的跨按键复用。
- 现状：先读 p2c.rs / scorer.rs / continuation.rs，确认拼音前缀 KV 已共享到什么程度、上文是怎样拼进输入的。
- 缓存键 = 上文 token 序列的哈希；上文变了（上屏、光标移动、换输入框、set_learning_scope、set_private）就失效。
- 缓存只存一份（当前输入框），内存上限按 iOS 键盘扩展预算定，超了就不缓存。
验收：同一上文下连续 10 次按键，第 2 次起的重排耗时下降多少（真机 Release 日志）；输出与不缓存时逐位一致（单测）；内存峰值不超过现在 + 预算。
```

## B5 晚到结果的插位规则

复刻规格（反汇编取证 + 青简落点）：[ref-ime-impl/b5-late-merge.md](ref-ime-impl/b5-late-merge.md)。规格与下面原文冲突时以规格为准。
**审计（2026-10-11）：与 D2 一起做（#341 要相反方向）。** 见 [ref-ime-impl/README.md](ref-ime-impl/README.md#审计2026-10-11)。

```
背景：RefIME 云候选默认等 200 ms（服务端可调），超时后才到的结果走 MergeTimeoutCloudFromFourth，只能插到第 4 位以后；
本地首选（IsProtectLocalFirst）和用户固顶的词（IsHardFixedByUser）不会被挤掉。用户已经看到的前几位不再变动，避免「正要按 1 结果首选变了」。
素笺里晚到的有：知微异步重排结果、联想、将来 P3 的 /rewrite。

任务：把「结果晚到时能动哪些位置」写成 Core 里一条统一规则。
- 读 crates/qingjian-core/src/engine/prediction/（policy.rs、cloud_word.rs）和 rescoring/，列出现在每种晚到结果怎么并进候选。
- 规则：候选窗已显示超过 X ms（或已经翻页 / 高亮移动过）之后到的结果，不得改变前 K 位（K=3 起，常数可配）；用户选过、固顶的词永远不被挤；
  在时限内到的结果可以正常重排。
- 规则放 Core，壳只负责报告「候选窗何时显示、用户是否动过高亮」。
验收：单测覆盖时限内 / 时限外 / 固顶三种情况；真机快打对比误选（选完立刻退格）次数。
```

## B6 设备分档

复刻规格（反汇编取证 + 青简落点）：[ref-ime-impl/b4-b6-context-cache-and-tiers.md](ref-ime-impl/b4-b6-context-cache-and-tiers.md)。规格与下面原文冲突时以规格为准。
**审计（2026-10-11）：搁置，等 B4 后实测。** 见 [ref-ime-impl/README.md](ref-ime-impl/README.md#审计2026-10-11)。

```
背景：RefIME 有 HighDeviceLtrValid、enable_low_performance：按设备性能决定开哪些模型。
素笺 iOS 上模型加载 53–90 ms，老机型和内存紧时可能更差。

任务：给 iOS 键盘加一个性能档位，决定知微重排与上文预测是否开启、beam 宽取哪组常数。
- 档位按「首次加载耗时 + 最近 N 次逐键 p95 + 可用内存」在本机算，不用机型白名单；结果存 App Group，每次大版本更新重算。
- 低档：关知微重排，保留 bigram 整句；中档：知微只在停手后重排；高档：现状。
- 档位只影响性能相关开关，不改学习与叠加逻辑。
验收：在最老的支持机型（或模拟内存压力）上验证低档 p95 达标；设置页能看到当前档位并手动覆盖。
```

---

## C1 用户词自动清理与热词发现

复刻规格（反汇编取证 + 青简落点）：[ref-ime-impl/c1-c3-cleanup-offline-space.md](ref-ime-impl/c1-c3-cleanup-offline-space.md)。规格与下面原文冲突时以规格为准。
**审计（2026-10-11）：英文词部分并入 D3，其余到 P2。** 见 [ref-ime-impl/README.md](ref-ime-impl/README.md#审计2026-10-11)。

```
背景：RefIME 有 UserDictCleanStrategyMaker / DynamicTargetUserDictCleanStrategy（用户词按目标容量动态清理）与 UserHotWordDetector（自动发现常用新词）。
素笺的分区学习表要控制在 KB 级，本地个人英文词表也没有衰减（todo.md 2026-09-24）。

任务：给本地学习表（crates/qingjian-learning 的 FrequencyLearner 与个人英文词表）加衰减与容量上限。
- 计分 = 次数 × 时间衰减（半衰期常数），超过容量时按分数淘汰；用户手动加的词不淘汰。
- 云端分区学习表打包时用同一套计分（「最近 90 天 + 全期高频」的具体化），保证本地与包同构。
验收：apps/cli 回放显示清理前后首选命中率不降；学习表体积有上限；偏好设置能看到条数。
```

## C2 「完全离线」开关

复刻规格（反汇编取证 + 青简落点）：[ref-ime-impl/c1-c3-cleanup-offline-space.md](ref-ime-impl/c1-c3-cleanup-offline-space.md)。规格与下面原文冲突时以规格为准。
**审计（2026-10-11）：搁置。** 见 [ref-ime-impl/README.md](ref-ime-impl/README.md#审计2026-10-11)。

```
背景：RefIME 有 LocalMode：一个开关让所有联网功能（上传、语音、AI）跳过，日志写 skip reason=localMode。
素笺打字本来就全本地，但素材上传、包同步、记一笔都会联网；有些用户要一个明确的总开关。

任务：加「完全离线」开关：打开后停止素材上传与包同步（本地队列保留、不清），已同步的包照常用，界面标明「离线中」。
- 开关是设置项，不是模式；放 iOS App 的「我」与键盘长按菜单。
- 每个联网点统一过一个守卫函数，日志记跳过原因，便于验证没有漏网的请求。
验收：开关打开后抓包（用户自己操作）零请求；关掉后队列补传。
```

## C3 中英 / 数字之间自动空格

复刻规格（反汇编取证 + 青简落点）：[ref-ime-impl/c1-c3-cleanup-offline-space.md](ref-ime-impl/c1-c3-cleanup-offline-space.md)。规格与下面原文冲突时以规格为准。
**审计（2026-10-11）：先做：合上游 PR #372 再补两条边界，不自己写。** 见 [ref-ime-impl/README.md](ref-ime-impl/README.md#审计2026-10-11)。

```
背景：RefIME 的 smartSpaceText 在中文与英文、数字之间上屏时自动补空格。素笺中英混输时有用，属于小功能。
任务：在 Core 上屏路径加可配的「中英之间自动加空格」（[general] 设置，默认关），依据是上屏文本与光标前一个字符的类别。
注意：用户手打的空格不重复加；代码 / 终端类场景（iOS 拿不到宿主 App，就只提供开关）不自动加。
验收：单测覆盖中→英、英→中、中→数字、已有空格四种；docs/user/ 对应页同步。
```

---

## 对照上游 issue 新增的三条（2026-10-11 审计）

都有上游 issue、素笺能复现、原型已验证，优先级排在上面 12 条之前。规格里写了改哪些函数、常数、单测与回放数字，直接照做。

- **D1 模糊音按键级展开**（#319、#306 / PR #330）：[ref-ime-impl/d1-fuzzy-as-augmenter.md](ref-ime-impl/d1-fuzzy-as-augmenter.md)，原型 [proto/d1-fuzzy-sources.diff](ref-ime-impl/proto/d1-fuzzy-sources.diff)。
- **D2 云请求触发**（#177、#341）：[ref-ime-impl/d2-cloud-request-strategy.md](ref-ime-impl/d2-cloud-request-strategy.md)。
- **D3 学习闸门（英文词）**（#545、todo「个人英文词表没有衰减」）：[ref-ime-impl/d3-learning-gate.md](ref-ime-impl/d3-learning-gate.md)，原型 [proto/d3-english-gate.diff](ref-ime-impl/proto/d3-english-gate.diff)。
