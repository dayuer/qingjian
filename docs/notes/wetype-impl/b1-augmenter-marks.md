# B1 纠错 / 模糊音统一成带标记的边（WeType 2.2.3 macOS，graphs_v2 路径）

一句话结论：WeType 的音节搜索是 **LOUDS 音节 trie 上的 BFS**，每个节点带一个 7 位「纠错预算」（初值 61）和一条 5 字节的 `AugmentRecord`（变换类型 tag + 4 字节载荷）。
28 个增强器只是「在游标上试一步，过了门槛就生一个子节点」：**敲错类扣预算（邻键 28、换位 31、多敲 31、漏敲 / 拼写表 / 重复键 61），模糊音、大小写、ü 写法不扣**。
预算只用来限制组合；真正的排序代价在后面：每个音节把路径上的 record 数成 `RecordStatistics`（16 个 u16 计数），
打分时 `Σ 计数 × 权重`（权重在超参数据 `BeamSearchParamsProto` 里，不是代码常数），LTR 侧再把 record 串成 `r_q_w` 这类字符串特征和 `exact_with_typo` 之类的 0/1、计数特征。

---

## 1. WeType 怎么做的

### 1.1 入口与调用链

```
graphs_v2::Edge::InitializeSyllableEdge                 0x1008f6fb8
  └─ SyllableEdge::SyllableEdge(Cursor, string, int budget=61, IIA const&, HyperParams, EnvContext, BloomFilter, bool×6, EditPinyinDict*)   C1 0x1009a47d4 / C2 0x10099e6a0
       （调用点 0x1008f7250：mov w4,#0x3d → 初始预算 61）
  └─ SyllableEdge::SearchFromLeft(HyperParams, EnvContext, string_view, EditPinyinDict*)   0x1009a4b40（9.3 KB）
       ├─ SearchImpl<LoudsTrie::Cursor, AugmentRecord, int, IterativelyInputAugmenters<28 个>, $_2>(...)
       │     ├─ 节点生成器 lambda（下称 Emit）                                0x1009d2140
       │     │     └─ SearchNode::IsValid(parent, record, cursor)               0x1009b7368
       │     └─ IterativelyInputAugmenters::callImpl<3>（第 3..10 个增强器）  0x1009d26a0（11.8 KB）
       │           → callImpl<11> 0x1009beee4 → callImpl<21> 0x1009c325c → …（递归展开参数包）
       │           → RetroflexInputAugmenter::callImpl 单独出函数                0x1009be688
       ├─ PrismCompletionAugmenterIteratively::VisitAllCompletionCursor / IfProcessThisCursor（补全，前 3 个增强器内联在 SearchFromLeft）
       ├─ SearchContext::RemoveIfDuplicatedFrontiers / ShrinkFrontiers(HyperParams) 0x1009a4234 / ShouldRemoveThisNode 0x1009e2d74
       └─ LeafNodeAdder::Add 0x1009a8e3c → RecordStatistics(records) 0x1009ae664 → Syllable(...)
打分：
  searchers_v2::PathV2::CalculateScore 0x1009f3464
       └─ Syllable::Statistics::Score(BeamSearchParamsProto::Reader, bool, int, int, int) 0x1009f0414   ← 兜底线性打分
  LTR 特征：CandidateV2::SetEmMixTypoFeature 0x100a4bbb4、SetEmMixFuzzyFeature 0x100a4ba38、
            SetSentenceSearcherMultipleReplacementFeature 0x100a4a280、features::SyllableRecordsToString 0x100b0eee4、
            features::FuzzyRecordToString 0x100b107bc
```

开关：每个增强器一个使能位，在 `IterativelyInputAugmenters` 对象 +0x38 的 u32 位图里（位号 = 增强器在参数包里的下标，`tbz w10,#3` 测换位、`#4` 测邻键……）；
位图来自 `_wxime_input_augment_option`（会话构造参数），由 `details::AugmentTupleTypeMaker::Looper<N>::Create(option, unsigned& mask)` 0x100741ef8… 逐个填。

### 1.2 AugmentRecord（5 字节，紧凑 variant）

`sizeof(AugmentRecord)=5`：`vector<AugmentRecord>::__append` 0x10067dc1c 用 ×0xCCCD…（÷5），`emplace_back` 0x100d02ef4 拷 `ldr w`+`ldrb [+4]`。
`AugmentRecordCompactVariant<monostate, 30 种 Record>::Visit(unsigned char index, …)`：

| 字节 | 含义 |
|---|---|
| 0 | tag（variant 下标，0 = monostate / 根） |
| 1..4 | 载荷，按 tag 解释 |

tag 表（参数包顺序 = tag，ToJSON 名字来自各 `Record::ToJSON`）：

| tag | 增强器 | ToJSON type / 字段 | 载荷布局（[确] 来自 ToJSON / 生成点） | 窗口 NeedWindowSize |
|---|---|---|---|---|
| 1 | ExactMatch | `exact_match` {matched, is_selected, is_user_input_sep} | b0=字符, b1 bit0=is_selected, bit1=is_user_input_sep | {1} |
| 2 | InsertSep | `sep` {is_selected} | b0 bit? | {1,2} |
| 3 | PrismCompletion | `completion` {prefix, is_selected} | b0..b1 前缀字符, b2 低 6 位=长度, bit6=is_selected | {1,3} |
| 4 | Exchange | `xchg` {first} | b0=字符 | {2} |
| 5 | KeyboardNeighbour | `kb_neighbour` {original, replacement} | b0=敲的, b1=替成的, b2 bit0=标志（见 1.6）, b3=位置 | {1} |
| 6 | InsertAtMiddle | `insertion` {insertion, match} | b0=补进的字母, b1=匹配的输入 | {1} |
| 7 | Skip | `skip` {ch} | b0=跳过的输入字母 | {2} |
| 8 | Consonant | `fuzzy` | b0,b1=两个声母（FuzzyRecordToString 拼 `f_<a>_<b>`） | — |
| 9 | XnToXngNasal | `fuzzy` | — | {1} |
| 10 | Retroflex | `fuzzy` | b0=字母, b1 bit0=方向（z→zh / zh→z） | — |
| 11..20 | HuiFei, XngToXn, HuangToWang, WangToHuang, UnToOng, OngToUn, UnToIong, IongToUn, AnAi, EngOng | `fuzzy` | — | — |
| 21 | FullKBShiftKey | — | — | — |
| 22,23 | YvLike, LveAndNve | — | — | — |
| 24..26 | DoublePin, DoublePinPrismCompletion, DoublePinCombineFuzzy | — | — | — |
| 27 | ProcessUpperCase | — | — | — |
| 28 | KeyboardNeighbourForDoublePin | — | — | — |
| 29 | EditPinyinOnce | `edit_pinyin_once` {pos, type, letter} | u16 位段（bit12-13=type 0/2）, b2=letter | — |
| 30 | RepeatPrevChar | `repeat prev` {ch, exact_match} | b0=字符, b1=匹配 | — |

注意：**名字和直觉相反**——`InsertAtMiddle` 是「漏敲」（在 trie 路径上补一个字母，特征缩写 `i`），`Skip` 是「多敲」（跳过一个输入字母，缩写 `d`=deletion）。
「模糊音」类 = tag 位掩码 `0x041fff00` = {8..20, 26}（`FuzzyRecordToString` 0x100b107ec 与 `RecordStatistics` 都用这个掩码）。

### 1.3 SearchNode 与预算

节点由 Emit（0x1009d2140）在 `make_shared` 块里就地构造，对象从 +0x18 起，**packed**：

```
struct SearchNode {            // 偏移相对对象起点
  int8  budget_bits;   // +0x00  bit0=标志, bits1..7 = 预算（7 位有符号；Emit: strb (score<<1)）
  AugmentRecord rec;   // +0x01  5 字节，tag 在 +0x01
  shared_ptr<SearchNode> parent; // +0x06（指针）/+0x0e（控制块）
  LoudsTrie::Cursor cur;         // +0x16：u32 节点号, u8 标签下标, u64 trie 指针（15 字节）
};
```

预算读法（所有增强器一样）：`ldrsb w, [node]; asr w, w, #1`。初值 61（0x1008f7250）。

Emit 伪代码（0x1009d2140）：
```c
void Emit(Ctx* c, Cursor cur, vector<string> const& pending, int budget, AugmentRecord const& rec) {
  if (!SearchNode::IsValid(c->parent, rec, cur)) return;           // 0x1009d2198
  auto n = make_shared<SearchNode>{budget<<1, rec, c->parent, cur};
  if (!pending.empty()) { c->pending_out.push_back({n, pending, c->flag}); return; }   // 还有没消化的输入片段
  bool terminal = louds.is_leaf(cur) && louds.label_depth_ok(cur);  // [+0x1d8]/[+0xd8] 位图
  if (terminal && rec.tag != 4 /*Exchange*/ && rec.tag != 7 /*Skip*/)
      c->leaves->push_back(n);                                     // 0x1009d22f4..0x1009d2304
  c->frontier->push_back(n);                                       // 0x1009d2598
}
```

IsValid（0x1009b7368）——**组合约束的一半在这里**：
```c
bool IsValid(SearchNode* p, AugmentRecord const& r, Cursor const& cur) {
  if (!p) return true;
  bool at_sep = cur.node && label(cur) == ',';                 // ',' = 音节尾（trie 里的分隔标签）
  if (!at_sep && (p->rec.tag == 3 || p->rec.tag == 25)) return false;   // 补全之后只能停在音节尾
  if (r.tag == 6 /*漏敲*/ && label(p->cur)==',' && p->parent && (p->parent->rec.tag==3||==25)) return false;
  if (r.tag == 7 /*多敲*/ && p->rec.tag == 0) return false;   // 不能在音节第一个键上判多敲
  if (p->parent && p->parent->rec.tag == 10 /*平翘舌*/ && first_char(r) == 'h') return false; // z→zh 之后不能再吃 h
  if (p->rec.tag == 8 /*声母模糊*/)
      return r.tag ∈ {1 ExactMatch, 3 Prism, 25 DPPrism, 27 UpperCase};   // 掩码 0x0a00000a
  return true;
}
```

callImpl 里对每个敲错类增强器的公共前置（0x1009d26f8 起）：
```c
if (!(mask & (1<<idx))) skip;
if (window.size() != NeedWindowSize) skip;                 // 窗口 = 接下来 1..3 个输入片段（string）
if (any(window, s == ",")) skip;                           // 用户敲了分隔符的地方不纠
if (parent_tag ∈ M = 0x271ffa08) skip;                     // M = {3,9,11..20,24,25,26,29}：补全/模糊/双拼/拼写表之后紧接着不做敲错
if (any(window, 含大写字母)) skip;                          // 0x1009d3650：大写 = 用户要原样
```

各增强器的门槛与新预算（[确]，地址是 `asr #1` 后的 `sub/cmp`）：

| 增强器 | 门槛 | 子节点预算 | 地址 |
|---|---|---|---|
| Exchange 换位 | budget ≥ 31 | budget − 31 | 0x1009d271c |
| KeyboardNeighbour 邻键（T26） | budget − 28 ≥ 0 | budget − 28 | 0x1009d2764 |
| 　邻键（输入片段长度 ≥ 2，即 T9 键） | budget − 61 ≥ 0 | budget − 61 | 0x1009d278c |
| 　垂直邻键（分方向模式） | budget − 28 == 33（本音节第一次纠错） | 0（吃光） | 0x1009d3218 `cmp #0x21`；0x1009d5d9c `str wzr` |
| InsertAtMiddle 漏敲 | budget ≥ 61 | budget − 61 | 0x1009d332c |
| Skip 多敲 | budget ≥ 31 | budget − 31 | 0x1009d336c / 0x1009d477c |
| EditPinyinOnce 拼写表 | — | budget − 61 | 0x1009c5848 |
| RepeatPrevChar 重复键 | — | budget − 61 | 0x1009c5f00 |
| KeyboardNeighbourForDoublePin | — | budget − 28 | 0x1009c4f34 |
| Consonant / XnToXng / Retroflex / 其余模糊、YvLike、LveNve、UpperCase | budget ≥ 0 | **不变** | 0x1009d3c38、0x1009d4ad4、0x1009c4d88 |

由 61 推出的组合规则（[推]，纯算术）：一个音节里**最多两处敲错**；邻键+邻键、邻键+换位、邻键+多敲、换位+邻键、多敲+邻键可以；
换位+换位、换位+多敲、多敲+多敲不行；漏敲、拼写表、重复键只能单独出现；垂直邻键只能单独出现；模糊音不占预算、可以和任何一处敲错共存（但不能紧跟在补全/模糊节点后面做敲错，见 M）。

### 1.4 增强器统一接口

```c
// BaseIterativeInputAugmenter<Derived>::operator()(window, parent_node, emit)
//   输入：parent（游标 + record + 预算）、window（接下来 k 个输入片段，k ∈ NeedWindowSize()）
//   输出：对每个可行的「游标走法」调 Emit(新游标, 剩余 pending, 新预算, 新 record)
static vector<size_t> NeedWindowSize();   // 每个增强器的窗口长度集合（静态局部变量缓存）
```
`NeedWindowSizeCalculator<…>` 0x1009b220c 把 28 个集合并起来，SearchFromLeft 按并集切窗口。

五个读透的增强器（伪代码；窗口与预算是 [确]，游标动作从 lambda 体读出）：

```c
// KeyboardNeighbour（tag 5）callImpl<3> 0x1009d2744..0x1009d3298；lambda 0x1009d5a78 / 0x1009d5cac / 0x1009d5ee8
void KN(parent, window /*1 个片段*/) {
  int b = parent.budget - 28;
  if (window[0].size() >= 2) b -= 33;              // T9：一个键多个字母
  if (b < 0) return;
  string s = window[0];
  if (s.size() == 1) {
    if (direction_split /* callImpl 第 7 个参数 x6 非空 */) {
      VisitT26KeyBoardNeighbourParallel(s[0], [&](char c){ try(c, b); });     // 0x100f2ef54，同排左右
      if (b == 33) VisitT26KeyBoardNeighbourVertical(s[0], [&](char c){ try(c, 0); }); // 0x100f2efe0，上下排；预算清零
    } else VisitT26KeyBoardNeighbour(s[0], [&](char c){ try(c, b); });       // 0x100f2f06c，全部邻键
  } else VisitT9KeyBoardNeighbour(s, ...);                                     // 0x100f2f104
  // try(c, b): cur' = parent.cur.child(c)（LOUDS 子节点按标签找）; if (cur') Emit(cur', {}, b, {tag 5, orig=s[0], repl=c, 0, pos});
}

// Skip 多敲（tag 7），窗口 2
void Skip(parent, window /*[x, y]*/) {
  if (parent.budget < 31) return;
  // 丢掉 x，用 y 走 trie（record {ch=x}）；IsValid 禁止在音节第一个键上判多敲
  if (cur' = parent.cur.child(y)) Emit(cur', {}, parent.budget - 31, {tag 7, ch=x});
}

// Exchange 换位（tag 4），窗口 2；lambda 0x1009d5884 用 Cursor::Move(string_view) 走 "yx"
void Exchange(parent, window /*[x, y]*/) {
  if (parent.budget < 31) return;
  if (cur' = parent.cur.move({y, x})) Emit(cur', {}, parent.budget - 31, {tag 4, first=y});
  // Emit 里：换位节点即使到了音节尾也不进叶子（tag 4 例外），要等下一步 ExactMatch 收尾
}

// XnToXngNasal 前后鼻音（tag 9），窗口 1，callImpl<3> 0x1009d3ce4..0x1009d4ad8
void XnToXng(parent, window /*[x]*/) {
  if (parent.budget < 0) return;
  if (x != 'n') return;                               // 0x1009d3eb8 cmp #0x6e
  if (!XngNasalOpt::IsEndWithXn(parent.cur)) return;  // 当前前缀 + n 是 an/en/in 结尾、且 +ng 合法
  cur' = parent.cur.move("ng");
  Emit(cur', {}, parent.budget /*不扣*/, {tag 9});
}

// PrismCompletion 补全（tag 3），窗口 {1,3}，在 SearchFromLeft 内联；辅助函数：
//   IsXhCase(input, prev) 0x1009bab48: input=="h" && prev∈{z,c,s} → 不在这里补全（zh/ch/sh 还没敲完）
//   IsStartWithAOE(input, cur) 0x1009bac7c: 音节首（cur 标签 ',' 或 0）且 input 是 a/o/e 单字母
//   IsInvalidPinyinCompletionFromFirstCh(c, a, b) 0x1009bb06c: 音节首时把 a+b 拼起来查一张合法表，查不到就不补
void Prism(parent, window) {
  if (!IfProcessThisCursor(parent.cur, input)) return;   // 输入到头 / 窗口末尾
  VisitAllCompletionCursor(parent.cur, [&](Cursor c2){    // 沿 trie 走到下一个 ',' 为止的所有路径
     Emit(c2, {}, parent.budget, {tag 3, prefix, len});    // [推] 不扣预算
  });
  // IsValid：补全节点之后只能停在音节尾；补全之后不能紧接敲错（M 掩码）
}
```

### 1.5 邻键表（[确]）

- 硬编码的 QWERTY 位图表，`u64[26]`，按字母下标取一行、bit i = 第 i 个字母是邻键：
  同排表 0x102eb6220（`__TEXT`，`VisitT26KeyBoardNeighbourParallel`）、上下排表 0x102eb62f0（`…Vertical`）、合表 0x103375e90（`__DATA`，`VisitT26KeyBoardNeighbour` 与 `IsT26KeyBoardNeighbour(char,char)` 0x100f2f71c 用）。
  抽查一项可知合表 = 同排 ∪ 上下排（不导出表内容，青简自己按几何算）。
- 不是按布局算的；没有物理键盘 / 触屏两张表，区分的是**同排 vs 上下排**（触屏上下误触更少、代价更高的处理）和 **T26 vs T9**；双拼另有 `KeyboardNeighbourForDoublePin`（tag 28，扣 28）。
- 触屏个性化走另一条路：`MemCacheUserCoordinateDict` + `coordinate_score_abbr="cod_score"` 特征（B2 的范围）。

### 1.6 record → 统计 → 分数

**RecordStatistics**（0x1009ae664，`Syllable::InitStat` 与 `LeafNodeAdder::Add` 调用；32 字节，16 个 u16；访问器跳表 0x102e5f6ea…0x102e5f704）：

| 偏移 | 计数 | 来源 tag |
|---|---|---|
| +0x00 | exact | 1, 27 |
| +0x02 | completion | 3 |
| +0x04 | insertion（漏敲） | 6, 30 |
| +0x06 | deletion（多敲） | 7 |
| +0x08 | replacement（邻键，标志位 0） | 5(b2.bit0=0), 28 |
| +0x0a | exchange | 4 |
| +0x0c / +0x0e | edit_pinyin type2 / type0 | 29 |
| +0x10 | replacement（邻键，标志位 1） | 5(b2.bit0=1) |
| +0x12 | fuzzy | 掩码 0x041fff00 的 tag + 双拼补全里带模糊的 |
| +0x14 | shift_correction | 21 |
| +0x16 / +0x18 / +0x1a / +0x1e | 双拼 / 双拼补全 / 双拼模糊 / 双拼邻键 | 24..28 |
| +0x1c | 消耗的输入长度 | 各 tag 按模式长度加（HuangToWang +5、HuiFei +3…） |

**兜底线性打分** `Syllable::Statistics::Score(params, bool use_completion, int a, int b, int c)` 0x1009f0414：
权重是 capnp `BeamSearchParamsProto` 数据段里的 float（字段存在性按数据段长度判，缺省 0；`|w|<1e-4` 当 0），**不是代码常数**：

```c
float Score(Stat s, Params p, bool use_comp, int a, int b, int c) {
  float x = 0;
  if (c + a > b && s.repl) { x += p.f78 * s.repl; s.repl = 0; }      // 超阈值时邻键单独计价
  int typos = s.del + s.comp + s.xchg + s.ins + s.edit2 + s.edit0;
  if (b > 0 && typos)        x += p.f7c * typos;
  if (s.repl && typos)       x += p.f80 * max(typos, s.repl);
  x += p.f0c * s.repl + p.f10 * s.del + p.f08 * s.ins + p.f90 * s.edit2 + p.f94 * s.edit0;
  x += -2.5 * s.repl_flag1;                                          // 唯一的硬编码权重 0x1009f06e4
  x += p.f18 * s.fuzzy + p.f14 * s.xchg + (use_comp ? p.f04 * s.comp : 0) + p.f0c * s.shift;
  x += (双拼条件) ? p.f1c : 0;
  x += p.f50 * (max(s.repl,1) - 1);                                  // 第二个以后的邻键额外扣
  return x;
}
// PathV2::CalculateScore 0x1009f40dc：path.score = parent.score + Score(...) + w_lm * lm_logprob + …
```
CalculateScore 的调试 JSON 键（[确] 字符串 0x102cce6da…）：`num_replacements / num_deletion / num_insertion / num_fuzzy / num_exchange / num_completion / num_shift_correction / num_english / exactly_match` 及各自 `*_with_w`、`lm_score_with_w`。

**LTR 特征**（MixRank，进 `flat_hash_map<string,float>`）：
- 逐 record 字符串特征 `SyllableRecordsToString` 0x100b0eee4：每条 record 写成 `<缩写>_<字符>_<字符>_<位置>`，`;` 连接；缩写 `c` 补全、`x` 换位、`r` 邻键、`i` 漏敲、`d` 多敲、`s` shift、`f` 模糊（`f_<声母>_<声母>`、`f_z_h` 等），即「哪个键错成哪个键」本身是特征。
- `exact_with_fuzzy`（0x100a4ba38）：候选类型 12、整条路径只有 1 个词且带模糊标志 → 1.0。
- `exact_with_typo`（0x100a4bce0）：路径 1 个词且带敲错 → 1.0；否则 `lm_exact_with_typo` / `multi_exact_with_typo` = 词数 − 1（计数）。
- 其余：`exact_after_typo`、`lm_exact_after_typo`、`multi_exact_after_typo`、`mrt`（multiple_replacement_typo）、`typo_after_mrt`、`swt`（single_with_typo）、`userDict_typo`、`dcs`（DynamicCorrectionScoreParams）、`cod_score`。

### 1.7 日志 / 调试串
`<keyboard_neighbour original="`、`<insert_at_middle insertion="`、`<exchange exchanged_to_first="`、`<fuzzy un_to_ong/>` 等（record 的 XML 形式）；`Reduce typo by version 2/3`、`RemoveTypoIfNeed`。

---

## 2. 证据表

| 结论 | 置信度 | 地址 / 字符串 |
|---|---|---|
| sizeof(AugmentRecord)=5，byte0=tag | 确 | 0x10067dc1c（÷5）、0x100d02ef4、0x100b107e8 `ldrb w20,[x21],#1` |
| tag 顺序 = 参数包顺序 | 确 | VisitHelper 0x1006779d4 跳表：1→ExactMatch ToJSON … 5→KN ToJSON |
| InsertAtMiddle=漏敲、Skip=多敲 | 确 | 缩写 `i`/`d`（0x102e95858/0x102e95870）、窗口 {1}/{2}（0x1009b35e4/0x1009b380c） |
| 预算初值 61 | 确 | 0x1008f7250 `mov w4,#0x3d` |
| 预算存 node byte0 的高 7 位 | 确 | 0x1009d21dc `lsl w13,w23,#1`; `strb`；各处 `asr #1` |
| 换位/邻键/漏敲/多敲 = 31/28/61/31 | 确 | 0x1009d2720、0x1009d2768、0x1009d332c、0x1009d477c |
| T9 邻键额外 −33 | 确 | 0x1009d278c `sub w10,w10,#0x21` |
| 垂直邻键只在预算 33 时、并清零 | 确（语义推） | 0x1009d3218 `cmp w8,#0x21`；0x1009d5d9c `str wzr,[x9]` |
| 拼写表、重复键 −61 | 确 | 0x1009c5848、0x1009c5f00 |
| 模糊音 / 大小写 / ü 不扣预算 | 确 | 0x1009d3c38、0x1009d4ad4（`mov x4,x25` 未改）、0x1009c4d88 |
| 一个音节最多两处敲错 | 推 | 由 61 与各代价算术推出 |
| IsValid 规则 | 确 | 0x1009b7368 全函数 |
| 父 tag ∈ 0x271ffa08 时不做敲错 | 确 | 0x1009d284c、0x1009d3410、0x1009d3d70 |
| 窗口含 "," / ";" / 大写不做 | 确 | 0x1009d27a4、0x1009d3dfc、0x1009d3654 |
| 邻键表硬编码 u64[26]，同排/上下排/合表 | 确 | 0x102eb6220、0x102eb62f0、0x103375e90；0x100f2ef54/0x100f2efe0/0x100f2f06c |
| 模糊类 tag 掩码 0x041fff00 | 确 | 0x100b107fc、0x1009ae6ec |
| RecordStatistics 字段表 | 确 | 0x1009ae664 + 跳表 0x102e5f6ea…704 |
| 兜底打分 = Σ 计数 × proto 权重，−2.5 硬编码 | 确 | 0x1009f0414..0x1009f0748 |
| 字段语义（num_replacements 等） | 推 | CalculateScore 的 JSON 键与计数对得上 |
| 特征串格式 `<abbr>_<c>_<c>_<pos>` | 推 | 0x100b0eee4（`;`=0x3b、`std::to_string`） |
| exact_with_typo 0/1、lm/multi_ 为词数−1 | 确 | 0x100a4bd1c、0x100a4bd94 |

---

## 3. 青简落点

### 3.1 现状：三种来源各在哪展开、代价怎么进 Viterbi

| 来源 | 在哪展开 | 粒度 | 代价 | 进路径的方式 |
|---|---|---|---|---|
| 模糊音 | `/Users/liyuqing/sproot/qingjian/crates/qingjian-core/src/fuzzy/rules.rs` `FuzzyRules::expand`（`alternatives` 规则叠加到不再增长） | 每位置若干写法 | 第 0 种 0，其余一律 `ranking::FUZZY_PENALTY = ln 2`（**不论叠了几条规则**，zhen→zeng 也只扣一次） | `fuzzy/expanded.rs` `Expanded.positions: Vec<(Vec<(String, f64)>, bool)>` |
| 音节级敲错 | `correction/typo/mod.rs` `variants()`（全音节表预算的一处编辑变体，同一变体留最便宜的类）；`engine/query/lookup.rs` `expand_positions` 里 `push_alternative` | 每个**完整**音节最多一处 | `TypoCosts::typo_cost(kind, accepted)` = {换位 5.0, 换键 5.0, 多敲 5.5, 少敲 5.5} − min(ln(1+接受次数), 3.0) | 同上，`push_alternative` 同写法取 min |
| 整段一处编辑 | `correction/mod.rs` `candidates()`（一处编辑 + 可选第二处相邻换键）；`engine/correcting.rs` `find_correction` 逐变体跑 `convert_sentence_static` | 整段输入 | `correction_cost(transpose, accepted)` = 5.0（换位 −1.0 只在纠正间比较）− 个人折扣；第二处 +5.0 无折扣；与原样比时用不打换位折扣的分 | 不进词图：改写输入后整句重跑 |

进 Viterbi：`sentence/viterbi/mod.rs` 的 `convert_paths(…, cost: impl Fn(usize,&str)->f64, …)`；
`span_candidates` 对每个命中按音节求 `Σ cost(start+i, syllable)` 得 `SpanWord.penalty`，格子内排序乘 `exp(−penalty)`；
路径上 `score = best_predecessor + weight_bonus − hit.penalty`，`Node.penalty` 累加，回溯到 `Conversion.penalty`（`altered()` = penalty > 0）。
调用方 `engine/query/converting.rs::paths` 传 `|index, syllable| expanded.cost(index, syllable)`；词级排序 `engine/query/phonetic.rs` 用 `expanded.penalty(hit.syllables())`（只有模糊音，没有敲错）。
上屏学习 `engine/commit/mod.rs::syllable_steps` 用对齐**事后重推**哪个音节是敲错（`typo::is_variant`），因为路径上没留标记。

和 WeType 的差别：青简没有「路径上的标记」，代价一出 `Expanded::cost` 就只剩一个 f64；同写法多来源 min 合并后来源丢失；整段编辑是另一套。

### 3.2 新增类型（一个类型一个文件，收进 `correction/mark/`）

```
crates/qingjian-core/src/correction/mark/mod.rs        //! 词图边上的变换标记：哪几种变换把敲的键读成了这个音节。只 mod + re-export
crates/qingjian-core/src/correction/mark/transform.rs  // enum Transform
crates/qingjian-core/src/correction/mark/transforms.rs // struct Transforms（u16 位集）
crates/qingjian-core/src/correction/mark/edge.rs       // struct EdgeMark
crates/qingjian-core/src/correction/mark/counts.rs     // struct MarkCounts（路径级计数，给特征/日志）
```

```rust
/// 一种把敲的键读成别的音节的变换。顺序即位号，持久化（输入日志）时按名字写，不按位号。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Transform {
    Fuzzy = 0,          // 模糊音（第一步不分规则）
    Transpose = 1,      // 相邻两键敲反（WeType xchg）
    Substitute = 2,     // 邻键（WeType kb_neighbour，同排）
    SubstituteVertical = 3, // 上下排邻键（第一步权重 = Substitute，行为不变；B2 / iOS 再分）
    Extra = 4,          // 多敲（WeType skip / d）
    Missing = 5,        // 漏敲（WeType insert_at_middle / i）
}

/// 一条边用了哪些变换。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Transforms(u16);
impl Transforms {
    pub const EXACT: Self = Self(0);
    pub fn with(self, t: Transform) -> Self;
    pub fn contains(self, t: Transform) -> bool;
    pub fn is_exact(self) -> bool;
    pub fn iter(self) -> impl Iterator<Item = Transform>;
    pub fn has_typo(self) -> bool;   // 除 Fuzzy 以外任一位
}

/// 一个位置上一种写法的标记：用了哪些变换，加上个人敲错表给的折扣（只对敲错位生效）。
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct EdgeMark {
    pub transforms: Transforms,

    /// 个人敲错表的折扣 min(ln(1+接受次数), discount_cap)，没有敲错位时为 0。
    pub discount: f64,
}

/// 一条路径（或一个词）上各变换出现了几次，给排序特征与日志用；不参与第一步的打分。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MarkCounts([u8; 6]);
impl MarkCounts { pub fn add(&mut self, t: Transforms); pub fn merge(&mut self, other: Self); pub fn get(&self, t: Transform) -> u8; pub fn typos(&self) -> u8; }
```

权重表不另起类型，放在已有的 `TypoCosts`（`correction/costs.rs`）上，保证 `--tune` 现有键全部照旧：

```rust
pub struct TypoCosts {
    // 现有 7 个字段原样保留
    /// 模糊音一个位置的代价（原 `ranking::FUZZY_PENALTY`），按位置算一次，不按规则条数累加。
    pub fuzzy: f64,                       // DEFAULT = LN_2
}
impl TypoCosts {
    /// 代价 = 基础 0 + Σ 各位权重 − 折扣（折扣只在有敲错位时减）。
    pub fn mark_cost(&self, mark: &EdgeMark) -> f64;
    pub fn weight(&self, t: Transform) -> f64; // Fuzzy→fuzzy, Transpose→transpose, Substitute|SubstituteVertical→substitute, Extra→extra, Missing→missing
    pub fn typo_mark(&self, kind: TypoKind, accepted: u32) -> EdgeMark; // 现有 typo_cost 改成 mark_cost(&typo_mark(..))
}
```

权重初值映射（保证第一步 +0.0）：

| Transform | 权重初值 | 来自 |
|---|---|---|
| Fuzzy | ln 2 ≈ 0.6931 | `ranking::FUZZY_PENALTY`（改成引用 `TypoCosts::DEFAULT.fuzzy`，或反过来，二者只留一个源头） |
| Transpose | 5.0 | `TypoCosts::transpose` |
| Substitute / SubstituteVertical | 5.0 / 5.0 | `TypoCosts::substitute` |
| Extra | 5.5 | `TypoCosts::extra` |
| Missing | 5.5 | `TypoCosts::missing` |
| 折扣 | min(ln(1+n), 3.0) | `TypoCosts::discounted` / `discount_cap` |

行为不变的三条硬约束（第一步必须守住，否则回放会动）：
1. **同一位置同一写法来自多种来源时保留代价最低的那条标记，不做位或**（现在 `push_alternative` 是 min；位或会把 ln2+5.0 叠上去）。
2. 模糊音不论叠几条规则只算一个 `Fuzzy` 位（位集天然去重，与现在「index>0 一律 ln2」一致）。
3. 敲错变体只从敲的原样派生，不在模糊写法上再派生（现状如此；与 WeType「模糊/补全节点后不接敲错」的 M 掩码也吻合）。

### 3.3 改哪些函数

| 文件 | 改动 |
|---|---|
| `crates/qingjian-core/src/fuzzy/expanded.rs` → 改成目录 `fuzzy/expanded/{mod.rs, form.rs}` | 新类型 `Form { text: String, mark: EdgeMark, cost: f64 }`（cost 在加入时用 `TypoCosts` 算好，Expanded 不持有权重）；`positions: Vec<(Vec<Form>, bool)>`；`push_alternative(index, text, mark: EdgeMark, cost: f64)` 仍按 cost 取 min；`cost(index, syllable) -> f64` 签名不变；新增 `mark(index, syllable) -> EdgeMark`、`edge(index, syllable) -> (f64, Transforms)` |
| `crates/qingjian-core/src/fuzzy/rules.rs` `expand` | 多收一个 `fuzzy_cost: f64`（或 `&TypoCosts`），第 0 种 `EdgeMark::default()`，其余 `Transforms::EXACT.with(Fuzzy)` + cost=fuzzy_cost |
| `crates/qingjian-core/src/engine/query/lookup.rs` `expand_positions` | `push_alternative(index, text, self.typo_costs.typo_mark(*kind, accepted), self.typo_costs.typo_cost(*kind, accepted))` |
| `crates/qingjian-core/src/engine/query/phonetic.rs` | `expand(&patterns)` 传 fuzzy 代价；可选给 `Scored` 加 `marks: MarkCounts`（第二步） |
| `crates/qingjian-core/src/sentence/viterbi/mod.rs` | `cost: impl Fn(usize,&str)->f64` 改成 `edge: impl Fn(usize,&str)->(f64, Transforms)`（`convert`/`convert_whole`/`convert_with`/`convert_paths`/`span_candidates`）；`Node` 加 `marks: MarkCounts`（Copy，6 字节）随 `penalty` 一起累加；`backtrack` 写进 `Conversion` |
| `crates/qingjian-core/src/sentence/span/word.rs` | `SpanWord` 加 `marks: MarkCounts`（`penalty` 保留） |
| `crates/qingjian-core/src/sentence/conversion.rs` | `Conversion` 加 `marks: MarkCounts`；`altered()` 第一步仍按 `penalty > 0`（与 `!marks.is_empty()` 等价，因为最低敲错代价 5.0−3.0=2.0>0、模糊 ln2>0，但先别换判据） |
| 调用点 | `engine/query/converting.rs:280`、`engine/query/english_tail.rs`、`sentence/viterbi/tests.rs`（3 处闭包 `|_,_| 0.0` → `|_,_| (0.0, Transforms::EXACT)`） |
| `crates/qingjian-core/src/correction/result.rs` | `Correction::marks() -> Transforms`：`Edit::Transpose→Transpose`、`Substitute→Substitute`、`Insert→Missing`、`Delete→Extra`，`second` 再加 `Substitute`（只给日志/特征，打分仍走 `correction_cost`） |
| `crates/qingjian-core/src/engine/commit/mod.rs` | 第二步：`accepted_typos` 优先读候选路径上的标记，对齐只做兜底（现在的 `syllable_steps` 重推保留） |
| `apps/cli/src/tuning.rs` | `KEYS` 加 `"fuzzy"`；第二步加 `"substitute-vertical"` |
| `docs/notes/crate-notes.md` | 敲错 / 模糊音代价段落改成「标记 + 权重表」，列权重初值 |

**整段一处编辑**（`correction/` 主路）第一步不并进词图：它比的是「改写后整句 vs 原样整句」，并进词图要让敲错边跨音节边界（`dinbushabg` 的漏敲在音节中间），
相当于把切分也纳入搜索，这是 WeType 的做法（游标级 BFS），但青简的切分在 parser、查词在 dictionary，改动远超 B1。第一步只让它产出同一套 `Transforms`，代价函数不动。

### 3.4 iOS 邻键单独一张表

- 现在的 `correction::typo::adjacent` 是固定的半键错位（`lower == upper || lower+1 == upper`），Mac 物理键盘与 iOS 26 键共用。
- iOS 的字母行在 `/Users/liyuqing/sproot/qingjian/cloud/ios/Keyboard/Sources/KeyLayer.swift`（`qwertyuiop` / `asdfghjkl` / shift + `zxcvbnm` + 删除）、几何在 `KeyboardLayout.swift`（`place(_:unit:width:)` 给每个键 x/width）。第三排两侧有 shift/删除，z 不在 a 正下方，错位与物理键盘不同。
- Core 保持平台无关：新增 `correction/typo/keyboard.rs` 的 `enum KeyboardGeometry { Physical, Touch26 { centers: [(f32, f32); 26] } }`，
  壳只把字母键中心（键宽为单位）交给 Core，Core 按距离阈值算 `Adjacency::{Horizontal, Vertical}`；`Engine::set_keyboard_geometry`（`engine/setup.rs`），
  素笺的 bridge 在 `cloud/crates/qingjian-cloud-bridge/src/session/config.rs` 里和 `set_fuzzy` 一起调。缺省 `Physical` = 现在的表，所以 Mac / Windows / Linux 行为不变。
- 变体表 `TABLE` 现在是 `LazyLock` 全局一份；有两种几何后改成按几何各缓存一份（`HashMap<GeometryId, Table>` 或 Engine 持有），
  `Substitute` 拆出 `SubstituteVertical`（WeType 上下排更贵、只能单独出现）；第一步两者权重相同。
- 冲突：变体表从全局静态变成 Engine 状态，`typo::variants` / `typo::kind` / `is_variant` 的签名要多一个几何参数，调用点 `engine/query/lookup.rs`、`engine/commit/mod.rs`、`correction/mod.rs`（整段第二处换键）都要传。

### 3.5 与青简约束的冲突与改法

- 输入优先：位集 u16 / 计数 6 字节都是 Copy，`Node` 与 `SpanWord` 只多几个字节；`SpanCache` 的键是写法串，不含代价也不含标记，缓存规则不变（「代价一变就清」的约定照旧）。长句（≥10 音节）不得变慢：用 `qingjian-cli --replay` 报的按键同步耗时分位数（`apps/cli/src/latency.rs`，现报 p50 / p99，B1 验收要 p95 就在那里加一档）对比。
- 不跨用户学习：权重只来自本地常数与 `--tune`；不引入 LTR（任务原文明确不做），WeType 的 `RecordStatistics × proto 权重` 对应青简的 `MarkCounts × TypoCosts`。
- 一个类型一个文件、子模块用目录：`fuzzy/expanded.rs` 变 `fuzzy/expanded/` 目录；`correction/mark/` 新目录。
- 不改用户可感知行为（第一步），所以不动 `docs/user/`；第三步若改了纠错结果，按 contributing 在同一提交里改用户文档里关于纠错的说明（先 grep `docs/user/` 有没有这一页）。

---

## 4. 复刻步骤

1. **加类型**：`correction/mark/{mod,transform,transforms,edge,counts}.rs`；`TypoCosts` 加 `fuzzy` 字段（DEFAULT = `LN_2`）与 `weight` / `mark_cost` / `typo_mark`；`ranking::FUZZY_PENALTY` 改成指向同一个常数。
2. **Expanded 带标记**：`fuzzy/expanded/` 目录化，`Form { text, mark, cost }`；`FuzzyRules::expand` 打 Fuzzy 位；`expand_positions` 打敲错位。断言：同写法多来源取最小代价的那条标记。
3. **Viterbi 透传**：`convert*` 的闭包改返回 `(f64, Transforms)`；`SpanWord` / `Node` / `Conversion` 加 `marks`；三个调用点与测试闭包跟着改。
4. **整段编辑出标记**：`Correction::marks()`，只进日志。
5. **验收第一步（行为不变）**：`cargo test -p qingjian-core`；`qingjian-cli --replay <clean2>`、`--eval-text data/eval/sentences.tsv` 与 `tools/release/gate-pass.sh`，指标必须与改前逐位一致（+0.0）；`--tune transpose=5,substitute=5,extra=5.5,missing=5.5,fuzzy=0.6931` 与不带 `--tune` 结果相同。
6. **日志与特征（仍不改排序）**：输入日志记录上屏候选的 `MarkCounts`（按名字写，例如 `{"typo":{"substitute":1}}`），`--replay` 按标记分桶报首选率：哪些变换救回了多少、误纠了多少（docs/plan/model-eval.md 的写法：召回 = 带敲错标记且被选中 / 真敲错条数；误纠率 = 首选带敲错标记但用户选了原样候选的比例）。
7. **第二步调权重（单独提交）**：
   - 把 WeType 的组合约束作为可选规则试：同一音节允许两处敲错（邻键+邻键、邻键+换位、邻键+多敲；换位+多敲、换位+换位禁止；漏敲独占）；上下排邻键只能单独出现。WeType 的数值（预算 61，邻键 28、换位 31、多敲 31、漏敲 61）只是相对比例，**青简要按自己的回放重扫**。
   - 「第二个邻键额外扣」（WeType `f50 × (repl−1)`）对应青简给 `MarkCounts` 加一项超线性惩罚，`--tune second-typo=`。
   - iOS：`set_keyboard_geometry(Touch26 {..})`，Substitute / SubstituteVertical 分开扫。
8. 同步 `docs/notes/crate-notes.md`（数据结构与常数），`docs/design/` 记一段「为什么用标记 + 权重而不是固定代价」，指回本文件里的 WeType 证据。

单测清单（输入 → 期望）：

| 测试 | 输入 | 期望 |
|---|---|---|
| transforms_bits | `EXACT.with(Fuzzy).with(Substitute)` | contains 两者、`has_typo()`、`iter()` 顺序 Fuzzy, Substitute |
| mark_cost_matches_typo_cost | 每个 `TypoKind` × accepted ∈ {0,1,2,1000} | `mark_cost(typo_mark(k,n)) == typo_cost(k,n)`（逐位相等） |
| fuzzy_cost_once | 规则 z/zh + eng/en，`zhen` 展开出 `zeng` | 该写法 mark = {Fuzzy}，cost = ln2（不是 2·ln2） |
| min_merge_keeps_cheapest | 位置 1 先有 (`gang`, Fuzzy, 0.693)，再 push (`gang`, Missing, 5.5) | 留 Fuzzy 0.693；`penalty(["mei","gang","xi"]) == 0.7` 的老断言照过 |
| expanded_mark_lookup | `mei gan x` + push `guan`(Missing) | `mark(1,"guan").transforms == {Missing}`、`mark(1,"gan")` 为 EXACT |
| viterbi_marks_flow | `meiganxi` 开敲错（现有测试在 `crates/qingjian-core/src/engine/tests/correction.rs`） | 首选 `Conversion.marks.get(Missing)==1`（gan→guan），`penalty` 与改前相等 |
| viterbi_exact_has_no_marks | `nihao` | `marks` 全 0，`altered()==false` |
| correction_marks | `nihoama` → `nihaoma`（Transpose）、`dinbushabg` → `dianbushang` | `{Transpose}`；`{Missing, Substitute}` |
| geometry_physical_unchanged | 26×26 字母对 | `Physical` 下的邻接与现在 `adjacent()` 全等 |
| geometry_touch26 | iOS 键中心（第三排右移 1.5 键） | `z` 与 `s`/`x` 相邻、与 `a` 只算上下排（具体以壳给的几何为准，测试里写死一组样例坐标） |
| replay_identity | CLI 回放 clean2 | 首选 / 前五 / 平均名次与改前一致（CI 外手动跑） |

---

## 5. 未知

- `AugmentRecord` 里邻键 b2.bit0（统计到 +0x10、兜底分固定 −2.5 的那一类）由谁置位：T26 的三个 lambda 都写 0（0x1009d5de4 `sturb wzr`），可能来自 T9 / 触点坐标路径或后处理。下一步看 `VisitT9KeyBoardNeighbour` 0x100f2f104 的 lambda（vtable 0x1031b6fb8）与 `SyllableEdge::SetUserInputSepFlag` 0x1009a6fa4。
- 分方向邻键（同排 / 上下排分开）开关：callImpl 第 7 个参数 x6（`[sp+0x30]`）非空时启用，来源没追。下一步从 `SearchFromLeft` 调 `callImpl<3>` 的实参往上追。
- `BeamSearchParamsProto` 各 float 的字段名与默认值：capnp 编译后没有名字，值在超参数据文件里（不导出）。只知道偏移与乘的是哪个计数。
- 预算是「每个音节」还是「每条 SyllableEdge」：trie 里 `,` 是音节尾标签，`SearchFromLeft` 产出 `vector<Syllable>`，[推] 是每音节；没确认一条 edge 会不会跨多个音节。
- Prism 补全是否扣预算：在 `SearchFromLeft` 内联，没逐条确认。
- `graphs_v3` 另有一套 `SkipTypoAugmenter / ExchangeTypoAugmenter / KeyboardNeighbourAugmenter / InsertAtMiddleTypoAugmenter` 与 `TypoRecord`（开关 `mmkb_cli_engine_graph_v3_disable` / `_force_with_fuzzy`），Mac 默认走哪代图没确认；本文全部是 v2。
- `Statistics::Score` 的三个 int 参数（来自 `[x10+0x200] / [+0x208] / [+0x1fc]`）语义未定，只知道控制「邻键单独计价」与「总敲错数项」是否生效。
