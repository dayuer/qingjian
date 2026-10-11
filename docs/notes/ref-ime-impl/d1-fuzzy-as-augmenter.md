# D1 模糊音做成按键级变换：RefIME 五个模糊增强器 + 青简 #319 / #306 的通用修法

一句话结论：RefIME 的模糊音不是「先切合法音节再替换」，而是 LOUDS 音节 trie 游标上的**按键级变换**：
看到 `n` 就试「走 n 再白送一个 g」（XnToXng），看到 `g` 就试「吞掉它不走」（XngToXn），在音节首把 `z/c/s` 后面插或删一个 `h`（Retroflex），
在音节首把一个声母键换成同组的另一个键再喂回去（Consonant），音节首整串 `fei↔hui`（HuiFei）。**只要变换后的路径在 trie 里存在就行，敲的原拼写合不合法无所谓**，
所以 `tin` → `ting`、`suang` → `shuang` 天然走得通；这些变换都不扣纠错预算。
青简的毛病在切分：`parser` 只认合法音节，`tin` 切成 `ti n…`，模糊音根本没机会作用，非末尾的残缺音节又把整段纠错放了进来，
而整段纠错的「换一个字母」不限邻键，于是 `t→q` 出了「请我说」。

青简修法（已在 /tmp 原型里跑通，见 §4.3）：**把「只有开着的模糊音才读得通的拼写」做成第二张小 trie，切分时也当完整音节（带 `fuzzy` 标记、排在同等合法切分后面）**；
有这种音节时整段纠错照样试，但必须比模糊音读法的整句分高出一次编辑代价才接管。双拼解码用同一张表，PR #330 的双拼特例变成这个机制的一个用例（#330 的 3 个测试在原型上原样通过）。
回放 clean2 与整句评测在「模糊音全关 / in-ing+s-sh / 全开」三种配置下与改前**逐条一致**（replay-details 0 条差异）。

---

## 1. RefIME 怎么做的

### 1.1 位置与公共前置

五个增强器都在 `IterativelyInputAugmenters<…28 个…>::callImpl<3>`（0x1009d26a0，覆盖参数包下标 3..10）与 `callImpl<11>`（0x1009beee4，下标 11..20）里内联，
开关位在增强器元组对象 +0x38 的 u32 位图（B1 已述）。下标 = AugmentRecord tag = 开关位号：

| tag / 位 | 增强器 | 位测试地址 | NeedWindowSize（[确]） |
|---|---|---|---|
| 8 | ConsonantInputAugmenterIteratively | 0x1009d3384 `tbz w8,#7`（字节 +0x38） | {1}（0x1009b3954 `mov w8,#1`） |
| 9 | XnToXngNasalAugmenter | 0x1009d3ce4 `tbz w8,#0`（字节 +0x39） | {1}（0x1009b3b7c） |
| 10 | RetroflexInputAugmenter | 0x1009d3edc `tbz w10,#9` | 2..6（0x1009b3c9c 走 `to_container` 的区间；callImpl 里 0x1009d3f00–0x1009d3f0c 校验 `2 ≤ n ≤ 6`） |
| 11 | HuiFeiAugment | 0x1009d4ce4 `tbz w10,#0xa` | {3}（0x1009b3ec0 `mov w8,#3`；callImpl 0x1009d4cf8 `cmp x9,#0x48` = 3 段） |
| 12 | XngToXnNasalAugmenter | 0x1009bef34 `tbz w8,#3`（字节 +0x39） | {1}（0x1009b4008） |

每个增强器进门前做同一组检查（Consonant 0x1009d3388–0x1009d3880，其余同构）：

```c
// window = 接下来 k 个输入片段（T26 下每段 1 个字母；T9 一段多个字母）
if (!(mask & (1 << tag))) skip;
if (window.size() != NeedWindowSize) skip;
if (parent.budget < 0) skip;                        // ldrsb [node]; tbnz #31 —— 只要求非负，不扣
if (any(window, s == ",")) skip;                    // 用户敲的分隔
if (NextInputMustBeSep(parent.rec)) skip;           // 下面解释，掩码 0x271ffa08
if (any(window, s == ";")) skip;
if (any(window, 含 'A'..'Z')) skip;                  // 大写 = 用户要原样
```

**对 B1 的一处修正**：B1 把 `0x271ffa08` 解释成「补全 / 模糊之后紧接着不做敲错」。符号说明它其实是
`BaseIterativeInputAugmenter::NextInputMustBeSep(record)`（tag > 30 时走 visitor 0x1009bc8e8，名字 [确]），
集合 = {3 补全, 9 XnToXng, 11 HuiFei, 12 XngToXn, 13..20 其余韵母类模糊, 24/25/26 双拼, 29 拼写表}：**这些 record 已经把音节的韵母定死了，下一步只能是音节分隔**（InsertSep 生成的虚拟 `,` 或用户敲的 `'`），
所以任何「在音节内部再走一步」的增强器都不许接在它们后面。声母类模糊（8 Consonant、10 Retroflex）不在集合里，因为它们发生在音节首、后面还要继续吃韵母。

另外三条只对模糊增强器的约束（[确]）：

| 约束 | 适用 | 地址 |
|---|---|---|
| 游标必须在音节首：当前节点标签是 `,` 或根（`cmp w8,#0x2c; ccmp w8,#0,#4,ne; b.ne skip`） | Consonant、Retroflex、HuiFei | 0x1009d38b4、0x1009d413c、0x1009d4ec4 |
| 父 record（父是 InsertSep tag 2 时看祖父）不能是 `IsFuzzyInitials` = {8, 10}：声母模糊不叠（`n→l→r` 两步链、`z→zh` 后再 Consonant 都不行） | Consonant、Retroflex | 0x1009d38c0–0x1009d3974（visitor 0x1009bca78 名为 IsFuzzyInitials、0x1009bcc08 为 IsInsertSeparator，[确]）；0x1009d4148–0x1009d41fc |
| 父 tag ∈ `0x0fdfff0f` = {0,1,2,3,8..20,22..27}，即**不能紧跟在敲错 record（4 换位、5 邻键、6 漏敲、7 多敲）或 21 shift 纠正后面**；tag 28..30 也不行 | XnToXng、XngToXn、Retroflex | 0x1009d4000、0x1009bf130、0x1009d4214；28..30 判断 0x1009d4a14、0x1009d49a0 |

再加 B1 已有的 `SearchNode::IsValid`（0x1009b7368）：Consonant 节点的下一条 record 只能是 {1 精确, 3 补全, 25 双拼补全, 27 大写}（掩码 0x0a00000a），即换出来的那个键必须被原样吃掉；
Retroflex 节点的祖父是 tag 10 时下一步不许再吃 `h`。

### 1.2 ConsonantInputAugmenterIteratively（tag 8，声母互换）

构造 `ConsonantInputAugmenterIteratively(bool, bool, bool, bool)` 0x10074326c：四个开关各往 `vector<string> groups`（对象 +0x8）推一组 [确]：

| 参数 | 组 | 字符串地址 |
|---|---|---|
| 第 1 个 | `"nl"` | 0x102c93d65 |
| 第 2 个 | `"rl"` | 0x102c93d68 |
| 第 3 个 | `"hf"` | 0x102c93d6b |
| 第 4 个 | `"gk"` | 0x102c93d6e |

callImpl 主体 0x1009d3978–0x1009d3c88：

```c
void Consonant(parent, window /* 1 段 */) {
  // 公共前置 + 音节首 + 父/祖父不是 {8,10}
  int b = parent.budget;                                  // 0x1009d39ac asr w9,w26,#1 —— 原样传下去
  flat_hash_set<char> seen;                               // 0x1009d3a7c.. absl 探测 + 0x1009d3ae4 插入
  for (char c : window[0])                                // T26 就一个字母
    for (string g : groups) {
      int i = g.find(c); if (i < 0 || g.size() < 2) continue;
      for (int k = 1; k < g.size(); ++k) {                // 环形取组里其余字母，0x1009d3af0 csinc
        char alt = g[(i + k) % g.size()];
        if (!seen.insert(alt)) continue;
        // 游标不动，把 alt 当成「待消化输入」挂在新节点上（pending = {string(1, alt)}）
        Emit(parent.cur, /*pending*/ {string(1, alt)}, b,
             AugmentRecord{tag 8, b0 = c /*敲的*/, b1 = alt /*换成的*/, b3 = pos});   // 0x1009d3bd4..0x1009d3c3c
      }
    }
}
```

要点：变换发生在**按键层**（把敲的键换掉再喂给 trie），不管敲的拼写是否合法：`fui` → 换成 `h` 后走 `hui`。青简的 n-l / f-h / l-r 对应前三组；`g-k` 青简没有。

### 1.3 XnToXngNasalAugmenter（tag 9，前鼻音补成后鼻音）

callImpl<3> 0x1009d3ce4–0x1009d4ae0：

```c
void XnToXng(parent, window /* 1 段 */) {
  // 公共前置；不要求音节首
  if (!contains(window[0], 'n')) return;                  // 0x1009d3eb8 cmp #0x6e
  if (parent.tag ∉ 0x0fdfff0f) return;                    // 不紧跟敲错
  Cursor cn = parent.cur.child('n');                      // LOUDS 子节点按标签找，0x1009d4568 cmp #0x6e
  if (!cn) return;                                        // 只要求「前缀 + n」是 trie 里的节点（是某个音节的前缀即可，不要求它本身是音节）
  if (!(IsEndWithXn_an(cn) || IsEndWithXn_en(cn))) return;   // lambda 0x1009bde98 / lambda0 0x1009be204
  Cursor cg = cn.child('g');                              // 0x1009d4964 cmp #0x67
  if (!cg) return;
  Emit(cg, {}, parent.budget /*不扣*/, AugmentRecord{tag 9, b3 = pos});   // 0x1009d4abc mov w8,#9; 0x1009d4ad4 mov x4,x25
}
```

`XngNasalOpt::IsEndWithXn`（游标从当前节点倒着走全路径比后缀；对象 +0x20 是 6 个 bool 的 `XngNasalOpt`）：

```c
bool IsEndWithXn(Cursor c, XngNasalOpt o) =
     (o[0] && ends(c,"an") && !ends(c,"ian") && !ends(c,"uan"))   // 0x102c98387 "an"
  || (o[1] && ends(c,"ian"))                                       // 0x102c9957b
  || (o[2] && ends(c,"uan"))                                       // 0x102c9958c
  || (o[3] && ends(c,"en"))                                        // 0x102c5daec
  || (o[4] && ends(c,"in"))                                        // 0x102c9956a
  || (o[5] && ends(c,"on"));                                       // 0x102cc9125  on→ong：等于「ong 少敲 g」
```

（返回式的拼法见 0x1009be1c0–0x1009be200；`an` 与 `ian/uan` 互斥的那一段是 [推]，从 `csinc/orr` 的组合读出。）

**这就是 `tin` → `ting` 的路**：父游标在 `ti`，敲 `n`：`ti→tin` 节点存在（`ting` 的前缀），路径以 `in` 结尾，`tin→ting` 存在 → 直接在 `ting` 上生节点，`g` 是白送的。
同一步 ExactMatch 也会生 `tin` 节点（不是音节尾，后面接 `w` 走不通就死掉）。

### 1.4 XngToXnNasalAugmenter（tag 12，后鼻音读成前鼻音）

callImpl<11> 0x1009bef30–0x1009bf1a0：

```c
void XngToXn(parent, window /* 1 段 */) {
  // 公共前置
  if (!contains(window[0], 'g')) return;                  // 0x1009bf104 cmp #0x67
  if (parent.tag ∉ 0x0fdfff0f) return;                    // 0x1009bf130
  if (!IsEndWithXn(parent.cur, opt @ obj+0x27)) return;   // 父游标已经在 …an/en/in/on 上
  Emit(parent.cur /*不动*/, {}, parent.budget, AugmentRecord{tag 12, b3 = pos});  // 吞掉敲的 g，0x1009bf184 mov w8,#0xc
}
```

`bing` 敲完会在 `bin` 上多一个节点；`ting` 也会在 `tin` 上生节点，但 `tin` 不是音节尾（Emit 只把 trie 叶子进 leaves），自然淘汰。两个方向用两份独立的 XngNasalOpt（+0x20 / +0x27），可以只开一边。

### 1.5 RetroflexInputAugmenter（tag 10，平翘舌，双向）

callImpl<3> 0x1009d3edc–0x1009d4c84，z→zh 方向的游标走法在 `RetroflexInputAugmenter::callImpl` 的 lambda 0x1009d6504：

```c
void Retroflex(parent, window /* 2..6 段 */) {
  // 公共前置 + 音节首 + 父/祖父不是 {8,10} + 父 tag ∈ 0x0fdfff0f
  uint8 has = 0;                                          // 0x1009d4248..0x1009d426c
  for (char c : window[0]) has |= c=='z' ? 1 : c=='c' ? 2 : c=='s' ? 4 : 0;
  if (!has) return;
  if (contains(window[1], 'h')) has |= 0x80;              // 0x1009d45e8..0x1009d4604：敲的是 zh/ch/sh
  string alt;                                             // 开关在对象 +0x26：bit2=z/zh, bit0=c/ch, bit1=s/sh（0x1009d4610..0x1009d4660）
  if ((has&1) && opt.z) alt += 'z'; if ((has&2) && opt.c) alt += 'c'; if ((has&4) && opt.s) alt += 's';
  if (alt.empty()) return;
  if (has & 0x80) {           // zh → z：丢掉敲的 h
    pending = {alt, window[2..]};                         // 0x1009d4824..0x1009d4aec
    Emit(parent.cur, pending, parent.budget, AugmentRecord{tag 10, b0 = alt[0], b1 = 1 /*方向*/});   // 0x1009d4b28 orr #0x100；0x1009d4b48 mov w8,#0xa
  } else {                    // z → zh：在声母后插一个 h
    pending = {alt, "h", window[1..]};                    // lambda 0x1009d6570 strh 'h'
    // lambda 按 pending 的笛卡尔积沿 trie 走，生 tag 10、方向 0 的节点
  }
}
```

[确] 的部分：窗口 2..6、音节首、z/c/s 判定、`h` 判定、三个开关位、tag 10、`b1=1` 表示 zh→z、预算不扣（0x1009d4b3c `ldr w4,[x8]`，x8 指向 0x1009d3f18 存的 `budget>>1`）。
pending 的具体拼法是 [推]（看到 `vector<string>(window+1, end)` 与 `basic_string::assign` 的调用顺序，没逐条跟到 lambda 内部的游标循环）。
`suang` 就是这条路：音节首、window[0]=`s`、window[1]=`u` 没有 `h` → 插 `h` → 游标走 `shuang`，原拼写 `suang` 合不合法无关。

### 1.6 HuiFeiAugment（tag 11，fei ↔ hui 整串）

callImpl<3> 0x1009d4ce4–0x1009d51e8：

```c
void HuiFei(parent, window /* 正好 3 段 */) {
  // 公共前置 + 音节首
  bool to_hui;
  if (has(window[0],'f') && has(window[1],'e') && has(window[2],'i')) to_hui = true;        // 0x1009d4f00 'f' / 'e' / 'i'
  else if (has(window[0],'h') && has(window[1],'u') && has(window[2],'i')) to_hui = false;  // 0x1009d4fb0 'h' / 'u' / 'i'
  else return;
  const char* target = to_hui ? "hui" : "fei";            // 0x102e799a8 "hui"、0x102e79990 "fei"
  Cursor c = parent.cur.move(target); if (!c) return;     // 0x1009d50c0.. 逐字走 LOUDS
  Emit(c, {}, parent.budget, AugmentRecord{tag 11, b0 = to_hui});   // 0x1009d51b8..0x1009d51d4
}
```

只在 `ui/ei` 前做 f/h 互换（方言里最常见的那一对），比通用的 f-h 窄得多。青简的 f-h 是通用声母互换，HuiFei 没有对应开关。

### 1.7 开关从哪来

`CreateAugmenterByFlagWithFuzzyOpt(_wxime_input_augment_option const&, unsigned)` 0x1007403a0 把选项结构的字节拼成增强器位图 [确]：

| 选项字节 | 置位 | 增强器 |
|---|---|---|
| +0x5..+0x8 任一 | 0x80 | Consonant（四组各一个 bool，传给构造函数） |
| +0x9..+0xb、+0x10..+0x12 任一 | 0x900 | XnToXng + XngToXn（六个 bool = XngNasalOpt；哪个字节对应 an/ian/uan/en/in/on 是 [推]） |
| +0xc..+0xe 任一 | 0x200 | Retroflex（z / c / s 三个 bool） |
| +0xf | 0x400 | HuiFei |
| +0x13 / +0x14 / +0x15 / +0x16 / +0x17 | 0x1000 / 0xc000 / 0x30000 / 0x40000 / 0x80000 | HuangToWang / UnToOng+OngToUn / UnToIong+IongToUn / AnAi / EngOng |

即 RefIME 的「模糊音设置」比青简细：前后鼻音按 an / ian / uan / en / in / on 分开，平翘舌按 z / c / s 分开，声母互换四组，另有 hui/fei、huang/wang、un/ong、un/iong、an/ai、eng/ong。

### 1.8 同一条输入既能模糊又能纠错：谁赢

1. **搜索层没有「有模糊就不纠错」的硬规则**：模糊与敲错都是同一次 BFS 里的 record。约束只有结构性的三条：模糊 record 不扣预算；韵母类模糊之后音节必须结束（NextInputMustBeSep）；
   XnToXng / XngToXn / Retroflex 不能紧跟在敲错 record 后面（0x0fdfff0f）。敲错之后隔一个精确匹配再模糊、模糊的音节之后下一个音节里敲错，都允许。
2. **排序层**（B1 §1.6 的 `Syllable::Statistics::Score` 0x1009f0414）：模糊只进一个计数 `fuzzy`（RecordStatistics +0x12，tag 掩码 0x041fff00），乘 `BeamSearchParamsProto` 的 f18；
   敲错按类别进 repl / del / ins / xchg / edit 计数，乘 f0c / f10 / f08 / f14 / f90 / f94，另有 `f7c × 敲错总数`、`f80 × max(敲错, 邻键)`、`f50 × (邻键−1)` 与硬编码的 −2.5。
   权重在超参数据里，二进制里没有数值（B1 未知项，本轮没在 /tmp 材料里找到这份数据，见 §5），**不能断言 f18 的大小，只能确定模糊与敲错是两套独立计数、模糊不吃预算**。
3. **`tinwosuo` 在 RefIME 里的样子**（[推]，由上面的规则推）：`ting`（XnToXng，0 预算）+ `wo` + `shuo`（Retroflex，0 预算）= 2 个模糊 record、0 敲错；
   同样读成 `ting` 的敲错路（敲 `n` 后 InsertAtMiddle 补 `g`）要花 61 整个预算，`qing` 根本出不来：`t` 的邻键是 r / y / f / g / 5 / 6 一带，KeyboardNeighbour 只换邻键，`t→q` 不在表里。
4. `Searcher::reduceTypoCandidateV3`（0x100a6adc8）只在候选流上按特征 id 0x10 / 0x11（敲错标志）与 0x27（分数）丢掉比第一个敲错候选差太多的敲错候选（两个阈值在 `[x19+0x40]`），
   和模糊无关；特征 id 到名字的映射没对上（§5）。

---

## 2. 证据表

| 结论 | 置信度 | 地址 / 字符串 |
|---|---|---|
| tag 8/9/10/11/12 = Consonant / XnToXng / Retroflex / HuiFei / XngToXn，开关位同号 | 确 | 参数包顺序（callImpl 模板名）；位测试 0x1009d3384、0x1009d3ce4、0x1009d3edc、0x1009d4ce4、0x1009bef34；tag 写入 0x1009d3c1c `mov w8,#8`、0x1009d4abc `#9`、0x1009d4b48 `#0xa`、0x1009d51d0 `#0xb`、0x1009bf184 `#0xc` |
| 窗口：Consonant/XnToXng/XngToXn {1}、HuiFei {3}、Retroflex 2..6 | 确 | NeedWindowSize 0x1009b3898 / 0x1009b3ac0 / 0x1009b3f4c / 0x1009b3e04 / 0x1009b3c08；callImpl 里 `cmp x10,#0x18`、`cmp x9,#0x48`、0x1009d3f00 |
| 五个模糊增强器都不扣预算，只要求预算非负 | 确 | Consonant 0x1009d39ac→0x1009d3c38；XnToXng 0x1009d3eb0→0x1009d4ad4；Retroflex 0x1009d3f18→0x1009d4b3c；HuiFei 0x1009d5048 `asr w4,w27,#1`；XngToXn 0x1009bf0fc→0x1009bf19c |
| 0x271ffa08 是 `NextInputMustBeSep`（韵母定死、下一步须分隔） | 确（名字）/ 推（语义） | visitor 0x1009bc8e8 符号名；各增强器 `mov w11,#0xfa08; movk #0x271f` |
| Consonant / Retroflex / HuiFei 只在音节首 | 确 | 0x1009d38b4、0x1009d413c、0x1009d4ec4（标签 `,` 或 0） |
| 声母模糊不叠（父或隔 InsertSep 的祖父 ∈ {8,10} 就跳过） | 确 | 0x1009d38c0–0x1009d3974；visitor 名 IsFuzzyInitials 0x1009bca78、IsInsertSeparator 0x1009bcc08 |
| XnToXng / XngToXn / Retroflex 不紧跟敲错（父 tag ∈ 0x0fdfff0f） | 确 | 0x1009d4000、0x1009bf130、0x1009d4214 |
| Consonant 四组 nl / rl / hf / gk，换键后游标不动、换出的键挂成 pending | 确 | 0x10074329c–0x100743374，cstring 0x102c93d65..d6e；Emit 参数 0x1009d3bd4–0x1009d3c3c |
| XnToXng：看到 n → 走 n → 后缀是 an/ian/uan/en/in/on → 再走 g → 在 …ng 上生节点 | 确 | 0x1009d3eb8、0x1009d4568、0x1009d48f8/0x1009d490c、0x1009d4964、0x1009d4ad8 |
| XngNasalOpt 六个后缀 an / ian / uan / en / in / on | 确（串）/ 推（布尔组合） | 0x102c98387、0x102c9957b、0x102c9958c、0x102c5daec、0x102c9956a、0x102cc9125；lambda 0x1009bde98 / 0x1009be204 |
| XngToXn：看到 g 且父游标以 Xn 结尾 → 游标不动、吞掉 g | 确 | 0x1009bf100–0x1009bf1a0 |
| Retroflex 双向：无 h 插 h、有 h 删 h；开关 +0x26 三位；record b1=1 表示 zh→z | 确（判定、开关、record）/ 推（pending 拼法） | 0x1009d4248–0x1009d4664、0x1009d4b28、lambda 0x1009d6570 |
| HuiFei 只做 fei↔hui，音节首，窗口 3 | 确 | 0x1009d4f00–0x1009d5044、0x102e79990 / 0x102e799a8、0x1009d51b8 |
| 选项字节 → 增强器位图 | 确 | 0x1007403b4–0x100740494 |
| 敲错换键只换邻键（t→q 不可能） | 确（B1） | `VisitT26KeyBoardNeighbour*` 0x100f2ef54 / 0x100f2efe0 / 0x100f2f06c |
| 模糊与敲错在打分里是两套独立计数，权重来自数据 | 确（B1） | 0x1009f0414 |

---

## 3. 青简落点

### 3.1 现状：模糊音在哪一步展开，为什么 `tin` 走不到 `ting`

调用链（路径都在 `/Users/liyuqing/sproot/qingjian/crates/qingjian-core/src/`）：

1. `engine/query/phonetic.rs::query_phonetic` → `engine/mod.rs::segment_longest_prefix(keys)` → `parser::segment`：
   `parser/mod.rs::segment_chunk` 只用全局 `SYLLABLE_TRIE`（`parser/trie.rs`，由 `parser/syllable.rs::SYLLABLES` 建）找完整音节，外加 `initial_lengths` 的声母简拼与末尾残缺前缀。**这里没有模糊音的任何参与**。
2. 每种切分 `segmentation.patterns()` → `FuzzyRules::expand`（`fuzzy/rules.rs`）→ `alternatives`：对**已经切出来的**音节逐条套规则，`apply_once` 只保留合法的结果。
   源拼写本身可以不合法（`apply_once` 不检查输入），但切分阶段从来不会切出不合法的完整音节，所以这条能力用不上。
3. `tinwosuo`：`tin` 不在音节表 → 最优切分 `ti n… wo suo`（4 音节、1 个非末尾简拼）。
4. `correction::unlikely_pinyin`（`correction/mod.rs`）看到非末尾的残缺音节 → `engine/correcting.rs::find_correction` → `correction::candidates`：全部一处编辑，**换字母不限邻键**（`correction/edit.rs::variants` 对每位试 a..z）。
   每个候选 `convert_sentence_static(…, false)`，而这里的 `expand_positions` 照样套模糊音，于是 in-ing 开着时 `qinwosuo` 的 `qin` 也能读成 `qing`。原型里打出的各候选分数（`score = 整句分 − 5.0`）：

   | 纠正 | 整句 | score |
   |---|---|---|
   | qinwosuo（t→q） | 请我说 | −19.52 |
   | tianwosuo（补 a） | 天我说 | −20.74 |
   | tingwosuo（补 g） | 听我说 | −21.03 |
   | 原样 `ti n wo suo` | 替你我说 | −20.57（raw） |

   最好的纠正 −19.52 > raw −20.57 → 纠成「请我说」。只开 s-sh 时 `qinwosuo` 读不成 `qing`，最好的纠正 `tianwosuo` −20.74 < raw，不纠，出「替你我说」——和 issue 里两种现象一一对上。
   根因两层：**(1) 切分不认 `tin`，模糊音读法压根不在比较里；(2) 整段纠错的换字母不限邻键，又叠了模糊音，`t→q` 这种远键替换也付同一个 5.0。**
5. 双拼（#306）：`shuangpin/scheme.rs::Scheme::syllable` 只认 `parser::is_syllable` 的声母 + 韵母组合，`s`+`uang` 不合法 → 解码停在这里留作尾巴，后面的模糊音同样没机会。

### 3.2 受影响面：「源拼写不合法、套一次规则就合法」的完整清单

用青简的 `SYLLABLES` 与 `FuzzyRules::apply_once` 同一套规则算（脚本 proto/d1-affected.py；全开共 60 个）：

| 规则 | 个数 | 源 → 目标 |
|---|---|---|
| z_zh | 3 | zua→zhua、zuai→zhuai、zuang→zhuang |
| c_ch | 3 | cua→chua、cuai→chuai、cuang→chuang |
| s_sh | 5 | sei→shei、shong→song、sua→shua、suai→shuai、suang→shuang |
| n_l | 3 | len→nen、nia→lia、no→lo |
| f_h | 12 | fai→hai、fao→hao、fe→he、fong→hong、fua→hua、fuai→huai、fuan→huan、fuang→huang、fui→hui、fun→hun、fuo→huo、ho→fo |
| l_r | 18 | len→ren、lua→rua、lui→rui、ra→la、rai→lai、rei→lei、ria→lia、rian→lian、riang→liang、riao→liao、rie→lie、rin→lin、ring→ling、riu→liu、ro→lo、rue→lue、rv→lv、rve→lve |
| an_ang | 17 | biang→bian、cuang→cuan、diang→dian、duang→duan、juang→juan、luang→luan、miang→mian、nuang→nuan、piang→pian、quang→quan、ruang→ruan、suang→suan、tiang→tian、tuang→tuan、xuang→xuan、yuang→yuan、zuang→zuan |
| en_eng | 2 | len→leng、ten→teng |
| in_ing | 2 | din→ding、tin→ting |

- 其中「连任何合法音节的前缀都不是」的（输入末尾时解析器连残缺音节都给不出，直接变尾巴）：z_zh 2（zuai zuang）、c_ch 2、s_sh 4（sei shong suai suang）、f_h 10、l_r 14、an_ang 17；n_l、en_eng、in_ing 为 0（`tin` 是 `ting` 的前缀，所以 `tin` 敲在末尾时本来就能按前缀出 听，只有在句中才坏）。
- 需要两步、中间态也不合法的只有 `nua→rua`、`nui→rui`（n→l→r）。RefIME 的 IsFuzzyInitials 同样禁止声母模糊叠两次，**不支持**，与 RefIME 一致。
- an_ang 一侧：`tiang` 这类是「多敲了 g」，开 an-ang 的人敲 `tiang` 想要 天，读得通是对的。

### 3.3 改法：(a) 为主，(b) 作为它的代价规则，二者合一

**(a) 切分阶段认「模糊源拼写」——等价于 RefIME 的按键级变换，但不动青简「切分 → 查词」的架构。**
RefIME 在 trie 游标上做变换；青简的切分 DP 本来就是在音节 trie 上走，所以只要多给它一张「只有模糊音才读得通的拼写」的小 trie，让 DP 也把这些长度当完整音节，
后面 `FuzzyRules::expand` 已经会把 `tin` 展开成 `ting`（`apply_once` 不查输入合法性），查词、整句、代价、上屏消耗都不用改。

| 文件 | 改动（原型 diff：proto/d1-fuzzy-sources.diff，10 个文件 +168/−40） |
|---|---|
| `fuzzy/rules.rs` | 新 `pub fn sources(&self) -> Vec<String>`：对每个合法音节套一次规则（不过滤）得到的拼写里，取「本身不合法、`apply_once(form, true)` 非空」的，排序去重。原 `apply_once` 拆成 `swaps`（只套规则）+ `apply_once`（过滤），行为不变。 |
| `parser/segmentation.rs` | `Syllable` 加 `pub fuzzy: bool`（敲的拼写只有模糊音才读得通）与 `Syllable::fuzzy(text)`；`Segmentation::fuzzy_count()` |
| `parser/mod.rs` | `pub use trie::SyllableTrie`；新 `pub fn segment_with(input, fuzzy: Option<&SyllableTrie>)`，`segment` = `segment_with(input, None)`；`segment_chunk` 多收这个参数，`fuzzy.matches(rest)` 的长度在没有同长合法音节时作为 `(len, complete, only_fuzzy)` 的完整音节入 DP；`sort_key` 改为 `(音节数, 残缺数 + 模糊数, 模糊数, 前长后短)`；**最优切分不含模糊音节时整段改用 `segment(input)` 的结果**（见下「两条护栏」） |
| `engine/mod.rs` | `Engine` 加 `fuzzy_sources: Option<parser::SyllableTrie>`；新 `segment_longest_prefix_with(text, fuzzy)`，原函数 = `…_with(text, None)` |
| `engine/setup.rs` | `set_fuzzy` 在规则变化时重建 `fuzzy_sources`（全关时 `None`）；`decode` 与 `takes_semicolon` 用 `scheme.decode_with(keys, self.fuzzy_sources.as_ref())` |
| `engine/query/phonetic.rs`、`engine/correcting.rs`、`engine/commit/mod.rs`（整句对得上判断那处） | `segment_longest_prefix` → `segment_longest_prefix_with(…, self.fuzzy_sources.as_ref())` |
| `correction/mod.rs::unlikely_pinyin` | 最优切分里有 `fuzzy` 音节也算「不像话」→ 整段纠错照样试（这是 (b)，见下） |
| `shuangpin/scheme.rs` | 新 `decode_with(keys, fuzzy)`，`decode` = `decode_with(keys, None)`；两键配不出合法音节时 `fuzzy_syllable(first, second, trie)`：声母 + 该键任一韵母落在 `fuzzy_sources` 里就收原拼写（`suang`），后面照常模糊展开 |

两条护栏（都在原型里，缺一条回放就会动）：

1. **排序**：`sort_key` 第二项用「残缺数 + 模糊数」。否则 `shoufe`（全开）的 `fe` 会以模糊音 `fe→he` 的完整音节压过末尾前缀 `fe…`，收费 掉到第 2（原型 p1 实测）。
2. **合法优先**：最优切分不含模糊音节时，直接返回不带模糊 trie 的切分。否则 `dingan`（in-ing）会多出 `din gan` 一路，词候选第 2、3 位从 定安镇 / 定 变成 定岗 / 顶岗；
   `dinan` 会冒出 定案。代价是 `dinan` 想要 定案 的人得不到它（RefIME 能），换来「合法拼写的候选与不开模糊音逐条一致」——这是 issue 与 contributing 的硬要求，先守住。

**(b) 有模糊路径时整段纠错的代价顺序**：不是「有模糊路径就不纠」，而是让模糊音读法进入噪声信道比较——
`unlikely_pinyin` 对模糊音节返回真，`find_correction` 照常生成候选，但 `raw_score` 取的是最优切分（此时就是模糊音读法，`tin wo suo` 的整句分已含两次 ln 2 的模糊代价），
纠正必须 `score − 5.0 > raw` 才接管。`tinwosuo`：raw ≈ 听我说 −16.0 − 2·0.69 ≈ −16.7，最好的纠正 请我说 −19.5，不纠，差 2.8。
保留纠错入口的理由（原型 p1 的教训）：若模糊音节直接算「像话」，`dakaiho`（全开，`ho→fo`）就不再纠成 打开后，`hone` / `serv` 的英文也会被 `ho ne` / `se rv` 挤掉。
代价顺序由此是：精确 0 < 模糊 ln 2/音节 < 音节级敲错 5.0−折扣（≥ 2.0）< 整段一处编辑 5.0，与 RefIME「模糊不扣预算、敲错扣」同向。

不做的（第一步）：
- 不改整段纠错「换字母不限邻键」。这是 #319 的第二层根因，但改它会动没开模糊音的用户的回放（`correction::candidates` 是全局行为），另起一个任务按 B1 的邻键表 / `TypoCosts` 处理，用 `data/eval/typo-adjacent.tsv` 量。
- 不加 RefIME 有而青简没有的规则（g-k、hui/fei 专用、on→ong、un/ong、an/ai、eng/ong、按 an/ian/uan 分开的鼻音开关）——那是用户可见的新设置，单独设计。

**与 PR #330 的关系**：#330 在 `Scheme::decode` 里对每个两键组合现场跑一次 `fuzzy.expand` 判断「模糊后有没有合法写法」；
通用机制下这件事就是 `fuzzy_sources.contains(initial + final)`，表在 `set_fuzzy` 时算一次。把 #330 的 `tests/shuangpin_fuzzy.rs` 原样拷进原型（/tmp/d1work/shuangpin_fuzzy_pr330.rs），3 个测试全过：
七套方案 `s`+`uang` 出 双、选 双 后只消耗两键、`;` 韵母续输、关模糊音 / 无关规则（n-l）时 `sd` 仍报错、`sj`/`si` 合法解码不变。
另外它顺带覆盖了 #330 没管的情况：小鹤 `tb`（t + in）开 in-ing 出 听、`tbwouo` 出 听我说。建议 #330 改成在这套机制上的一个用例（或合并时由维护者替换实现、保留它的测试与文档改动）。

### 3.4 与青简约束

- Core 平台无关：全在 `qingjian-core`；壳不动。
- 一个类型一个文件：没有新类型（`SyllableTrie` 原样转为 `pub`）；`segment_with` / `segment_longest_prefix_with` 是自由函数，留在原模块文件。
- 输入优先：`sources()` 只在 `set_fuzzy` 时跑（全表 400 来个音节 × 9 条规则）；切分多查一张 ≤60 词的小 trie；合法优先护栏在最优切分合法时会再切一遍（微秒级）。
  回放「按键同步部分」p50 / p99：改前 0.4 / 2.2 ms，原型 0.3 / 2.1 ms（全开，噪声内）。
- 用户可感知：开了模糊音后 `tinwosuo`、`suang`、双拼 `sd` 的结果变了 → 同一提交改 `docs/user/input/fuzzy-and-shuangpin.md`（写「敲成不合法的拼写也按模糊音读，例如开 in/ing 后 tin 当 ting」），按键没变不动 `keys.md`。
  实现要点进 `docs/notes/crate-notes.md` 的 `qingjian-core` 段（模糊源拼写表、两条护栏、纠错比较口径）。
- 还用旧 `segment_longest_prefix`（不认模糊源）的地方：`engine/composing.rs` 两处按音节删除 / 移光标的单位长度、`engine/prediction/mod.rs` 两处、`engine/mod.rs::marked_rest`。
  原型没改，`tin` 在这些地方仍按 `ti` + `n` 计；落地时一并换成 `_with` 版本（`composing.rs` 的两处是自由函数，要把 `Option<&SyllableTrie>` 从调用方传进去）。

---

## 4. 复刻步骤

### 4.1 步骤

1. `fuzzy/rules.rs`：拆 `swaps` / `apply_once`，加 `sources()`（文档注释写清「不合法、套一次规则就合法；规则互逆，从合法音节反套枚举」）。
2. `parser`：`Syllable.fuzzy`、`Segmentation::fuzzy_count`、`SyllableTrie` 转 `pub`、`segment_with`、`segment_chunk` 收模糊 trie、`sort_key` 第二项改「残缺 + 模糊」、合法优先护栏。
3. `engine`：`fuzzy_sources` 字段与 `set_fuzzy` 重建；`segment_longest_prefix_with`；`query_phonetic`、`find_correction`、`commit` 那处整句对照、`composing.rs` / `prediction/mod.rs` / `marked_rest` 全部换成带模糊表的版本。
4. `correction::unlikely_pinyin`：模糊音节算「不像话」。
5. `shuangpin/scheme.rs`：`decode_with` + `fuzzy_syllable`；`engine/setup.rs` 的 `decode` / `takes_semicolon` 传表。
6. 文档：`docs/user/input/fuzzy-and-shuangpin.md`、`docs/notes/crate-notes.md`。
7. 验收：`cargo test -p qingjian-core`；`tools/release/gate-pass.sh`（默认模糊音关，必须逐位不变）；另跑一遍带 `--fuzzy all` 与 `--fuzzy in-ing,s-sh` 的 `--eval-text` 与 `--replay … --clean2 --replay-details`，与改前 diff 必须为空（§4.3 的方法）。

### 4.2 单测清单

原型里写好并通过的在 proto/d1-fuzzy_sources_test.rs（集成测试，小词库，不依赖语言模型）：

| 测试 | 输入 | 期望 |
|---|---|---|
| sources_are_the_illegal_spellings_one_rule_away | in-ing / s-sh / 全关 / 全开 | `["din","tin"]`；`["sei","shong","sua","suai","suang"]`；空；全开 60 个 |
| segmentation_reads_fuzzy_sources_as_complete_but_after_legal_ones | `segment_with("tinwosuo", in-ing+s-sh)` | 首个切分 `tin wo suo`，`tin.fuzzy == true`、`wo.fuzzy == false` |
| 同上（合法拼写不受影响） | `dingan` / `dinan` / `nihao` | `segment_with(…) == segment(…)` 逐条相等 |
| 同上（末尾前缀优先） | `shoufe`，全开 | 首个切分 `shou fe…` |
| tinwosuo_is_read_by_fuzzy_rules_not_corrected | `tinwosuo`，in-ing+s-sh | 首选 听我说，`query.correction == None` |
| 同上（关模糊音对照） | `tinwosuo`，全关 | 首选不是 听我说（行为同改前） |
| 同上（合法拼写对照） | `dingan`，in-ing | 首选 定案 |
| suang_needs_s_sh | `suang`，s-sh / n-l | 首选 双 / 候选里没有 双 |
| shuangpin_pairs_decode_through_the_same_sources | 自然码 `sd`、`sdrf`，s-sh；全关 `sd`；小鹤 `tb`，in-ing | 双；双人；无 双；听 |
| committing_a_fuzzy_word_consumes_its_typed_keys | `tinwosuo` 选 听 | 上屏 听，剩 `wosuo` |
| PR #330 的三个测试（/tmp/d1work/shuangpin_fuzzy_pr330.rs） | 七套方案 s+uang、`;` 续输、开关对照 | 原样通过 |

落地时再补（原型没写）：
- `fuzzy/rules.rs` 单元测试：`sources()` 与 §3.2 表逐条相等（防止以后改规则悄悄扩大受影响面）。
- `parser` 单元测试：`'` 分段时模糊音节只在各自段内生效（`tin'wo`）；`MAX_SEGMENTATIONS` 截断后模糊切分不挤掉合法切分（最优合法时护栏已保证，最优是模糊时看前 8 条里仍有 `ti n… wo suo`）。
- `engine/tests/correction.rs`：全开时 `dakaihou` 少敲 u 的 `dakaiho` 仍纠成 打开后；`hone` / `serv` 的英文首选不变；`rinshi`（l-r）首选 临时（模糊音读法胜过纠成 认识，因为 ln 2 < 5.0）。
- `composing.rs`：`tinwosuo` 按音节删除一次删掉 `suo`、再一次删 `wo`、再一次删 `tin`（不是 `n`）。

### 4.3 原型实测（`git worktree add /tmp/d1-proto HEAD`，HEAD = dd1ce657；已 `git worktree remove --force`）

CLI 首选（`--config /tmp/empty.toml`）：

| 输入 | 改前 | 原型 |
|---|---|---|
| `--fuzzy in-ing,s-sh tinwosuo` | 纠正 → `qin wo suo`，请我说 | 切分 `tin wo suo`，**听我说**，无纠正 |
| `--fuzzy in-ing tinwosuo` | 请我所（纠正） | 听我所（`suo` 无 s-sh，合理） |
| `--fuzzy s-sh tinwosuo` | 替你我说 | 替你我说（不变：`tin` 不是 s-sh 的源） |
| `tinwosuo`（全关） | 替你我所 | 替你我所 |
| `--fuzzy s-sh suang` | 数羊 | **双** |
| `--fuzzy f-h fui` | 服（`fu` + 尾巴 `i`） | 会 |
| `--fuzzy c-ch cuangkou` | 出港口 | 窗口 |
| `--fuzzy l-r rinshi` | 认识（纠正） | 临时（模糊读法胜过纠正） |
| `--fuzzy all dakaiho` / `shoufe` / `hone` / `serv` / `raho` | 打开后 / 收费 / hone / service / 绕 | 同改前 |
| `--fuzzy in-ing dingan` / `--fuzzy in-ing,s-sh dinan` | 定案… / 迪南… | 逐条同改前 |
| `--shuangpin ziranma --fuzzy s-sh sd` / `sdrf` | 尾巴 `sd` / 无候选 | 双 / 双人 |
| `--shuangpin xiaohe --fuzzy in-ing tbwouo` | 无候选 | 听我说 |

评测（`tools/eval/offline.toml` + `--extra-dict data/generated/dicts/idioms.qj`，不带神经模型；脚本 /tmp/d1work/evalrun.sh）：

| 配置 | 整句评测 首选 / 前三 / 字准（1878 句） | clean2 回放 词首选 / 整句首选 | 改前 vs 原型 |
|---|---|---|---|
| 模糊音全关 | 31.5% / 36.4% / 76.2% | 88.9% / 70.3% | 完全一致 |
| in-ing,s-sh | 30.6% / 35.1% / 75.5% | 88.9% / 71.0% | 完全一致；`--replay-details` 1244 条 0 差异 |
| 全开 | 25.8% / 30.6% / 71.6% | 87.9% / 65.2% | 完全一致；`--replay-details` 0 差异 |

（没加两条护栏的 p1 版本在全开下回放词首选 −2 条、整句 −1 条：`serv`、`dakaiho`、`raho`、`shoufe`、`hone`，就是 §3.3 两条护栏和 (b) 的来由。）
注意：评测集与回放日志都是在模糊音关着时采的，几乎不含 `tin` 这类拼写，**上表只证明「不伤合法输入」，不证明收益**；收益靠 §4.2 的定点测试。
没跑：带通变模型的 fusion 配置、`gate-pass.sh` 全套（默认模糊音关，按上表「全关完全一致」推断不动）、`cargo test --workspace`（只跑了 `qingjian-core` 的 363 个库测试 + 新增 9 个集成测试 + clippy `-D warnings`，全过）。

---

## 5. 未知

- `BeamSearchParamsProto` 里模糊权重 f18 与敲错权重的具体数值：二进制里只有偏移；/tmp 材料里没有超参数据文件，`/Library/Input Methods/RefIME.app/Contents/Resources` 粗看也没有同名资源（没深挖，按 BRIEF 不导出数据）。下一步看谁加载 `BeamSearchParamsProto`（B1 §5 同一条）。
- Retroflex z→zh 方向 lambda（0x1009d6504）内部按 pending 笛卡尔积走游标的循环没逐条跟完，record 的 b1 在这条路上是否为 0、b0 写什么是 [推]。
- XngNasalOpt 六个字节与 `_wxime_input_augment_option` +0x9..+0xb / +0x10..+0x12 的一一对应是 [推]（只确认了「任一非零就开 0x900」）。
- `reduceTypoCandidateV3` 用的特征 id 0x10 / 0x11 / 0x27 对应哪个特征名、两个阈值从哪来：没对上特征枚举表。
- RefIME 里 `on→ong`（XngNasalOpt[5]）在设置界面上叫什么、默认开不开：没看 Swift 设置页。
