# B5 晚到结果的插位规则（CloudInputMerger）

范围：WeType 2.2.3 arm64，源文件 `wxime/engine/session/cloud_input_merger.cpp`（字符串 0x102cea3a4）。全部为静态阅读，没有运行。
常数一律是 **WeType 的值，青简要自己重扫**。

---

## 1. WeType 怎么做的

### 1.0 一图看懂

```
本地候选出来 (CandidateUpdatedEvent, 不带云结果)
   └─ emit_state=1；缓存这份本地列表 (LocalCacheHolder)；start = steady_clock::now()   ← 计时起点
云结果到 (CloudResultEvent, type==2)
   ├─ isOutOfTime()==false（≤200ms，或按用户选词速度缩短）
   │     Merge(): 本地前 5 + 普通云词 → mergeSort → mergeExpPos(带期望位置的云词) → 余下本地照旧
   │       ├─ 合并后前 min(5,已出) 个字与已显示的一样 → 返回 2，不发信号（候选窗不刷新）
   │       └─ 不一样 → SetGenerator + 发候选更新信号（emit_state=4）
   └─ isOutOfTime()==true
         ├─ AB enable_cloud_time_out != "true"（默认）→ 直接丢弃（emit_state=5）
         └─ == "true" → MergeTimeoutCloudFromFourth：本地前 3 个原样，云词从第 4 位起插（emit_state=4）
```

### 1.1 数据结构（`CloudInputMerger::Impl`，构造 0x100c0c018）

| 偏移 | 类型 | 含义 | 来源 |
|---|---|---|---|
| +0x08 | f32 | `user_dict_local_first_hot_val_thresh_` | AB `..._protect_user_expt` 第 1 段，缺省 0.1 |
| +0x0c | u32 | `user_dict_local_first_td_thresh_`（秒） | 第 2 段，缺省 604800（7 天） |
| +0x10 | u32 | `user_dict_local_first_min_runes_` | 第 3 段，缺省 2 |
| +0x14 | f32 | `user_dict_local_first_short_hot_val_thresh_` | 第 4 段，缺省 1.0 |
| +0x18 | u32 | `user_dict_local_first_short_td_thresh_`（秒） | 第 5 段，缺省 1000 |
| +0x30 | ptr | 用户画像服务（`isOutOfTime` 取平均选词速度） | InitContext |
| +0x40 | i64 ns | 计时起点（steady_clock） | CandidateUpdated lambda 0x100c11b14 写 |
| +0x48 | mutex | 保护 +0x40 | |
| +0x88 | bool | `disallow_force_top1`：PC（`[ctx+0x188]∈{5,6}`）且非内测号时 = `AB != "1"`；其他平台 0 | `mmkb_cli_engine_pc_allow_cloud_force_top1_expt` |
| +0x89 | bool | 上下文某 vector 非空（`[ctx+0x238]!=[ctx+0x240]`），开「长云词不抢首位」检查 | 构造 0x100c0c4a8 |
| +0xa0 | i32 | 超时 ms，缺省 **200**；AB `mmkb_cli_engine_cloud_time_out_ms` `stoi ≥ 1` 时覆盖 | 0x100c0c0e4 / 0x100c0c37c |
| +0xa4 | bool | 按用户选词速度收紧时限：AB `mmkb_cli_engine_use_user_profile_dict == "true"` | 0x100c0c2f8 |
| +0xa5 | bool | `enable_time_out_cloud_`：AB `mmkb_cli_engine_enable_cloud_time_out == "true"`，**缺省 false** | 0x100c0c264 |
| +0xb8.. | LocalCacheHolder | 已显示的本地候选（SharedGenerator + 若干 shared_ptr），+0x260 有值标志，+0x268 mutex | |
| +0x2a8 | CloudPendingHolder | 云端改写拼音栏用 | |
| +0x300 | atomic i32 | emit_state（上报用）：1 本地已出、3 合并后可见区无变化、4 合并并重发、5 超时且不允许晚插而丢弃、8 云结果为空、99 错误/本地缓存缺失；同步路径另有 2/6/10、7/11、9/12 | |
| +0x304 | bool | IsDailyTestClient | |

日志：`"AB_params enable_time_out_cloud_: "` + `", max_timeout_ms_: "`（0x102cea3ec/40f）；`"User protection pram by ab test, user_dict_local_first_hot_val_thresh_:"`…（0x102cea458 起）；
`"Cloud search result skip. time cost ms is "` + `", but avr cost ms is "` / `", but max cost ms is "`（0x102cea597/5c2/5d8）；`"User select speed adjust to "`（0x102cea56c）；
`"insert time out cloud cand: "`（0x102cea5ee）；`"Local cache missing, this should not happen."`（0x102cea53f）。

候选类型 = `CandidateV2+0x138` 处 variant 下标（`CandidateID` 的 32 个备选，nmn_dem 里有完整 variant 名）：
0 TrieMatching、1 Pinyin、3 **CloudPinyinSearch（云词）**、0xd Emoji、0x10 TrieMatchingWithSyllableCorrections（纠错词）、0x11 UserHotWord、
0x12 NumCalculateRes、0x13 Calendar、0x17 End2End（整句模型）、0x1c / 0x1f `BaseUserFixCandidateID<28/31>`（用户固定位置）。
`CandidateV2` 大小 0x310；`+0x140` 文本 std::string；`+0x20` u16 标志位（bit1 见下）；`+0x298` u8 **期望位置 exp_pos**（云词，1 起，0=无）。

### 1.2 计时起点（问题 1 的后半）

`'lambda'(CandidateUpdatedEvent const&)` 0x100c11940：
```c
on_candidate_updated(ev):
  pending.ResetPending()
  if ev.type != 0 { Reset(); forward(ev); return }          // 非普通更新：清状态透传
  if ev.cloud_results == null {                              // 只有本地候选（绝大多数按键）
      emit_state = 1
      local_cache.ResetLocalCache(ev)                        // 记住「这一版已经发给壳显示的列表」
      lock(+0x48); start(+0x40) = steady_clock::now(); unlock // 0x100c11b10/0x100c11b14
      forward(ev); return
  }
  // 云结果已随本地一起到（命中本地缓存，TryResponseByCache）：同步合并
  r = Merge(iter, &ev.cloud, disallow_top1=false, prev_shown=None, ev.e2e, sync=true, ev.extra)
  ... emit_state = 2/6/10 (r==0) | 3/9/12 (r==2) | 7/11/8 (r==1)；TryUpdateCloudPendingInput；forward(ev)
```
**计时起点 = 本地候选列表发出（交给壳显示）的那一刻**，不是请求发出、也不是壳回报的「已上屏显示」。每次按键产生新的本地列表都会重置。[确]

### 1.3 isOutOfTime（0x100c0e208）

```c
bool isOutOfTime() const {
  i64 limit = timeout_ms;                                    // +0xa0
  if (profile && use_user_speed /*+0xa4*/) {
      i64 avg = profile->sub()->stat(5);                     // vtbl+0x358 → vtbl+0x30(5)：平均选词耗时 ms [推]
      if (avg == 0 || avg > timeout_ms) log("User select speed adjust to", timeout_ms, "prev is", avg);
      else limit = avg;                                      // 用户选得比 200ms 还快：时限缩到他的平均速度
  }
  f64 elapsed_ms = (now() - start) / 1e6;                    // 0x412e848000000000 = 1e6
  if (elapsed_ms > limit) { log("Cloud search result skip. time cost ms is", elapsed_ms, ...); return true; }
  return false;
}
```

### 1.4 CloudResultEvent 处理（外层 0x100c0d1ac，内层 0x100c0d328）——问题 1 的总分支

```c
on_cloud_result(ev):
  pending.ResetPending()
  if ev.type != 2 { forward(ev); return }
  reader_lock; if cancelled { unlock; return }
  cache = take(local_cache)                                   // 已显示的本地列表
  if !cache { log("Local cache missing..."); emit_state = 99; return }
  if isOutOfTime() {                                          // 0x100c0d504
      if !enable_time_out_cloud_ { emit_state = 5; return }   // 0x100c0de34：默认就是丢弃
      split = SplitCloudResults(ev.cloud)                      // (A=无期望位置, B=有期望位置)
      if split 两个都空 { … return }
      gen = MergeTimeoutCloudFromFourth(cache.generator, split)  // 协程帧 0x200，resume 0x100c14868
      iter.SetGenerator(gen); out.copy(cache.extra)
      emit_signal(); emit_state = 4                            // 0x100c0d7c4 / 0x100c0d7d0
  } else {
      r = Merge(iter, &ev.cloud, disallow_top1 = this->+0x88, prev_shown = Some(cache.shared_gen),
                e2e = cache.e2e, sync = false, extra = ev.extra)   // 0x100c0dcb0
      switch r { 2: emit_state = 3 (不刷新)  1: emit_state = 8  0: TryUpdateCloudPendingInput(first); emit_signal(); emit_state = 4 }
  }
```

### 1.5 SplitCloudResults（0x100c0e658）

```c
(A, B) SplitCloudResults(cloud):   // A.reserve(10), B.reserve(3)
  for c in cloud: (c.exp_pos /*+0x298*/ == 0 ? A : B).push(c)
```

### 1.6 Merge（0x100c0ea14，时限内）

```c
int Merge(iter, void** cloud, bool disallow_top1, optional<SharedGen> prev_shown,
          shared_ptr<vector> e2e, bool sync, ExtraInfo extra) {
  (A, B) = SplitCloudResults(*cloud); *cloud = null
  if A.empty && B.empty return 1
  local_gen = iter.MoveGenerator()
  ctx_empty = (ctx->+0x210 字符串为空)                      // 0x100c0eb38；含义未定，见未知
  gen = MergeImpl(local_gen, (A,B), ctx_empty, hot, td, min_runes, short_hot, short_td,
                  disallow_top1, has_ctx(+0x89), e2e, +0x305, +0x306, sync, extra, N=5)  // 0x100c0eb8c: N=5
  if prev_shown {                                           // 只在已有显示列表时比较
      n = prev_shown.cached_count
      if prev_shown.done || n >= 5 {
          k = min(n, 5)                                     // 0x100c0ed58
          for i in 0..k: if gen[i].text != prev_shown[i].text → 把已取出的 i+1 个接回 gen 前面；SetGenerator；return 0
          return 2                                          // 前 k 个逐字相同：不换、不发信号
      }
  }
  iter.SetGenerator(gen); return 0
}
```

### 1.7 MergeImpl（协程 resume 0x100c15b38）

```c
generator MergeImpl(local, (A,B), ...):
  top = take(local, 5)                                      // 只有本地前 5 参与合并
  saved_corr = 最后一个 type==0x10（纠错词）的拷贝
  (L, info) = mergeSort(top, A, ctx_empty, hot, td, min_runes, short_hot, short_td, disallow_top1, has_ctx, e2e, ...)
  co_yield* mergeExpPos(L, B, info, sync, extra)            // 帧 0x108，resume 0x100c1521c
  for c in local(第 6 个起):
      if c.type == 3 && saved_corr && c.text == saved_corr.text: co_yield saved_corr   // 同字云词换回本地纠错词
      else co_yield c
  // 注意：A 里没被 mergeSort 用掉的云词就此丢弃
```

### 1.8 mergeSort（0x100c0fa60）——普通云词（A）的插位与本地首选保护

```c
(vector out, LocalFirstTop1Info{protect, not_forced})
mergeSort(L /*本地前5*/, A, ctx_empty, hot, td, min_runes, short_hot, short_td, disallow_top1, has_ctx, e2e, ..) {
  if L.empty: return (A?, …)                                // 0x100c0fc34
  // ① 找第一个「本地前 5 里也出现在云端 A 里」的词
  common = first (k, m) with L[k].text == A[m].text         // 0x100c0fb58..0x100c0fc24
  // ② 云端是否强推首位
  not_forced = !(A[0].GetCandidateFlag() & (1<<22))        // 0x100c0fce0（visit 表 0x1031d5638 → GetCandidateFlag）
  // ③ 本地首选保护
  if disallow_top1                     protect = true      // PC 且 AB != "1"
  else if IsHardFixedByUser(L[0]) || L[0].type == 0x1f  protect = true
  else protect = IsProtectLocalFirst(L[0], ctx_empty, hot, td, min_runes, short_hot, short_td)
  a = 0; pending = false
  for i in 0..L.len:
    if pending { out.push(A[a++]); pending = false }
    else if i == 0 && a == 0 {                              // 首位
       if protect || not_forced { }                         // 首位留给本地
       else if common && common.k == 0 { out.extend(A[0..common.m]) ; a = common.m }  // 云端排在这个共同词前面的几条上前
       else if has_ctx && utf8_len(A[0]) > 6 { pending = true }   // 长云词（>6 字）不抢首位，排第 2
       else { out.push(A[0]); a = 1 }                       // 云词抢首位
    }
    else if a == 0 {                                         // 还没插过云词
       if i <= 3 && NoSeparateIncludeE2E(L[i]) { }          // 这几种本地词前面不插
       else if common && common.k == 0 && common.m == 0 {   // 本地首选 = 云端首选
           if i <= 2 && (L[i].type != 0 || L[i].flags & 2) { }   // 只在「普通 Trie 词且无 bit1」前插
           else { out.extend(A[1..min(3,|A|)]); a = …}      // 插云端第 2、3 条，最多 2 条
       }
       else if common && i >= common.k { out.extend(A[a..common.m]); a = common.m }
       else { out.push(A[a++]) }                            // 普通情况：插 1 条
    }
    out.push(L[i])
  if out.len == L.len:                                      // 一条云词都没插进去
     common ? out.extend(A[0..common.m]) : out.push(A[0])
  return (out, {protect, not_forced})                        // 0x100c10288: strh (not_forced<<8)|protect
}

NoSeparateIncludeE2E(c) =                                   // 0x100c10318：这些词前面不许插云词
  IsHardFixedByUser(c) || c.type ∈ {0xd Emoji, 0x11 UserHotWord, 0x1f UserFix}  // 掩码 0x80022000
  || GetDictID(c) == 0x84 || c.type ∈ {0x12, 0x13} || GetDictID(c) == 0x8c || c.type == 0x17 /*E2E*/
```
所以时限内**普通云词最多插 1 条**（共同词情况下是「排在共同词之前的那几条」或「云端第 2、3 条」），一般落在第 2 位；只有服务端打了 bit22 且本地首选不受保护时才抢首位。没用上的 A 全部丢弃。[确：控制流；推：bit22 = 服务端「可置顶」标记]

### 1.9 mergeExpPos（协程 resume 0x100c1521c）——问题 2

输入：L = mergeSort 的结果，E = B（带 exp_pos 的云词），info，`sync`（帧 +0x105），`extra`（帧 +0xf0，4 个布尔）。

```c
generator mergeExpPos(L, E, info, sync, extra):
  set<string> seen                                          // 帧 +0x30，max_load_factor 1.0
  i = 0 (L 下标, 帧+0xf4)  j = 0 (E 下标, 帧+0xf8)  pos = 1 (输出名次, 1 起, 帧+0xfc)
  while i < |L| && j < |E|:
     defer { rotate_after_cloud_top1() }                    // 0x100c11154
     c = L[i]
     if skip_local(c, sync, extra) { i++; continue }        // 0x100c10ec4，服务端指示的过滤（见下）
     if !(IsPinyinLike(c) || c.type == 0x17 || c.type == 3) { yield c; i++; pos++; continue }
     if IsHardFixedByUser(c) || GetDictID(c) == 0x84 || c.type ∈ {0x12,0x13} || GetDictID(c) == 0x8c
         { yield c; i++; pos++; continue }                  // 这些位置永远不让
     e = E[j]
     if c.text == e.text {                                  // 云词与本地词同字：保留本地那个对象，云的那条消耗掉
         if seen.insert(c.text) { yield c; pos++ }
         i++; j++; continue
     }
     if c.type == 0 && !(c.flags & 2) {                     // 普通 Trie 词且无 bit1：云词直接插在它前面
         if seen.insert(e.text) { yield e; pos++ } ; j++; continue
     }
     if pos >= e.exp_pos && !(IsHardFixedByUser(c) || dict==0x84 || type∈{0x12,0x13} || dict==0x8c)
        && (!info.protect || pos > 1) {                     // 到了期望名次；本地首选受保护时不占第 1 位
         if seen.insert(e.text) { yield e; pos++ } ; j++; continue
     }
     if seen.insert(c.text) { yield c; pos++ } ; i++        // 还没到期望名次：先出本地
  // 收尾
  while j < |E|: yield E[j++]                               // 剩下的期望位置云词照样追加
  while i < |L|: if !skip_local(L[i]) yield L[i] ; i++      // 0x100c15874 循环
```
- **期望位置字段**：`CandidateV2+0x298`（u8，1 起）。比较 `pos >= exp_pos`（0x100c155c8），pos 是本次输出的名次。
- **去重**：一张 `unordered_set<string>` 记已出的文本；任何后到的同字条目（本地或云）都跳过。云词与当前本地词同字 → 出本地、丢云。
- **插入上限**：exp-pos 云词没有数量上限（B 预留 3，`reserve(3)` 0x100c0e694），普通云词上限见 mergeSort。
- **本地首选保护**：`info.protect && pos == 1` 时不插（0x100c1575c 起的 `tst/ccmp/cset`）；固顶、0x84/0x8c 词典、计算器/日历结果所在位置永远不让。
- `rotate_after_cloud_top1`（0x100c11154）：若第一个输出是云词（j==1 && i==0），把 L 开头连续的「UserHotWord 或 0x84 词典」词提到原首选前面，被挤下来的本地首选落到它们后面。[推]
- `skip_local`（0x100c10ec4）：`extra[0]` 时丢 GetDictID ∈ {0x47,0x58,0x62} 的本地词；`extra[1]` 且 dict==0xffff 且有特征 27≠0、特征 28≥3 时丢。仅 `sync||extra[2]` 生效。服务端下发的过滤，青简不需要。[推]

### 1.10 MergeTimeoutCloudFromFourth（协程 resume 0x100c14868）——问题 3

```c
generator MergeTimeoutCloudFromFourth(local /*已显示的那份缓存*/, (A, B)):
  out = []; i = 0; j = 0
  while i < |A| && j < |B|:                                 // 普通云词与期望位置云词交织
     if i + j + 4 >= B[j].exp_pos { out.push(B[j++]) }       // 0x100c1498c: add w11,w11,#0x4
     else                         { out.push(A[i++]) }
  out += A[i..] ; out += B[j..]
  n = 0; inserted = false
  for c in local:
     if !inserted && n == 3 {                                // 0x100c14a1c: cmp x10,#0x3 ← 「第 4 位」常数
         for x in out { log("insert time out cloud cand: ", x.text); yield x }
         inserted = true
     }
     yield c; n++
  if !inserted: for x in out yield x                         // 本地不足 3 条：云词接在后面
```
- 「第 4 位」：**0x100c14a1c `cmp x10, #0x3` / `b.lo`**——本地前 3 条先出（第 1 条在 0x100c14cac 处 count=0 时就出），第 4 个名次起插云词；exp_pos 的基数 +4 在 **0x100c1498c**。[确]
- **不去重**：这个函数里没有任何文本比较，已在本地列表里的云词会出现两次（是否有下游去重层没查，见未知）。[确：本函数]
- **插几个**：A、B 全部插入，无上限（A 来自服务端，SplitCloudResults 预留 10/3）。[确]
- 不比较「可见区有没有变」，必发信号。前三位不动是靠位置保证的。

### 1.11 IsProtectLocalFirst / IsHardFixedByUser / LocalFirstTop——问题 4

二进制里**没有叫 `LocalFirstTop` 的函数**，只有结构 `LocalFirstTop1Info`（2 字节：`protect`、`not_forced`，0x100c10288 打包），由 mergeSort 产出、给 mergeExpPos 用。[确]

`IsHardFixedByUser(c)`（0x100c10468）：
```c
c.type == 0x1c (BaseUserFixCandidateID<28>) → true
IsPinyinLike(c) /* type ∈ {0,1,0xd,0xe,0x10,0x1c,0x1f}，表 0x1031d5738 */
   → PinyinIDLike::GetPositionFixedByUser()：词条记录 flags 的 bit8（+0x20 或 +0x40 处 u16 的 0x100）；某一备选恒 true
else false
```
即「用户手动把这个词固定在某个位置」。[确]

`IsProtectLocalFirst(c, force, hot, td, min_runes, short_hot, short_td)`（0x100c10530）：
```c
dict = GetDictID(c)                                         // 表 0x1031d5970
if dict == 0x80 && features.has(4)          return true
if (dict & 0xff80) == 0x80 {                                // 0x80..0xff：用户侧词典 [推]
    if dict == 0x84                         return true
    A = features[22] /*热度 f32*/; B = features[9] /*距上次使用的时间差 f32*/
    if utf8_len(c.text) < min_runes { hot = short_hot; td = short_td }   // 单字用更严的门槛
    if force                                return true     // force = 上文串为空（Merge 传入）
    return A && B && A.is_f32 && B.is_f32 && A >= hot && B <= td
}
if dict == 0xfffe && c[+0x218] {                            // lambda 0x100c10b3c：对组成词逐个看
    for w in c.word_features: if w.has(22) return w[22] >= 1.0 && w[34] == 2u
    return false
}
return false
```
判定的是「本地首选是不是**用户自己常用且最近用过**的词」：缺省门槛（AB 为空时）热度 ≥ 0.1 且 7 天内用过；不足 2 个字的词要热度 ≥ 1.0 且 1000 秒内用过。和分数、整句无关。[确：常数与比较；推：特征 22=热度、9=时间差的语义来自日志字段名 `hot_val` / `td`]

### 1.12 三个 AB 开关——问题 5

| 开关 | 读取地址 | 字段 | 影响 |
|---|---|---|---|
| `mmkb_cli_engine_cloud_local_merger_protect_user_expt` | 0x100c0c5cc | +0x08..+0x18 | 逗号分隔 5 段 `hot,td,min_runes,short_hot,short_td`，只喂 **IsProtectLocalFirst**（mergeSort 的首位保护）；为空时用 0.1/604800/2/1.0/1000；段数不为 5 时保留 HyperParams 的值 |
| `mmkb_cli_engine_pc_allow_cloud_force_top1_expt` | 0x100c0c194 | +0x88 | 仅 PC（平台码 5/6）且非内测号读取；不为 `"1"` 时 `disallow_top1=true` → mergeSort 里 `protect=true`，**云词永远拿不到首位**（含 mergeExpPos 的 pos==1） |
| `mmkb_cli_engine_enable_cloud_time_out` | 0x100c0c20c | +0xa5 | 只影响**超时分支**：`"true"` → MergeTimeoutCloudFromFourth；否则超时结果直接丢（emit_state=5）。缺省为假 |
| （附）`mmkb_cli_engine_cloud_time_out_ms` | 0x100c0c334 | +0xa0 | 时限，缺省 200 |
| （附）`mmkb_cli_engine_use_user_profile_dict` | 0x100c0c2a0 | +0xa4 | 让 isOutOfTime 按用户平均选词速度收紧时限 |

### 1.13 壳怎么刷新——问题 6

Swift `CandidatesStore.updateIterator(_:sessionID:version:mode:candidateFlag:source:)` 的主线程闭包（0x1001412cc）：
```swift
highlightMoved = false            // 0x1001413f8 strb wzr,[x21,#0x58] + didset
self.+0x38 = 0                    // 高亮下标归零 [推]
self.+0x4b = false; self.+0x50 = -1
if sameSession && oldIterator.version > newVersion { drop }   // 0x10014146c: 只丢「版本更旧」的
iterator = new; loadFirstPage(with: mode)                      // 0x100141594
```
壳**没有**按来源区分，任何一次迭代器更新（包括云合并后的那次）都会把高亮归零、回第一页。WeType 不让云结果打扰用户靠的是引擎侧：时限内且前 5 没变就不发信号（Merge 返回 2）；时限外要么丢弃、要么只动第 4 位之后。用户已经移动高亮时会不会被云更新拉回第一格，静态看不出有额外保护（见未知）。[确：闭包里无条件清高亮；推：+0x38 是高亮下标]

---

## 2. 证据表

| 结论 | 置信度 | 地址 / 字符串 |
|---|---|---|
| 超时缺省 200ms，AB `cloud_time_out_ms` ≥1 覆盖 | 确 | 0x100c0c0e4 `mov w8,#0xc8`；0x100c0c37c |
| 计时起点 = 本地候选更新事件（无云结果）发出时 | 确 | 0x100c11b10 `bl steady_clock::now`；0x100c11b14 `str x0,[x20,#0x40]` |
| isOutOfTime：elapsed_ns/1e6 > limit；limit 可被用户平均选词速度收紧 | 确 | 0x100c0e36c–0x100c0e390；0x100c0e24c `mov w1,#5`；串 0x102cea56c |
| 超时且 `enable_cloud_time_out` 非 "true" → 丢弃 | 确 | 0x100c0d50c `ldrb w8,[x24,#0xa5]`；0x100c0de34 `mov w9,#5` |
| `enable_cloud_time_out` 缺省假 | 确 | 0x100c0c0e8 `strh wzr,[x26,#0xa4]`；0x100c0c254 比 "true" |
| 超时路径走 MergeTimeoutCloudFromFourth | 确 | 0x100c0d6d8 协程帧指向 0x100c14868 |
| 「第 4 位」：本地 3 条后插 | 确 | 0x100c14a1c `cmp x10,#0x3` |
| 超时路径 exp_pos 基数 +4 | 确 | 0x100c1498c `add w11,w11,#0x4` |
| 超时路径不去重、全部插入 | 确 | 0x100c14868–0x100c14d48 无文本比较 |
| 云结果按 exp_pos(+0x298) 是否为 0 拆两组 | 确 | 0x100c0e6c0 |
| 时限内只拿本地前 5 合并 | 确 | 0x100c0eb8c `mov w8,#5`；0x100c15c70 |
| 前 min(5,n) 个字没变 → 返回 2，不发信号 | 确 | 0x100c0ed10/0x100c0ed58；0x100c0eec8 `mov w19,#2`；0x100c0dcc0 |
| 云词抢首位需 bit22 且本地首选不受保护 | 确（bit22 语义为推） | 0x100c0fce0 `bic w19,w8,w0,lsr #22`；0x100c0fe5c |
| 长云词 >6 字不抢首位（has_ctx 时） | 确 | 0x100c0ffd4 `cmp x0,#0x6` |
| NoSeparateIncludeE2E 的类型/词典集合 | 确 | 0x100c10350 掩码 0x80022000；0x100c10398 `#0x84`；0x100c103d0 `#0x8c` |
| mergeExpPos 用 seen 集合去重；同字出本地 | 确 | 0x100c15648 / 0x100c155a4 hash set insert |
| exp_pos 比较 `pos >= exp_pos` | 确 | 0x100c155c8 |
| 本地首选受保护时 exp-pos 云词不占第 1 位 | 确 | 0x100c1575c–0x100c15768 |
| IsHardFixedByUser = UserFix 类型或词条「位置已被用户固定」位 | 确 | 0x100c1047c `cmp w8,#0x1c`；0x100c10ab4 `ubfx w0,w8,#8,#1` |
| IsProtectLocalFirst 门槛 0.1/604800/2/1.0/1000 | 确 | 0x100c0c684–0x100c0c6a4 |
| 0x84 词典恒受保护；0x80+特征 4 受保护 | 确 | 0x100c1065c；0x100c105a0–0x100c10624 |
| 特征 22=热度、9=时间差 | 推 | 键 0x16/0x9（0x100c10758/0x100c1083c）+ 日志字段名 |
| PC 上 force_top1 AB 不为 "1" 时首位永远本地 | 确 | 0x100c0c16c `[x19+0x188]-5 ≤1`；0x100c0c1dc；0x100c0fce4 |
| 壳 updateIterator 无条件清高亮、回第一页 | 确（字段语义推） | 0x1001413f8、0x100141404、0x100141594 |
| 壳只丢版本更旧的迭代器 | 确 | 0x10014146c `cmp w8,w27; b.ls` |

---

## 3. 青简落点

### 3.1 现在每种晚到结果怎么并进候选（读代码所得）

| 晚到结果 | Core 侧 | 壳侧合并 | 动哪些位置 | 有没有「已显示多久 / 用户动过」的判断 |
|---|---|---|---|---|
| 云联想词（`qingjian-predict`，`Engine::poll_prediction`，`crates/qingjian-core/src/engine/prediction/mod.rs`） | 只校验拼音、丢过期序号；`PredictionPolicy.slots` 缺省 2（`policy.rs`） | **Mac**：`CandidateLayout::set_cloud`（`crates/qingjian-core/src/candidate/layout/candidate_layout.rs`），填在第一页**末尾**几格、同字丢弃；守卫 `cloud_slots_untouched(page, highlighted, …)` 写在壳里（`apps/macos/src/host/cloud/mod.rs:147`）。**素笺 iOS**：`cloud/crates/qingjian-cloud-bridge/src/session/cloud.rs` 的 `poll()` 把云词插到 `CLOUD_POSITION = 1`（**紧跟首选，第 2 格**），不看高亮、不看时间，同字跳过 | Mac 第一页末尾；iOS 第 2 格起，会把第 2、3… 格的本地词全挤后 | 只有 Mac 有「翻页 / 高亮已落在要被挤的格」判断，没有时间判断 |
| 整句神经重排（通变，`engine/rescoring/mod.rs` `poll_rescoring` → 壳重新 `query()`） | `rescore_paths` 改整句路径顺序，**含首选** | Mac `apps/macos/src/host/model/mod.rs:156`：`page != 0 || navigated` 时不重画，否则整表替换再 `set_cloud`；Windows `apps/windows/server/src/dispatch/rescore/mod.rs:181`、Linux `apps/linux/server/src/dispatch/rescore/mod.rs:160` 同类判断；iOS `cloud/.../session/rescore.rs` `requery_local` 无条件整表替换、云词插回第 2 格 | 整个第一页，包括第 1 位 | 只有「翻页 / 动过高亮」，没有时间窗 |
| 词级神经重排（知微，`engine/rescoring/word_rescore/mod.rs`） | 第一页同结构档的前 ≤6 个词互换，**含首选** | 同上一行（同一次 `poll_rescoring`） | 第一页前几格，包括第 1 位 | 同上 |
| 将来 P3 `/rewrite` | 未实现 | — | — | — |

问题：① 守卫逻辑在 4 个壳里各写一份（Mac/Win/Linux 一样，iOS 没有），违反「平台层不做排序」；② 没有时间窗，用户正要按 1 / 空格时首选可能被重排换掉；③ 用户固定位置（`CandidateKind::Custom(n)`）在 `CandidateLayout` 里位置不变，但整句重排、iOS 的插位不认它。

### 3.2 统一规则（Core）

规则一句话：**候选窗显示后，时限内到的结果可以正常重排（固定位置和受保护的首选除外）；时限外到的、或者用户已经翻页 / 移动过高亮之后到的，只能动第 K+1 位以后（K=3）；用户固定位置的词任何时候都不动。可见区没变就不刷新。**

与 WeType 的差别（有意的）：WeType 超时缺省是丢弃，青简改成「从第 K+1 位起插」（WeType 的 AB 实验分支），理由是青简的云结果比 WeType 慢得多（OpenAI 兼容接口），全丢等于没有云联想；计时起点 WeType 用的是引擎发出列表的时刻，青简改成壳报告的「真的画出来了」，因为 iOS 键盘扩展里主线程到屏幕可能差一帧以上。

新增目录 `crates/qingjian-core/src/engine/late/`（一类型一文件）：

```rust
// late/mod.rs —— mod 声明、re-export、impl Engine 的入口
pub use display::DisplayState;
pub use outcome::LateOutcome;
pub use policy::LatePolicy;
pub use source::LateSource;
pub(crate) use merge::merge_late;

// late/policy.rs
/// 晚到结果的插位策略。常数是 WeType 的值，青简要用真机误选率重扫。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LatePolicy {
    /// 候选窗画出来之后多久内到的结果还能正常重排（WeType 200ms）。
    pub grace_ms: u32,

    /// 过了时限、或用户动过之后，前几位冻结（WeType 3：第 4 位起插）。
    pub frozen: usize,

    /// 时限内云词最多抢几格（WeType 普通云词 1 条）。
    pub max_in_grace: usize,

    /// 首选是用户常用词时不让云词抢（WeType IsProtectLocalFirst）。
    pub protect_top1: bool,
}
impl Default for LatePolicy { /* 200, 3, 1, true */ }

// late/source.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LateSource { Cloud, WordRescore, SentenceRescore, Rewrite }

// late/display.rs
/// 当前这一版候选在壳里的显示状态。只由壳的报告驱动，Core 不碰 UI。
#[derive(Debug, Clone, Default)]
pub struct DisplayState {
    /// 这一版列表的版本号（每次 query 产出新列表 +1）。
    pub(crate) generation: u64,

    /// 壳报告画出来的时刻；None = 还没画出来（结果比画面先到，随便排）。
    pub(crate) shown_at: Option<std::time::Instant>,

    /// 用户翻过页或移动过高亮。
    pub(crate) touched: bool,

    /// 已显示的第一页文本，用来判断「可见区变没变」。
    pub(crate) visible: Vec<String>,
}

// late/outcome.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LateOutcome {
    /// 可见区逐字没变：壳不要重画（WeType Merge 返回 2）。
    Unchanged,
    /// 时限内，按正常规则重排了。
    Reranked,
    /// 时限外 / 用户动过：只改了第 frozen+1 位之后。
    AppendedAfterFrozen,
    /// 没有可插的（全部同字去重掉了）。
    Empty,
}

// late/merge.rs —— 纯函数，便于单测
/// `late` 是晚到方给出的新顺序（重排）或新词（云），`rerank` 区分两者。
pub(crate) fn merge_late(
    shown: &[Candidate],
    late: Vec<Candidate>,
    rerank: bool,
    state: &DisplayState,
    policy: &LatePolicy,
    now: std::time::Instant,
    is_protected_top1: impl Fn(&Candidate) -> bool,
) -> (Vec<Candidate>, LateOutcome);
```

Engine 新增（`late/mod.rs` 的 `impl Engine`）：
```rust
/// 壳：这一版候选已经画到屏幕上了（版本号来自 QueryResult）。
pub fn note_candidates_shown(&mut self, generation: u64);
/// 壳：用户翻页或移动了高亮。
pub fn note_candidates_touched(&mut self);
/// 云词到了：按规则并进当前列表。替代壳里的 set_cloud + cloud_slots_untouched + CLOUD_POSITION。
pub fn merge_prediction(&mut self, prediction: &Prediction) -> LateOutcome;
/// 重排分到了：内部 re-query 后按规则合并（替代壳里的「page != 0 || navigated」判断与整表替换）。
pub fn merge_rescoring(&mut self) -> LateOutcome;
/// 当前列表（壳据 LateOutcome 决定要不要重画）。
pub fn current_candidates(&self) -> &[Candidate];
```

`merge_late` 的规则（伪代码，对应 WeType 的三段）：
```rust
let fixed = |c| matches!(c.kind, CandidateKind::Custom(_));          // = IsHardFixedByUser
let in_grace = state.shown_at.is_none()
    || (!state.touched && now - shown_at <= grace_ms);
let frozen = if in_grace { 0 } else { policy.frozen };
let protect0 = policy.protect_top1 && shown.first().is_some_and(is_protected_top1);
// 1. 先算新顺序：
//    rerank：按 late 的顺序，但 fixed 的词钉回原位，[0, frozen) 原样，protect0 时第 0 位原样
//    cloud ：同字丢云（本地保留，WeType mergeExpPos 规则）；in_grace 时最多 max_in_grace 条插在
//            第一个「不是 fixed / emoji / 整句」的位置（WeType NoSeparateIncludeE2E），且 protect0 时不进第 0 位；
//            !in_grace 时全部插在 frozen 位之后
// 2. 可见区（第一页）逐字与 state.visible 相同 → LateOutcome::Unchanged，不换列表
```
`is_protected_top1`（对应 IsProtectLocalFirst）：查 `qingjian-learning` 的 `FrequencyLearner` 里这个词的用户频次与最后使用时间——用户词或学到的词，且「频次 ≥ 阈值、距上次使用 ≤ 7 天」；不足 2 字用更严的门槛。数据已在 learner 里，不新增存储；常数初值抄 WeType 的形状（7 天 / 1000 秒），频次阈值要按青简的计数尺度重定（WeType 的 0.1 / 1.0 是它的「热度」尺度，不能照搬）。不跨用户、不上传。

壳要上报的事件（只这三个，其余全在 Core）：
1. `note_candidates_shown(generation)`：候选窗这一版真的画出来了（Mac `render()` 之后、Win/Linux 发出 frame 后、iOS SwiftUI `onAppear`/`task(id: generation)`）。
2. `note_candidates_touched()`：翻页、方向键 / Tab 移动高亮、鼠标悬停改高亮。
3. 拿到 `LateOutcome` 后：`Unchanged` 不重画；其他重画当前页，**高亮保持在原下标**（不要像 WeType 那样清零——前 frozen 位没动，高亮在冻结区内就是同一个词）。

### 3.3 改哪些文件

- 新增 `crates/qingjian-core/src/engine/late/{mod.rs, policy.rs, source.rs, display.rs, outcome.rs, merge.rs, tests.rs}`。
- `crates/qingjian-core/src/engine/mod.rs`：Engine 加字段 `late_policy: LatePolicy`、`display: DisplayState`；`query()` 产出新列表时 `display.generation += 1`、清 `shown_at/touched`。`QueryResult` 带上 `generation`。
- `crates/qingjian-core/src/candidate/layout/candidate_layout.rs`：`set_cloud` 改由 `merge_late` 的结果驱动（保留分页 / Custom 定位逻辑），`capacity()` 的「本地至少占第一格」并进 protect0。
- `crates/qingjian-core/src/engine/prediction/policy.rs`：`slots` 语义改为「时限外最多插几条」，或并进 `LatePolicy`（二选一，倾向并进，`[predict]` 配置不变名）。
- `crates/qingjian-platform` `Config`：`[predict]` 下加 `late_grace_ms`、`late_frozen`（快捷键之外的可调常数，放配置符合「不写死」）。
- 壳：删 `apps/macos/src/host/cloud/mod.rs` 的 `cloud_slots_untouched` 与 `model/mod.rs:176` 起的整表替换；Windows `dispatch/rescore/mod.rs:181`、Linux `dispatch/rescore/mod.rs:160` 的判断；`cloud/crates/qingjian-cloud-bridge/src/session/cloud.rs` 的 `CLOUD_POSITION` 插位与 `session/rescore.rs` 的 `requery_local`，全换成调 Engine。每个壳只加上报两个事件。
- 文档：`docs/notes/crate-notes.md`（core 新常数）、`docs/user/` 里云联想那页（「候选窗出来一会儿之后到的云词排在第 4 位以后」）。

### 3.4 与青简约束的冲突与改法

- **平台层不排序**：现状 4 个壳各有一份守卫和插位（iOS 还把云词插到第 2 格），本规格把它们收回 Core；壳只报事件。
- **输入优先**：`merge_late` 是 O(页大小) 的纯函数，在 poll 时跑，不进按键路径；云结果再晚也不阻塞。
- **Core 平台无关**：时间用 `std::time::Instant`（Core 里 rescoring 已在用），由 Engine 方法内部取 `now`；单测用 `merge_late` 直接传 `now`。
- **一个候选一种辅助语言**：不涉及。
- **不跨用户学习**：保护判定只读本机 learner。
- WeType 的「服务端期望位置 exp_pos」「服务端强推首位 bit22」「服务端过滤本地词典」都依赖它自家云协议，青简的 OpenAI 兼容接口没有这些字段，不做。

---

## 4. 复刻步骤

1. 建 `engine/late/` 各文件与类型（签名见 3.2），`LatePolicy::default()` = {200, 3, 1, true}，注释写明「WeType 值，待重扫」。
2. 写 `merge_late` 纯函数 + `tests.rs`（下列用例先写、先红）。
3. Engine 接线：`query()` 递增 generation；加 `note_candidates_shown/touched`、`merge_prediction`、`merge_rescoring`、`current_candidates`。
4. `is_protected_top1`：从 `FrequencyLearner` 读频次与最后使用时间（没有最后使用时间就先只用「是用户词 / Custom」，并在 todo 里记一条补时间戳）。
5. `CandidateLayout` 改为消费合并结果；删除 `set_cloud` 里的去重（已在 merge_late 做）或保留为兜底。
6. 逐个壳替换：Mac → Windows → Linux → 素笺 iOS（bridge），每个只剩「报两个事件 + 按 LateOutcome 决定重画」。高亮下标保留。
7. `apps/cli` 加 `--late-sim`：回放输入日志时按记录的「显示 → 结果到达」间隔模拟，统计首选被换掉的次数。
8. 真机：快打对比误选（上屏后 1 秒内退格）次数，改前 / 改后各 200 句；扫 `grace_ms ∈ {100,200,300}`、`frozen ∈ {2,3}`。

单测清单（`late/tests.rs`；shown = 已显示的本地列表，t = 结果到达距 shown_at 的毫秒）：

| # | 输入 | 期望 |
|---|---|---|
| 1 | 云 `[X]`，shown `[a,b,c,d,e]`，t=50，未动，a 非保护 | `[a,X,b,c,d,e]`，Reranked（插在第 2 位，最多 1 条） |
| 2 | 云 `[X,Y]`，t=50 | 只插 X：`[a,X,b,…]`（max_in_grace=1，Y 丢） |
| 3 | 云 `[X,Y]`，t=500 | `[a,b,c,X,Y,d,e]`，AppendedAfterFrozen |
| 4 | 云 `[X]`，t=50 但 touched=true | 同 3：`[a,b,c,X,d,e]` |
| 5 | 云 `[c]`（与本地第 3 位同字），t=500 | 去重后无可插 → Empty，列表不变 |
| 6 | 云 `[X]`，shown_at=None（还没画出来） | 按时限内规则：`[a,X,b,…]` |
| 7 | shown `[Custom(2)=F, a, b, c]`（F 固定第 2 位），云 `[X]`，t=50 | X 不进 F 的位置：`[a,F,X,b,c]` |
| 8 | 重排 late=`[b,a,c,d]`，t=50，a 非保护 | `[b,a,c,d]`，Reranked |
| 9 | 重排 late=`[b,a,c,d]`，t=50，a 是保护词（用户 3 天前用过、频次够） | `[a,b,c,d]`（首位不动） |
| 10 | 重排 late=`[d,c,b,a,e]`，t=500 | 前 3 位冻结为 `[a,b,c]`，余下 {d,e} 按 late 次序 → `[a,b,c,d,e]`，与原来一样 → Unchanged |
| 11 | 重排 late=`[a,b,c,e,d]`，t=500 | `[a,b,c,e,d]`，AppendedAfterFrozen |
| 12 | 任意结果合并后第一页逐字不变 | Unchanged（壳不重画） |
| 13 | 单字首选「的」，2 小时前用过（超过单字的 1000 秒门槛），重排 late=`[得,的,…]`，t=50 | 不受保护：`[得,的,…]`，Reranked |
| 14 | 结果的 generation ≠ 当前 generation | 丢弃，列表不变 |

---

## 5. 未知

- `Merge` 传给 MergeImpl 的 `ctx_empty`：取自 `[this+0x20]+0x210` 的字符串（加锁读，0x100c0eb0c），为空时 IsProtectLocalFirst 的 `force` 为真（用户词一律保护）。没查 `[this+0x20]` 是什么对象（下一步：看 `CloudInputMerger::Impl` 构造里 +0x20 的来源，0x100c0c05c 附近，及谁写 +0x210）。
- `Impl+0x305 / +0x306` 两个布尔传进 mergeSort 栈参数（0x100c15dc4），本次读的路径里没看到使用；写入点没找到。
- `+0x20` 处 u16 标志的 bit1（「普通 Trie 词且无 bit1」时云词直接插在它前面）：猜是「整段完全匹配 / 非补全」，要去 `TrieMatchingCandidateID` 的构造看谁置位。
- GetCandidateFlag 的 bit22 是否就是服务端「可置顶」：要从云协议解析（`CloudSearchByInputV2` 的 response → CandidateFlag 赋值）确认。
- 超时插入路径不去重，是否有下游层（`MostLikelyFilter` 等）去重：查会话链里 CloudInputMerger 之上的 wrapper。
- 壳侧：用户移动高亮后云更新到来时是否有保护——`updateIterator` 闭包无条件清高亮；可能引擎侧有 `HighlightMoved` 之类的事件让 CloudPendingHolder 取消，未查（下一步：`CandidatesStore.highlightMoved.didset` 0x10014069c 的调用目标）。
- `isOutOfTime` 里用户画像 `stat(5)` 的确切含义（平均选词耗时 / 中位数 / 窗口长度），需看 +0x30 对象 vtable 0x358 → 0x30 的实现。
- `ReportCloudResultEmitType`（0x100c128fc）上报的字段没读，与本任务无关。
