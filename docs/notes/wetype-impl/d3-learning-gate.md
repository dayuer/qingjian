# D3：上屏后学什么、多快排第一、学错了怎么自愈（对应上游 #545 / #475 / #425 与 todo「个人英文词表没有衰减」）

取证对象：WeType 2.2.3 macOS arm64，纯静态。地址都是 vmaddr。凡是数值都是 **WeType 的值，青简要自己重扫**；
标「数据给」的是从随包 capnp 超参文件读的，本机没有那份文件，值未知。
中间产物：/tmp/d3/（UserSingleCnSelect 反汇编、复现脚本 pick.py / seq.py / rem.py / brute2.py、原型 diff 已存为 `proto/d3-english-gate.diff`、回放明细 det-*.jsonl）、
/tmp/d3/rank/（排序后处理器）、/tmp/d3/learn/（学习闸门与英文词库）。

**对前文的更正**：B2B3.md 把 `EmitInputEvent+0x74` 标成 learnable——**不对**。它是事件里内嵌的 `KeyBoardLayout`（起点 +0x50）的 `valid` 字段
（构造清零 0x1006341e0；`GetABCTestKeyboard` 置 1 0x100f473d4；云词事件写 0 0x100df2284；PinyinV2Session 某 emit 点写 0 0x100918db8）[确]。
`UserCoordinateDictWrapper` 里 `if(!e+0x74) return` 的意思是「没有键盘几何就不学坐标」。WeType 的上屏事件里**没有**私密 / 不学字段，
学不学由会话类型、候选 ID 的种类、输入串形态决定（§1）。

---

## 0. 结论速览

| 问题 | WeType | 素笺现状（HEAD dd1ce657） |
|---|---|---|
| 中文模式回车原样上屏的字母串学成英文？ | **不学**（ForceEmitCandidateID 被闸门拦）[确] | 学（`take_raw`，有长度 / 切分 / 分叉补丁三道过滤）；`sm`、`ia`、`ial`、`ac` 回车一次就学进去 |
| 中文模式选英文候选学？ | 独立英文搜索出的 `EnglishCandidateID` **不学**；拼音图里的英文边（TrieMatching 大写音节）学，key = 敲的串 [确] | 一律学（`commit_with` 的 English 分支），残段里选的也学 |
| 用户英文词什么时候出现在中文候选里 | 长度 < 5 且不在随包英文库 → **不展示** [确] | 学过就展示，`english_first` 只看「选过次数 ≥ 中文在这串下的选择数」 |
| 英文词被拒绝（在首位、用户选了别的）降权？ | **没有** [确：搜索阴性] | 没有扣分，但 `chosen <= english_weight` 是两边计数比较，中文选多了会让英文退回第 2 |
| 英文词衰减 / 删除 | StaticTarget：每学一个英文词顺带扫 10 个键，`freq<5 且闲置>30 天` 删（C1）；上屏后第一次文本变化是删除 → 撤销那次学习（英文 Reduce 2）；拉黑 | 只有删候选快捷键 `forget_english`；退格撤销不退英文 |
| 选一次多快排第一（拼音） | 用户词分数 1 次 ≈ −2.95、3 次饱和（−2.05，上限 −1.95）；能否到第一由 LTR 模型 + 后处理器决定，静态读不出次数 [确公式 / 推结论] | **一次**：`choice_bonus = 8·ln(1+n)`，1 次 = 5.5 nat，几乎总能翻；实测 `shi` 第 15 位的 式、`d` 第 19 位的 地 选一次就第一，重启后仍第一 |
| 单字 / 高频字特殊处理 | 单字用户词没有「长期热度 > 1.1 不降级」保护：闲置 > 60 秒且模型分差 > 3.0 就被挤回第二 [确] | 没有特殊处理 |
| 单字母简拼（`d`） | **不查用户词库**（GetJPLeaves 要求输入 ≥ 2 字母）[确]；单字选择记录 `UserSingleCnSelectDict` 只在五笔 / 笔画会话启用 [确] | 和多字母同一套：选一次就第一 |

上游三个 issue 在素笺上的状态（2026-10-11 实测，`./target/release/qingjian-cli --config /tmp/empty.toml`）：
- **#425（shi → 式）、#475（d → 的）不复现**：冷启动 `shi` 首选 是、`d` 首选 的；故意选别的字一次就到第一、带 `--user-dict` 重启后仍第一（§3.3）。
  上游评论说 0.1.5-beta.1 已不复现，与此一致。素笺这边的风险反过来：**一次误选就翻掉高频字**（`d` 误选第 19 位的 地 一次，地 就压过 的）。
- **#545 复现了两条路径**（§2.2）：① 分段上屏留下非拼音残段（`wolai` 选 我啦 → 残段 `i` → 唯一候选 I → 空格即学）；
  ② 回车原样上屏的短串（`sm` 回车一次 → 下次 `sm` 首选变成英文 sm，压过 什么）。

---

## 1. WeType：什么样的上屏会被学

### 1.1 学习闸门 `UserDictSingleEmitWrapper::Impl::onInputEmitted` 0x100947a98 [确]

```c
mode = impl->session_type;          // impl+0x191 = _wxime_session_config+4；config+0x20!=0 时改成 (config+4==0 ? 7 : 6)  0x10095702c
if (e.selections.empty()) {         // 没有选候选（纯原样上屏、标点）
    if (!e.text.empty() && mode == 4) UpdateUserStrokesDict(e);
} else if (mode in {0,1,6,7}) {     // 拼音类会话，掩码 0xC3
    if (IsSelectedIUV(sel)) return;
    UpdateNormalUserDict(e, sel);   // 中文用户词 + 中文会话里的英文词（路径 A）
    UpdateSyllableCorrectionDict(sel);
} else if (mode == 2) {             // 英文会话（SessionCreator 0x1006b9d40：type==2 建 UserDictEnSendWrapper）
    UpdateEnglishUserDict(sel);     // 路径 B
} else if (mode == 5) UpdateUserWubiDict(e);
UpdateUserEmojiDict(e);
```

### 1.2 中文会话 `UpdateNormalUserDict` 0x10094808c：逐个 CandidateSelection 判断，任一条不过就**整次上屏不学** [确]

1. `sel.input`（+0x18）或 `sel.word` 为空（0x1009483bc）。
2. `sel.input` 里 `,` 与 `#` 合计 > 8（0x1009487e0）。
3. `sel.input` 含 ASCII 字母与 `,` `#` 以外的字符（字母表 0x102eb9d50）。
4. 候选 ID 不是 `IsPinyinIDLike`：表 0x1031b03c0（逐项核过返回值）只有 variant 下标 **0 TrieMatching、1 Pinyin、13 Emoji、14 CandCompletion、16 TrieMatchingWithSyllableCorrections、28 BaseUserFix** 为真；
   **5 CloudEnglishSearch、9 EnglishCandidateID、10 ForceEmit（回车原样）都为假 → 不学**（0x100948aa0）。
5. 组合候选或 `RecordToUserDict(strategy)` 为假（0x100948a98 / 0x100948b94）。

过了闸门后判「是不是英文」（0x100949014）：`isEn = (sel.flags@+0x98 & 4) || 音节串（'#'→','）全是 'A'–'Z' 或 ','`。是英文时（0x10094926c–0x1009493ec）：

```c
key = tolower(sel.input);                          // 用户实际敲的串
w   = word 里的字母（表 0x102eb9d50 过滤）tolower;
if (key.size() < w.size() && w.starts_with(key)) key = w;   // 输了前缀就存整词
AddUserEnDict(d82, d8f, blocklist, sync, key, word, emit_type, recent, blockvec, weight = 1.0f);
```

`AddUserEnDict` 0x10094b67c：先 `recent.push_back({type=1,key,word})`（供撤销），`TryToRemoveFromUserBlockListDict`（重新学到就解除拉黑），
再写 `!env2!`（kind 0x82，中文里用）与 `!en_fe!`（kind 0x8f，全英文模式用）两个库。`UserEnglishDict::Add` 0x100774d84 → `Impl::Add` 0x100774e54：
key 逐字 tolower、word 保留大小写，投线程池 `SyncAdd` 0x100775220：先 `TryToRemoveNextSyllables(key, strategy, dry_run=false)`（英文库的 StaticTarget 清理，C1），
再 `rw->Add(key, WordItemV2(word, freq=1, time=now))`。**选一次就入库，没有潜在词两段式**（潜在词库 0x94 只给中文）[确]。

### 1.3 英文会话（与 #545 无关，记下备用）[确]

- 路径 B `UpdateEnglishUserDict` 0x100947c10：选候选即 `AddUserEnDict(dictId∈{3,0x1c} ? null : d82, d8f, key=sel.input, …)`，没有长度 / 像不像拼音检查。
- 路径 C `UserDictEnSendWrapper`（只写 0x8f）：`ProcessInput` 0x10095f1f0 攒 alnum / `'` / `-`；`UpdateEnglishUserDict(bool)` 0x10095ee50：
  `word.empty() || word == 光标前原有的词 || word.size() <= 2` → 不学，否则 `d8f->Add`（日志 `add word to user dict: `）。即**英文模式直接敲出的词要 ≥ 3 个字符**。

### 1.4 用户英文词在中文候选里的展示门槛 [确]

`SyllableGraph+0x2d0 = 5`（0x10090ac34）→ `EnglishEdge::SetMinLenForSelfMakingUserWord` 0x100ac0d8c；`ViewEnglishSyllable` 0x100ac2ff8 起：
**用户英文词长度 < 5 且静态英文库查不到 → 不展示**；≥ 5 的参与排序。英文候选的位置由 `EnglishCandsAdjustStep3` 0x100ce1320 决定：
取第一个 `conf >= 0.3`（0x3e99999a）的英文结果，按它自带的 `expected_pos`（特征 key 44，`{u8 pos, f32 conf}`，0xff 当 20）上移，pos 可以是 0；
`expected_pos` 来自英文边结果（数据给 / 模型给，值未知）。代码里没有「像拼音就压后」的硬规则。

### 1.5 撤销与删除 [确]

- **上屏后第一次文本变化是删除 → 撤销那次学习**：`$_1` 0x100959018 挂在 TextAroundCursor 变化事件上，`type==1`（删除）→ `TryRevertLastRecord` 0x10094d4b0；
  `type==2`（及键盘类型 5/6 下的 `type==0`）保留记录；其他变化 → 确认学习、清记录。每次新上屏前也 Reset（0x100958ecc）。**没有时间窗，只看「下一次变化」**。
  策略 `impl+0x1f4`（AB `mmkb_cli_engine_user_dict_roll_back`，空则 `_pc` 版本；缺省 0）：0 = 中文 `Reduce(key,word,1)`、
  英文在 0x82 / 0x8f 各 `Reduce(…, 2)`（0x10094da44）；1 = 事务 RollBack；2 = 不撤。`WordItemV2::Reduce(n)` 0x1007745ac：freq / f20 / f28 各减 n，下限 0。
- **拒绝不扣分**：符号与字符串里搜 Reject / Punish / Penalty / Negative / Decrease / Demote / Skip 都无相关项；`EmitAndDropRecorder::Modify` 0x10094c42c 只记上报样本。
- StaticTarget 清理（C1 §1.3）：英文库每次 SyncAdd 都真删（dry_run=0）；拉黑表 0x96；`dirty value` 删除只作用于中文库 0x80 / 0x94。

### 1.6 证据表（学习侧）

| 结论 | 置信度 | 地址 / 字符串 |
|---|---|---|
| +0x74 = KeyBoardLayout.valid，不是 learnable | 确 | 0x1006341e0、0x100f473d4、0x100df2284、0x100918db8 |
| 学习闸门按会话类型分派 | 确 | 0x100947a98；SessionCreator 0x1006b9d40 |
| IsPinyinIDLike 只对 0/1/13/14/16/28 为真，English(9)/ForceEmit(10)/CloudEnglish(5) 不学 | 确 | 表 0x1031b03c0（逐项 `mov w0,#0/1; ret`）；variant 顺序见 nm 里 `std::variant<TrieMatchingCandidateID, …>` |
| 中文会话英文词 key = 敲的串，前缀则补全成整词 | 确 | 0x10094926c–0x1009493ec |
| 英文选一次即入库，两个库 | 确 | 0x10094b67c、0x100774d84、0x100775220；`!env2!` 0x100be9c88、`!en_fe!` 0x100be9e94 |
| 英文模式直接敲：长度 ≤ 2 不学 | 确 | 0x10095ef30 |
| 用户英文词 <5 且不在静态库不展示 | 确 | 0x10090ac34、0x100ac3018 |
| Step3 置信度 0.3、expected_pos 可为 0 | 确 / 值数据给 | 0x100ce1368、0x100ce13fc；GetEnglishResExpectedPos 0x100ce0fb8 |
| 撤销 = 下一次文本变化是删除；英文 Reduce 2 | 确 | 0x1009590a8、0x10094d7b0、0x10094da44 |
| 无拒绝降权 | 确（搜索阴性） | nmn_dem.txt / str.txt |
| query_str 是原始按键 | 推 | 坐标学习要求 `coords.size()==query_str.size()` |

---

## 2. 青简 / 素笺侧：#545 的复现与根因

### 2.1 现在会学成英文的三条路（代码）

| 路径 | 位置 | 过滤 |
|---|---|---|
| 回车原样上屏 | `crates/qingjian-core/src/engine/composing.rs` `take_raw` | `looks_like_english_word`（≥ 2 字母、全 ASCII 字母、中文模式下 ≤ 15 且不能整段切成完整拼音）、`decode` 不完整、`split_english_tail` 为空、分叉补丁「改一处成两音节以上完整拼音且不在随包表」 |
| 选英文候选 | `engine/commit/mod.rs` `commit_with` 的 `CandidateKind::English` 分支 | **无**：`learner.record(candidate)` + `note_english_commit` → `learn_english` |
| 整句末尾英文词 | 同上 `CandidateKind::Sentence` 分支 | `is_english_word` |

学进去之后：`engine/extras.rs` `insert_english` 里 `english_first = unlikely_pinyin && chosen <= english_weight && !short_acronym && !short_capitalized`，
其中 `short_acronym / short_capitalized` 只对 `english_weight == 0`（没选过）的词生效——**选过一次就绕过「两字母让中文」**。

### 2.2 复现（素笺 HEAD，冷启动、空配置）

1. **残段路径（#545 的 IA / I 就是这条）**：分段上屏时选了一个只覆盖前缀的候选，剩下的不是拼音。
   用冻结日志 `data/eval/input-log-2026-10-04.jsonl` 前 150 个键串，每串依次选第 2–12 个候选、看剩余缓冲区的首选（脚本 /tmp/d3/brute2.py，原型 CLI 加了「上屏后打印剩余」与 `:cur`）：

   | 键串 → 选的候选 | 残段 | 残段首选 |
   |---|---|---|
   | `wolai` → 我啦、`fufei` → 负分 | `i` | **I**（English，唯一候选） |
   | `damox` → 大妈 | `ox` | **ox**（English） |
   | `macos` → 吗 | `cos` | **cos**（English） |
   | `shangchaun` → 上 | `chaun` | **Chauncey**（英文补全） |
   | `dachulai` → 打车 | `ulai` | 无候选（回车即原样上屏） |
   | `ia`（单独敲） | — | **IA**（English，唯一候选） |

   残段首选是英文、用户按空格 → `commit_with` English 分支无条件 `learn_english`。`shengxia` 在冷启动前 60 个候选里只留下 `''` / `a` / `xia` / `gxia` 残段，
   没直接造出 `ia`；但 `shengx` 一类前缀候选（升学、盛行按 `sheng x…` 读）消耗 `shengx` 就会留下 `ia`，机制相同。
2. **回车短串路径（Sm 就是这条）**：`sm` 回车一次 → 个人英文词表多一条小写 `sm` → 下次 `sm` 首选变成英文 sm，什么 掉到第 2；
   `ia` 回车 → 学成 `ia`；`ial` 回车 → 学成 `ial` 且下次排第一；`ialai`、`ulai` 不学（分叉补丁「改一处成完整拼音」挡住）。
   小写 `sm` 绕过了 `short_capitalized`（它看的是学到的写法，随包里是 Sm）。
3. **恶性循环**：英文候选一旦排第一，空格上屏又走 `record + learn_english`，`english_weight` 继续涨，`chosen <= english_weight` 永远成立。
   没有任何衰减；退格撤销（`apply_retraction`）只退中文的 `record / record_choice`，不退 `learn_english`。

本机实况（只统计条数，不列词）：`~/Library/Application Support/Qingjian/user-english.tsv` 89 条，其中 ≤ 2 个字母的 18 条（10 条与随包写法完全一致，8 条是大小写不同的变体）、
3–4 个字母且不在随包表的 5 条。

### 2.3 WeType 与素笺的差异，就是 #545 的修法

- WeType 中文模式**回车原样上屏不学英文**，**独立英文候选也不学**，只学拼音图里英文边、key 用敲的串；用户英文词 < 5 个字母又不在随包表的根本不展示。
- 素笺有意学回车串（`gist`、`python` 这类随包表没有的词只能靠它），不照搬「不学」，改成照搬 **「短词门槛 + 展示门槛」**，再加一条 WeType 没有、素笺需要的 **残段不学**。

---

## 3. WeType：选一次之后多快排第一

### 3.1 用户词的四个量与打分 [确]

`WordItemV2`（UserDictV2 内）：`+0 u32 freq`、`+4 u32 last_time(秒)`、`+8..+0x1c u32[6]` 按来源计数、`+0x20 f32 f20 长期热度`、`+0x24 u8 来源`、
`+0x25 u8 标志`、`+0x28 f32 f28 短期热度`、`+0x30 string word`。新建 0x100773e94：`freq=n, f20=max(opt.f4, n), f28=n`。
合并 `SoftMergeFrom` 0x100774104（调用者 `UserDictReaderWriterV2::Impl::Add` 0x100766498）：

```c
self.freq += o.freq;
dt = |self.last - o.last|;
d_long = exp((dt / -86400.0) / k);  d_short = exp(dt / -10.0);     // k=1；AB mmkb_cli_engine_user_dict_merge_strategy==1 时按词长调
if (self.last > o.last) { self.f20 += o.f20*d_long; self.f28 += o.f28*d_short; }
else { self.last = o.last; self.f20 = self.f20*d_long + o.f20; self.f28 = self.f28*d_short + o.f28; }
```

查词打分 `UserDictLookuper::LookUpByPinyinWithParams` 协程体 0x100883958（常数是 `UserDictV2HyperParams` 构造 0x10075ccf0 的代码缺省值，proto 字段 ≤ 1e-4 时用它们；数据给的线上值未知）：

```c
age = clamp(now - item.last, 1, T=2592000);           // 时钟回拨用 86400
s9  = item.f20 * exp(age / -86400);                    // 长期热度（τ=1 天）→ 特征 F22
s10 = item.f28 * exp(age / -10);                       // 短期热度（τ=10 秒）→ 特征 F33
if (s9 > hot=0.9) {
    x = s9 > cap=2.8 ? cap + 0.0504*ln(s9 - cap + 1) : s9;
    base = -3.0 + (-1.95 - -3.0) * (x - 0.9) / (3.0 - 0.9);
} else base = -3.0 + (-6.0 - -3.0) * (exp(age/T) - 1) / (e - 1);
fn = log10(min(freq, 500)) / log10(500);
bonus = s9 >= hot ? 0*fn + 0.01/age : (age < 7d ? 0 : 0.8333 * (base(age=1d) - base) * fn);
score = base + bonus;                                   // 约 -6 .. -1.95
```

数值：选 1 次 10 秒后 −2.949、1 小时 −2.97、1 天 −3.059、7 天 −3.459、30 天 −6.0；1 分钟内累计 f20=2 → −2.451、3 → −2.045，之后几乎封顶。
**用户词这条分数选到第 3 次就饱和**。这只是 LTR 模型的一个输入（特征 F9 年龄秒、F10 freq、F22 s9、F33 s10、F21 按来源次数），模型权重数据给。

### 3.2 后处理器（`MixRankerPostProcessorV2::Run` 0x100cd8940）[确]

顺序：`SetUserWordProb`（AB `mmkb_cli_engine_user_word_prob_expt`，日测缺省 `3;100;1800;-1;-1`）→ `OnlineFreqReRankProcessor(V2)` →（去重）→
`AdjustSelectedSyllableHint` → `Init` → `RemoveUnusefulUserDictRelated` → `RecallModel` → `TypoCandidate` → `CorrectedSyllableCandidate` →
`if (!AdjustUserDictProcessor()) { FirstUserCandidateRanker(); CheckUserDictSpurHotVal(); RepairUserWord(); }` → `PrecisionLoss` → `RankModel` → 英文 Step1/2/3 → 过滤。
首选由 `CandidateV2Ranker::GetAndPopTop` 0x100a36fe4 取 **priority（+0x314）最大**者。`kOnlineFreqReRankProcessorV2` 0x1033e4b28 只是调试标签串，不是开关。

- **AdjustUserDictProcessor** 0x100cdbbe0（强制提前）：首位不是补全（F16/F17=0）时，往后找同长度（F12 相同）的主用户词（kind 0x80）；
  `allow = F23==3 || hc[3]>hc[0] || ((首位单字 || 首位是英文) && (F23==1 || hc[1]>hc[0]))`（hc = 按来源次数），且 `c.F2 > top.F2`（推：加种类分前的综合分），
  且在前 50 个里位置关系满足（`dup`）→ `priority = 2`，**直接成为首选**。
- **FirstUserCandidateRankerProcessor** 0x100cdc7b4（陈旧降级）：首位是用户词、不是英文库、`F9 > 60` 秒、模型分 `s9(第二名) > s8(首位) + gap(3.0)` → 第二名 `priority = 1`。
  保护条件 `protect = utf8len(top.word) >= 2 && top.F22 > hot(1.1)`（0x100cdc9f4）——**单字用户词没有这条保护**。gap / hot 的覆盖值数据给。
- **CheckUserDictSpurHotVal** 0x100cdcbfc（连击通道）：前 5 名里短期热度 F33 > 1.2 的用户相关候选，写 `spur = 首位F22 − 它的F22 + 0.001` 进候选 ID（+0x4c），
  上屏时 `opt.f4 = max(spur, 1)`（0x1009495a4），新词条 `f20 = max(f4, freq)` → 一步抬过首位。触发约等于**几秒内连选同一个非首位候选两次**。
- **RepairUserWord** 0x100cdd0a8：首位用户词闲置 > 1800 秒且模型分落后（P3/P4 = −1）→ 第二名提前。
- `SingleUserWordCanbeSecond` 0x100cde918：单字版降级逻辑，**无调用者**（全文件无 bl/b）。

### 3.3 单字与简拼 [确]

- 单字母简拼：`UserDictSyllableGraph::Impl::GetJPLeaves` 0x100cab3cc：`GetStartSearch()==0 && InputSize() >= 2` 才查用户词库简拼叶子，
  输入长度上限 12（某参数为真时 5；键盘类型 5/6 不限）。**`d` 一个字母时根本不查用户词库**，用户词提权对单字母简拼不起作用。
  静态候选是否另带用户历史特征未确认（未知 7）。
- `UserSingleCnSelectDict`（单字选择记录）：`UserSingleCnSelectWrapper::Impl::SelectCandidate` 0x10095ae48 只在上屏**一个 utf8 字符**（键盘类型 5 时不限）时写；
  key = `getRealKey(kb_type, 输入串, 双拼方案)`；payload：`byte0 = 版本 1`、`byte1..5 = 最近 5 次选中的条目序号（1 起，最新在前；`UpdatePayloadByAddInfo` 0x1008a7c30 每次右移一格）`、
  之后每条 `u8 len,拼音, u8 len,字, {u32 count, u32 last_time, u16 0xffff, …}`，按字匹配（`MergeFrom` 0x1008a6a88：count+1、last=max）。
  查询 `get_recent_selected_idx_from_ptr` 0x1008a8794：策略 1 = 最近 3 次相同；**策略 2 = 最近 4 次里同一条 ≥ 3 次**；策略 3 = 最近 5 次里 ≥ 4 次；命中就返回那一条（单字置顶）。
  缺省策略 0（ctor 0x1008a67d8 `str wzr,[x19,#0x60]`）= 关闭；只有 `WubiSessionV2::Impl::Impl`（0x100b7ed44）与 `StrokeSession::Impl::Impl`（0x100ba8aec）调 `SetStrategy(2)`，
  读取方 `TryGetSingleCnSelectCand` 只被五笔 / 笔画会话调用（0x100b79b74、0x100ba5104）。**拼音不用它**。

### 3.4 素笺实测：选几次排第一

工具：/tmp/d3/pick.py、/tmp/d3/seq.py 驱动 `qingjian-cli` 交互模式（数字键上屏，同一进程内学习；带 `--user-dict` 时退出写回），另在临时工作树写了引擎测试
`d3_how_many_picks_until_first`（样例词库故意让 式 > 是、地 > 的 的静态词频，`CountingLearner`）。

| 场景 | 结果 |
|---|---|
| `shi` 选第 15 位的 式 | 选 1 次后第 1；`--user-dict` 重启后仍第 1 |
| `d` 选第 19 位的 地 | 选 1 次后第 1 |
| 小鹤双拼 `ui` 选第 16 位的 式 | 选 1 次后第 1 |
| `d`：先选 地 3 次，再改选 的 | 的 要选 3 次（3:3 平局时静态分更高的 的 赢），第 4 次查询回到第 1 |
| 选过 式 后 `zheshishenme` / `woshixuesheng` | 整句仍是 这是什么 / 我是学生（词级选择不污染整句） |
| 引擎测试（样例词库，静态首选是错的） | `shi→是`、`d→的` 名次序列都是 [2, 1]；反向回选 式 也是 [2, 1] |

机制（`crates/qingjian-core/src/engine/query/phonetic.rs` → `ranking::rank`）：得分 = 上下文 log 概率 + `choice_bonus(同输入串下选过次数, 8.0)`（= 8·ln(1+n)）
+ `weight_bonus(全局次数)`（0.5·ln(1+min(n,20))）− 扣分。一次选择 5.5 nat，比 WeType 用户词分数线（选 1 次 −2.95，对静态分要靠模型）激进得多。

---

## 4. 青简落点

约束对照：全部改动在 Core（`qingjian-core` 判定 + `qingjian-learning` 存储），平台层不动；一个类型一个文件；判定都在上屏路径上 O(词长)，不阻塞按键；只学本机。
用户可感知（英文候选出不出、排第几、数据文件格式）→ 同一提交改 `docs/user/` 对应页（英文混输与学习的那页，先 grep `user-english` 定位），改了学习实现要点同步 `docs/notes/crate-notes.md`。

### (a) 英文学习入口的过滤规则

新增 `crates/qingjian-core/src/engine/learning/english_gate.rs`（一个自由函数 + 一个枚举，放 `learning/` 目录；枚举单独 `english_origin.rs`）：

```rust
/// 这次上屏的英文是从哪条路来的。
pub(in crate::engine) enum EnglishOrigin { Raw, Candidate, SentenceTail }

/// 英文模式外，要不要把 `word` 记进个人英文词表。`remainder`：同一段拼音里前面已经上屏过词，这是剩下的残段。
pub(in crate::engine) fn english_learnable(engine: &Engine, word: &str, origin: EnglishOrigin, remainder: bool) -> bool;
```

判据（中文模式；英文模式照旧全学）：

| # | 规则 | 出处 | 常数 |
|---|---|---|---|
| 1 | `remainder`（`self.chain.same_buffer()` 为真时上屏的英文候选 / 回车串）→ 不学 | 自定（#545 根因；WeType 读不到残段场景，未知 9） | — |
| 2 | 回车原样上屏：长度 < `MIN_RAW_ENGLISH_LETTERS` → 不学 | WeType 英文模式直接敲的门槛是 ≥ 3（0x10095ef30）；中文模式 WeType 干脆不学 | 3 |
| 3 | 回车原样上屏：长度 < `MIN_UNLISTED_ENGLISH_LETTERS` 且不在随包英文表 → 不学 | WeType 展示门槛 5（0x10090ac34），用在学习侧更早挡掉 | 5 |
| 4 | 回车原样上屏：现有三道（`looks_like_english_word`、`split_english_tail`、分叉补丁 `looks_like_pinyin_typo`）保留 | 素笺现有 | — |
| 5 | 英文候选 / 句末英文：只挡规则 1；长度 ≤ 2 的照学但不让它抢第一（见 (b′)） | WeType 中文模式不学独立英文候选；素笺保留学习（`ok` → OK 要能记住） | — |

`composing.rs::take_raw` 与 `commit/mod.rs::commit_with`（English 分支与句末英文）改成调 `english_learnable`；不学时仍走 `reset_bip39_streak`，
英文候选上屏时 `learner.record(candidate)` 也一并跳过（否则 `english_weight` 照涨）。BIP-39 计数不受影响（`mnemonic::is_bip39_word` 照常计）。

**(b′) 两字母英文抢第一要选两次**（`extras.rs::insert_english`）：`short_acronym / short_capitalized` 的条件从 `english_weight == 0` 改成
`english_weight < SHORT_ENGLISH_FIRST_PICKS`（2），并且 `word` 用随包表的写法判大小写（小写学到的 `sm` 也按随包 `Sm` 判）。出处：自定；
对应 WeType「短英文词不靠一次选择上位」（展示门槛 5 + 位置由数据给的 expected_pos / conf ≥ 0.3 决定）。没原型、没回放，见 §5。

### (b) 选择提权规则

**结论：不加单字 / 简拼专用记录，不改现有加分。** 理由：
- #425 / #475 在素笺不复现，现规则一次就翻（§3.4）。
- 试过照 WeType 的保守做法（单字母简拼第一次选择不算，第二次才加分）：冻结日志单字母词选 24 条原本 24 条首选，改后 **23 条**（`l` → 了 退到第 3），
  词首选 920 → 919 / 1057；本机日志 819 → 819 / 984。数据不支持，已撤回。
- WeType 的 `UserSingleCnSelectDict`（3 / 4 次多数票置顶）只用于五笔 / 笔画；素笺形码已经是 `rank_choice_first`（选择次数压过上下文），不需要。

可选的小改（不必做，要做就先回放）：对 **单个汉字 + 单字母输入**，误选防护靠已有的退格撤销（`apply_retraction`）；如果要更保守，
照 WeType 的「陈旧降级」写成：同输入串下 `choice == 1` 且上次选择距今 > 60 秒、静态首选与它的上下文分差 > 3.0 nat 时，加分按 `choice_bonus(0)` 算。
需要给选择表加「最后选择时间」（C1 的 `last_day` 列不够细，要秒），代价大于收益，**不建议**。

### (c) 英文词表的降权 / 衰减

1. **退格撤销也退英文**（照 WeType 撤销语义，[确] §1.5）：
   - `crates/qingjian-core/src/engine/commit/last.rs` 的 `LastCommit` 加字段 `pub english: Option<String>`（这次上屏学进个人英文词表的词）。
   - `Learner` 加 `fn unlearn_english(&mut self, _word: &str) {}`（`learning/learner.rs`；`MutedLearner` 转发；`FrequencyLearner` 在 `learner_impl.rs` 实现：次数 −1，到 0 删除，`english_dirty = true; rebuild_english()`）。
   - `composing.rs:149` 退格计数处：这条上屏刚好删干净（`is_erased()` 变真）且 `english.is_some()` → `unlearn_english` 一次。
     WeType 是「下一次文本变化是删除」就撤；素笺以「整个删掉」为准，免得删一个字母就撤。
2. **衰减 / 删除**：照 C1 的 `RetentionPolicy::STATIC_ENGLISH`（`freq < 5 且闲置 > 30 天` 删，每学一个英文词顺带看后面 10 个键，`crates/qingjian-learning/src/retention/`），
   需要 `user-english.tsv` 加第三列「最后使用日」，旧两列文件读入时填加载当天。规格与单测在 C.md §C1，不重复。
3. **被拒绝让位**（todo 那条；WeType 没有，自定）：`insert_english` 里精确英文词不排第一时，同一段字母下中文首选的选择数比英文选过次数多 `ENGLISH_DEMOTE_MARGIN`（3）次以上，
   英文从第 2 位让到第一页第 5 位：
   ```rust
   let demoted = !english_first && chosen >= english_weight.saturating_add(ENGLISH_DEMOTE_MARGIN);
   let mut position = if items.is_empty() || english_first { 0 } else if demoted { items.len().min(4) } else { 1 };
   ```
   原型回放见 §5：冻结日志不变；本机日志词平均名次 1.60 → 1.57、英文平均名次 1.67 → 2.17（首选 / 前五命中不变）。英文样本只有 6 条，信号弱。

### (d) 已污染用户的一次性清理

- 判定在 Core（要用音节表、随包英文表、`looks_like_pinyin_typo`）：`Engine::sanitize_user_english(&mut self) -> usize`（`engine/learning/` 下新文件 `sanitize.rs`，`impl Engine` 一块），
  对个人英文词表每条跑「(a) 的回车规则 2–4」（当作回车上屏的串来判，因为文件不记来源）：不过就 `learner.forget_english(word)`。
  另外保留：与随包表写法**完全一致**且长度 ≤ 2 的（`OK`、`I`、`W`）不删——它们排位已由 (b′) 管住。
- 只跑一次：`FrequencyLearner` 保存时给 `user-english.tsv` 写版本头 `# 青简个人英文词 v2：词\t次数[\t最后使用日]`；读到旧头时 `english_needs_sanitize() -> bool` 为真，
  Engine 在 `with_learner` / `set_learner` 后检查并清理一次、日志记 `清理个人英文词` 与条数。`Learner` 加 `fn english_needs_sanitize(&self) -> bool { false }` 与 `fn english_words(&self) -> Vec<String>`（只读列表）。
- 云同步合并（`frequency_learner/remote.rs` `merge_remote`）进来的英文词同样过一遍，否则别的设备会把碎片带回来：合并后 Engine 再调一次 `sanitize_user_english`。
- 本机文件按规则 2–3 粗算（Python 近似，未跑 typo 判定）：89 条里会删 ≤ 2 字母中写法与随包不同的 8 条 + 3–4 字母不在随包表的 5 条。

### 改哪些文件（汇总）

| 文件 | 改动 |
|---|---|
| `crates/qingjian-core/src/engine/learning/english_gate.rs`（新）、`english_origin.rs`（新）、`learning/mod.rs` | (a) 判定函数与枚举 |
| `crates/qingjian-core/src/engine/composing.rs` | `take_raw` 用判定；退格删净时 `unlearn_english` |
| `crates/qingjian-core/src/engine/commit/mod.rs` | English 分支、句末英文用判定；`remainder = split` |
| `crates/qingjian-core/src/engine/commit/last.rs` | `LastCommit.english` |
| `crates/qingjian-core/src/engine/extras.rs` | (b′) 两字母门槛、(c3) 让位 |
| `crates/qingjian-core/src/engine/mod.rs` | 常数 `MIN_RAW_ENGLISH_LETTERS = 3`、`MIN_UNLISTED_ENGLISH_LETTERS = 5`、`SHORT_ENGLISH_FIRST_PICKS = 2`、`ENGLISH_DEMOTE_MARGIN = 3` |
| `crates/qingjian-core/src/engine/learning/learner.rs`、`muted.rs` | `unlearn_english`、`english_needs_sanitize`、`english_words` |
| `crates/qingjian-core/src/engine/learning/sanitize.rs`（新） | (d) |
| `crates/qingjian-learning/src/frequency_learner/{learner_impl,tables}.rs` | 实现上面三个方法、版本头 |
| `crates/qingjian-core/src/engine/tests/english_learning.rs`（新） | 单测 |
| `docs/user/…`（英文混输 / 学习页）、`docs/notes/crate-notes.md`、`docs/plan/todo.md`（勾掉「个人英文词表没有衰减」的位置部分）、`cloud/docs/fork-patch.md`（分叉补丁表加行） | 文档同步 |

---

## 5. 原型与回放（已跑）

临时工作树 `/tmp/d3-proto`（HEAD dd1ce657，结束后已 `git worktree remove --force`），diff 存 `proto/d3-english-gate.diff`。原型只做了：
规则 1（残段不学，英文候选与回车两处）、规则 2 的简化版（回车：< 4 个字母不学，3 个字母要在随包表）、(c3) 让位；另有 CLI 调试改动（上屏后打印剩余、`:cur`）。
core 单测 365 条全过（含新加的 2 条原型测试）。

`qingjian-cli --config /tmp/empty.toml --replay <日志> --misses 0`：

| 日志 | 版本 | 词 首选 | 整句 首选 | 英文 首选 / 前五 / 平均名次 |
|---|---|---|---|---|
| 冻结 `data/eval/input-log-2026-10-04.jsonl` | HEAD | 920/1057 (87.0%) | 102/165 (61.8%) | 16/21 / 95.2% / 1.48 |
| 同上 | 原型（规则 1+2） | 920/1057 | 102/165 | **15/21** / 90.5% / 1.50 |
| 同上 | 原型（+ c3 让位） | 920/1057 | 102/165 | 15/21 / 90.5% / 1.50 |
| 本机 `input-log.jsonl` | HEAD | 819/984 (83.2%)，平均名次 1.60 | 86/143 | 3/6 / 100% / 1.67 |
| 同上 | 原型（规则 1+2） | 819/984 | 86/143 | 3/6 / 100% / 1.67 |
| 同上 | 原型（+ c3 让位） | 819/984，**平均名次 1.57** | 86/143 | 3/6 / 100% / **2.17** |
| 冻结日志 | 单字母第一次选择不算（已撤回） | **919**/1057 | 102/165 | — |

英文少的那 1 条（冻结日志第 1869 行）：`ac` 当时是回车学进去的小写 ac，原型不学两字母回车串后，`ac` 首选变成 啊吃、随包 AC 在第 3。
这是规则 2 的已知代价（两字母小写缩写回车后下次不再置顶），回车照样能打出 ac。正式实现按 §4(a) 规则 2 取 3 时，`ac` 仍不学，结果相同。

整句评测 `--eval-text data/eval/sentences.tsv`（1878 句）：HEAD 与原型都是首选 31.5%、字准确率 76.2%（冷启动没有学习，这几处改动不影响，预期如此）。

---

## 6. 复刻步骤

1. `english_gate.rs` / `english_origin.rs`：写 `english_learnable`，常数进 `engine/mod.rs`；`take_raw` 与 `commit_with` 改调它，`remainder` 取 `self.chain.same_buffer()`（`take_raw` 里要在 `chain.reset()` 之前取）。
2. `extras.rs::insert_english`：(b′) 两字母门槛按随包写法判大小写、`english_weight < 2` 才让中文；(c3) 让位。
3. `LastCommit.english` + `Learner::unlearn_english` + `composing.rs` 退格删净时撤销。
4. `sanitize.rs` + `Learner::english_needs_sanitize / english_words` + `user-english.tsv` 版本头；`with_learner` / `set_learner` 与 `merge_remote` 后各调一次。
5. （另起一个提交）C1 的 `RetentionPolicy::STATIC_ENGLISH` 与 `最后使用日` 列。
6. 回放冻结日志与本机日志、`--eval-text`，把前后数字写进 PR 与 `cloud/docs/fork-patch.md`。

单测清单（`crates/qingjian-core/src/engine/tests/english_learning.rs`，用样例词库 + 带 `learn_english / unlearn_english` 记录的 spy learner，参照原型 `d3_proto.rs`）：

| 输入 | 期望 |
|---|---|
| 样例词库含 `我啦\two la`、英文表含 `I`：`wolai` 选 我啦 → 残段 `i` → 选 I | 不调 `learn_english`，`weight("I")` 不变 |
| 直接敲 `i` 选 I | 调一次 `learn_english("I")` |
| `sm` 回车 | 不学；下次 `sm` 首选仍是中文 |
| `ial` 回车（不在随包表） | 不学 |
| `gist` 回车（随包表有） | 学 |
| `wxyzq` 回车（5 字母、不在随包、切不成拼音） | 学 |
| `ulai` 回车（残段） | 不学 |
| 英文模式 `sm` 回车 | 学（英文模式不受这些规则限制） |
| 英文表 `Sm\tsm`，选过 Sm 一次后敲 `sm` | 什么 第一、Sm 第二；选过两次后 Sm 第一 |
| 英文表 `OK\tok`，选过 OK 两次后敲 `ok` | OK 第一（保持上游「选过 OK 的用户下次还是它」，只是要两次） |
| `gist` 回车学进去后退格删净 | `unlearn_english("gist")` 一次，词表里没有 gist |
| `gist` 回车后只退一个字母 | 不撤销 |
| 精确英文词 `key`：中文 可以 在 `key` 下选过 4 次、英文选过 1 次 | key 在第 5 位（或末尾，候选不足 5 个时） |
| 同上，中文选过 3 次 | key 仍在第 2 位 |
| 旧版 `user-english.tsv`（无版本头）含 `sm`、`ia`、`ial`、`OK`、`gist` | 加载后清掉 sm / ia / ial，留 OK / gist；保存后带版本头，再加载不再清 |
| 云同步合并进来 `ia` | 合并后被清掉 |
| BIP-39 词连续回车三次 | 照常转私密输入（规则不影响计数） |

---

## 7. 未知

1. WeType 分段上屏后剩下的非拼音残段如何处理（会不会出现「只覆盖前缀、留下 `i`」的候选、残段能否被选成英文学进去）：没找到对应代码。
   下一步：`PinyinV2Session::selectCandidateImpl` 0x10091757c 里部分上屏后重建输入的地方，与 `ToOriginInputStr` 0x1009198bc。
2. FirstUser / AdjustUserDict 里特征 F2 / F12 / F16 / F17 / F25 / F36 的确切语义（没有 FeatureKey 名表）；用户词 score（−6..−1.95）如何进入综合分 f158（推：与静态 log 概率同尺度相加，再加数据给的种类分）。
   下一步：`BuildCandidateV2From` 0x100a4dd1c、`GetWordScoreInfo` 0x100a4ea14。
3. 因此 WeType「拼音选 N 次排第一」的具体 N 静态读不出（取决于数据给的 LTR 权重与种类分）。
4. `ranker.+0x80` 字符串（FirstUser 要求非空、Repair 要求空）是什么：疑为已选前缀 / 部分上屏上下文 [猜]。下一步看 CandidateV2Ranker 构造与写 +0x80 处。
5. AdjustUserDict 里来源编号 0 / 1 / 3 的含义（F23、F21 下标）：下一步看 `UpdateNormalUserDict` 0x1009489e4–0x100948a2c 的 `UserDictAddOption.byte0` 赋值。
6. `UserDictV2::Reduce` 除撤销外是否还有调用者（只经虚表 +0x5b8，未定位）。
7. 单字母 `d` 的静态候选是否另带用户历史特征（用户词库不查已确认）：下一步看 `CopyToCandidate` 0x100c8a264 的上游。
8. 英文会话 dictId 3 / 0x1c 是哪个库；`EnglishCandsAdjustStep1` 0x100ce04fc 没读完（从 0x100ce07b0 往后）；`expected_pos` 的产生者（`EnglishWithPinyinStrategy::GetResult` 0x1007a32ec）。
9. `UserSingleCnSelectWrapper::Impl+0x20` 那个输入串（AddInfo 的 key 来源）由 `PendingInputDetailInfoEvent` lambda 写入，未逐行确认是原始按键还是规范化拼音。
