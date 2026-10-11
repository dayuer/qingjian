# B4 上文（光标前文本）模型状态跨按键缓存 / B6 设备分档 —— 复刻规格

材料：WeType 2.2.3 macOS arm64（/tmp/wetype-rea/WeType.arm64，full.asm）。纯静态阅读。
约定：[确] = 直接看到的指令 / 符号 / 字符串（附地址）；[推] = 多条证据推出；[猜] = 名字或形状推测。
WeType 的常数全部是「WeType 的值，青简要自己重扫」。

另外说明一个辅助事实（两节都会用到）：`wxime::data::HyperParams::XxxParams()` 返回的是 **capnp struct reader**（0x30 字节：+0x10 = data 段指针，+0x20 = dataSize（单位 bit））[推]（`LtrModelParams::HighDeviceLtrModelIsValid` 直接调 `capnp::_::PointerReader::getPointerType` @0x10073526c）。
所以反汇编里「`ldr w8,[sp,#0x20]; cmp w8,#0x40; b.hs` 才读 `data+4`」的意思是「字段存在才读，否则用缺省值」。下文「参数 X（缺省 N）」都按这个解读。

---

## B4 上文缓存

### 1. WeType 怎么做的

WeType 有两套独立的上文缓存：**词级 NNLM（GPT-2 结构，`nlrgpt2_nnlm.bin`）的 StreamingNNLMModel** 与 **E2E decoder-only 模型的 DecoderOnlyNNModel**。两者都是「文本变化事件 → 异步把上文的 KV 算好存一份 → 查询时按字符串比较命中」。

#### 1.1 触发：光标附近文本变了就异步预填

- `AutoInferTextBeforeCursor`（session 包装层，ctor @0x1006f61e0）在构造时向 session 的一个 `lsignal::signal<void()>`（文本改变信号，session+0xb28 → +0x10）挂回调 [确 @0x1006f6310 `create_connection`]。
  挂不挂由两个条件决定 [确 @0x1006f628c–0x1006f62e4]：
  ```c
  p = HyperParams.NNLMRankerParams();
  if (!p.present || !p.data[0] /*enable bit*/) return;           // 不挂
  int dev_perf = atomic_load(env->device_and_svr_stat.level);     // env+0x278，见 B6
  int max_lvl  = p.dataSize >= 64 ? p.data_i32[1] : 0;            // data+4，缺省 0
  if (dev_perf > max_lvl) return;                                 // 设备档位不够就不预填
  connect(text_changed_signal, lambda);
  ```
- 回调 lambda（@0x1006f66b4）[确]：
  ```c
  lock(ctx->mutex /*+0x1b8*/); optional<Reason> r = ctx->change_reason /*+0x464, has=+0x468*/; unlock;
  if (r && r == 99) return;
  if (r && (r == 5 || r == 6)) return;
  if (r && r == 9) return;
  inner_session->SetOption(SessionOptions(16), false);   // vtable+0x48，16 = “推理上文”
  ```
  `PinyinV2Session::SetOption(16)` → `Searcher::InferTextBeforeByNNLM(bool)`（vtable+0x68 @0x100a38cc8）→ `SentenceSearcherV3::InferTextBeforeByNNLM` @0x100a096e4 → `AsyncNNLMRanker::InferTextBefore(bool sync)` @0x100c6b030：
  `sync` 为真就拿锁当场跑 `NNLMRanker::InferTextBefore()`，否则 `aligned_alloc` 一个任务交 `async::threadpool_scheduler::schedule`（带 CancellationToken，`RunIfNotCancel` @0x100c6cddc）[确]。
- E2E 一侧完全同构：`AutoPrefillContextForNN`（ctor @0x1006f677c）挂同一类信号，lambda @0x1006f6d50 跳过同样的 reason（99、5、6、9），否则 `Prefill(false)`；`SetOption(16)` 时 `Prefill(true)` [确 @0x1006f6aec]。

reason 的枚举名没找到（`TextAroundCursorChangeReason` 只有类型名），见「未知」。

#### 1.2 NNLM 一侧：`NNLMRanker::InferTextBefore()` @0x100c5c190 [确]

```c
void NNLMRanker::InferTextBefore() {
  if (!model_ /*+0x28 StreamingNNLMModel*/ || !model_->ModelIsValid()) return;
  string tb = lock_copy(ctx_->text_before /*TextAroundCursorContext+0x210*/);
  if (tb.empty()) { model_->ClearCache(); return; }
  Entry* e = ctx_->nnlm_text_before_entry /*ctx+0x188 shared_ptr*/;
  if (e && e->text == tb) return;                      // 命中：逐字节比较原始上文串
  string norm = model_->NormTextBefore(tb);            // 见下
  if (norm.empty()) return;
  int keep = Assoc.dataSize > 0x9f ? max_or(Assoc.data_i32[4] - 1, /*≤0 时*/ 9) : 9;   // HyperParams::AssociateParams() data+0x10，缺省 9
  vector<string_view> runes = SplitUTF8Runes(norm);
  if (runes.size() > keep) norm = concat(runes[size-keep .. size]);              // 只留最后 keep 个字
  model_->InferText({norm}, /*flag*/true, /*pinyin*/"", nullptr);               // 算这串的状态
  optional<NNLMState> st = model_->GetNNLMState(norm);
  if (st) {
    ctx_->nnlm_text_before_entry = make_shared<Entry>();   // 0x70 字节控制块+对象
    entry.text = tb;                 // 键 = 原始（未截断）上文
    entry.kv   = st->kv;             // vector<float>
    entry.logp = st->next_logprobs;  // vector<float>
  }
}
```
`NormTextBefore` @0x100ea7d60 [确]：把上文拆成 UTF-8 字，从**最后一个字往前**逐个查 `NNLmWordMapDict::LookUpByWord`，遇到第一个词表外的字就停，返回这段「词表内后缀」（标点、英文、emoji 处截断）[推：拷贝的是末尾那段字节]。

#### 1.3 NNLM 模型对象 `StreamingNNLMModel`（ctor @0x100ea5e0c）的数据结构 [确/推]

| 偏移 | 内容 |
|---|---|
| +0x18 | `shared_ptr<NNLmModel>` |
| +0x28 | `lm_type`（int；3 = `NNLmType::GPT2_WITH_PY_XNET`，字串 @0x102cf302c）|
| +0x38 | 指向模型自带的初始状态（`<s>` 状态）|
| +0x40 | `per_token_kv_len` = 初始状态 `kv.size()`（一个 token 的 KV 浮点数）|
| +0x44 / +0x45 | `has_text_before_cache` / `text_before_flag` |
| +0x48 | `unordered_map<string, NNLMState>`：**前缀文本 → 状态**（键 `"<s>"` 是根）|
| +0x70 | `utils::LRUCache`（容量 = 构造参数）|
| +0x80 | valid |
| +0x88 | 互斥量（所有入口用 `try_lock`，拿不到就记 `!lk.owns_lock()` 直接返回 0）|
| +0xc8 | `vector<ModelInput>`（每条 0x68 字节）|

`NNLMState`（@ map 节点 +0x28）[推]：`vector<float> kv`（该前缀**全部 token**的 KV 展平，长度 = token 数 × per_token_kv_len）、`vector<float> next_logp`（对下一个词表项的 log 概率）、`float score`（累计 log 概率）、`string`。
证据：`SetTextBeforeCache` 要求 `a.size() % per_token_kv_len == 0` 并返回 `a.size()/per_token_kv_len - flag`（token 数）；`LookUpLMScoreInCache` 用 `parent.score + parent.next_logp[word_id]` 算分（@0x100ea7b60–0x100ea7b68）。

#### 1.4 把上文装进模型：`NNLMRanker::SetTextBeforeCache()` @0x100c5be38 [确]

在 `NNLMRanker::ReRankPaths` 开头调用；每个 ranker 实例只做一次（+0x48 = done，+0x49 = enabled）：
```c
void NNLMRanker::SetTextBeforeCache() {
  if (!enabled_ || done_) return;
  if (!model_) return;
  model_->ClearCache();                                      // 清前缀表，"<s>" 复位为模型初始状态，LRU 清空
  string tb = lock_copy(ctx_->text_before);
  if (model_->NormTextBefore(tb).empty()) { enabled_ = false; done_ = true; return; }
  Entry* e = ctx_->nnlm_text_before_entry;
  if (!e || e->text != lock_copy(ctx_->text_before)) {       // 后台还没算好 / 上文又变了
    enabled_ = false;                                        // 本轮不用上文（不等）
  } else {
    n_tokens_ = model_->SetTextBeforeCache(copy(e->kv), copy(e->logp), e->flag);
  }
  done_ = true;
}
```
`StreamingNNLMModel::SetTextBeforeCache(kv, logp, flag)` @0x100ea6284 [确]：`try_lock` 失败 → 日志 `!lk.owns_lock()` 返回 0；`kv` 为空或不能被 `per_token_kv_len` 整除、或表里没有 `"<s>"` → 日志 `SetTextBeforeCache failed`（@0x102cf2fee）返回 0；否则 **把 `"<s>"` 这一项的 kv / next_logp 用 move 换成上文的**，置 +0x44=1、+0x45=flag，返回 `kv.size()/per_token_kv_len - flag`。
也就是说：**上文状态挂在前缀树的根上**，之后所有候选前缀都从它往下长，候选打分不再单独带上文。

#### 1.5 候选打分时的前缀复用（同一轮内 & 跨按键）

- `StreamingNNLMModel::InferText(texts, flag, pinyin, timer)` @0x100ea6b24 [确]：对每条候选文本，从完整文本往前逐个截掉最后一个字，查前缀表，收集「表里没有的前缀」（`absl::flat_hash_set` 去重），直到碰到已缓存的前缀（最长前缀命中）；只为缺的那几段组 `ModelInput`（`PrepareModelInput` @0x100ea81f0），然后 `BatchInferWithCache`。
- `BatchInferWithCache(timer)` @0x100ea8c3c [确]：按「(过去长度, 新增长度)」把输入分组（`map<int, map<int, vector<int>>>`），同形状的拼 batch 一次前向；每组前看计时器 `elapsed_ms = (now-start)/1e6`，**超过预算就停**并记 `NN model batch infer too slow, break`（@0x102cf3217）；算出的每个新前缀状态 `LRUCache::Put(key, weight = kv.size()/per_token_kv_len)`（权重 = token 数）再写进前缀表。
- 回到 `InferText`：`LRUCache::GetAndClearPoped()` 拿到被挤出的键，**除 `"<s>"` 外**逐个从前缀表删掉（@0x100ea7444–0x100ea7484，比较 `len==3 && "<s>"`）。
- `LookUpLMScoreInCache(text, pinyin)` @0x100ea77b4 [确]：键 = 文本（GPT2_WITH_PY 时与拼音合成）。表里有 → `LRU.Get` 触一下，返回 `state.score`；没有 → 取「去掉最后一个词」的前缀（只有一个词时用 `"<s>"`），查 `NNLmWordMapDict::LookUpByWord(最后一个词)` 拿 id，返回 `parent.score + parent.next_logp[id]`；都没有返回 0。
- `LRUCache::Impl::Put` @0x100f47c20 [确]：容量是**权重和**（token 数之和）上限，超了从链表尾逐个淘汰，直到 `total ≤ capacity`；淘汰的键进 popped 列表。
- 容量：`NNLMRanker` ctor @0x100c5ba54 [确]：`NNLMRankerParams.dataSize >= 0xa0 && data_u32[4] != 0 ? data_u32[4] : 100`（data+0x10，缺省 **100 个 token**），传给 `StreamingNNLMModel(model, capacity)`。`"<s>"`（即上文）不计入、不被淘汰。

所以：**跨按键时**，同一段上文下 `"<s>"` 根状态不变（ctx 里那份 Entry 不变就不重算），候选前缀的状态在 LRU 里活着，下一键的候选多半是上一键候选加一个字 → 只算最后一个字。

#### 1.6 E2E 一侧（DecoderOnlyNNModel）

- `IE2EModel::PrefillContext(string, bool)`（DecoderOnly 版 @0x100eee354）[确]：模型有效时 `GetLastLineText(text)`（只取最后一行）→ `get_valid_text_before_suffix(text, vocab)`（同 NormTextBefore，词表内后缀）→ 推一个 `Job` 进 `UnboundedQueue<DecoderOnlyNNModel::Job>`（后台工作线程消费）。长度上限是 `AutoPrefillContextForNN::Prefill` 里 `ctx->TextBeforeCursor(model->vfunc@0x30())` 给的 [确 @0x1006f6e24–0x1006f6e40]，具体数没读到。
- 工作线程里的 `DecoderOnlyNNModel::PrefillContext(const string&)` @0x100eee7d8 [确]：
  ```c
  lock(mu_ /*+0x2c8*/);
  if (text == saved_text_ /*+0x1b8*/) return;                       // 完全相等 → 什么都不做
  runes_new = SplitUTF8Runes(text); runes_old = SplitUTF8Runes(saved_text_);
  L = common_prefix_len(runes_new, runes_old);                       // 按字比较的公共前缀
  n_new = runes_new.size() - L + 1;                                  // 只为新增部分建 token 张量
  CopySavedCacheToTensor(k, v, L);                                   // 保留旧 KV 的前 L 个位置
  ok = InferenceV2(...新增 token...);                                // 增量前向，写回 saved KV
  if (!ok) { log("Decoder-only NN Model PrefillContext failed."); SearchCache::Reset(); PrefillStartToken(); }
  saved_text_ = text;
  ```
- `CopySavedCacheToTensor(src, dst, L)` @0x100eed378 [确]：`saved_len = numel(src) / (n_layer(+0x6c) × n_head(+0x84) × head_dim(+0x88) × 2)`；`saved_len < L` 抛错；`L == saved_len`（或 L<0）**不拷贝，dst 直接共享 src**；否则新建张量，按 (层×K/V×头) 每行 `memcpy(L × head_dim × 4 字节)`，即**把 KV 截到前 L 个位置**。
- 解码时：`BeamSearchDecodeText` @0x100eef4f0 先 `PreProcessInput` @0x100ef15b8：同样规范化上文、发 prefill Job，然后**等 prefill 完成**：`deadline = now + (输入条数 < 2 ? 15 : 32) ms` 的条件变量等待 [确 @0x100ef1920–0x100ef196c]；超时 → `Can not get lock, prefill too slow`（@0x102cf54b6）放弃 E2E；拿到后若 `saved_text_ != 当前上文` → `Decoder-only NN Model text_before not matched.`（@0x102cf54d9）放弃。
  beam 每一步的循环（0x100eef85c ↔ 0x100ef11d0）里两处 `CopySavedCacheToTensor` [确 @0x100eef8ec、@0x100ef0448]：每步把保存的上文 KV 交给推理张量，通常 L == saved_len → 共享不拷贝 [推]。
- 首 token：`PrefillStartToken()` @0x100eec448 在 `Init` 与失败兜底时调，日志 `Decoder-only NN Model PrefillStartToken failed.`。

#### 1.7 日志与开关汇总

| 字符串 / 参数 | 位置 | 含义 |
|---|---|---|
| `set_text_before_cache` | str.txt | 计时段名 [推] |
| `SetTextBeforeCache failed` | @0x102cf2fee | 上文 KV 尺寸不对 |
| `!lk.owns_lock()` | @0x102cefb3a | try_lock 失败 |
| `NN model batch infer too slow, break` | @0x102cf3217 | 批量前向超预算 |
| `InferTextBefore WIP` | async_nnlm_ranker.cpp | 异步任务还在跑 [推] |
| `Can not get lock, prefill too slow` / `... text_before not matched.` | @0x102cf54b6 / 54d9 | E2E 不等上文 |
| NNLMRankerParams：data bit0 enable；data+4 允许的最差设备档（缺省 0）；data+0xc 参与重排的最长路径（缺省 8）；data+0x10 LRU 容量（缺省 100） | 0x1007333bc 的调用点 | |
| AssociateParams data+0x10：上文保留字数 + 1（缺省 9 个字） | @0x100c5c2dc | |

### 2. 证据表

| 结论 | 置信度 | 地址 / 字符串 |
|---|---|---|
| 上文推理由「光标附近文本变化」信号触发，异步线程池执行 | 确 | 0x1006f6310、0x1006f66b4、0x100c6b030 |
| reason ∈ {99, 5, 6, 9} 时不触发 | 确（语义未知）| 0x1006f66f4、0x1006f6724、0x1006f6740 |
| 上文缓存键 = 原始上文串逐字节比较（不是哈希、不是 token 序列）| 确 | 0x100c5c23c–0x100c5c290、0x100c5bf68–0x100c5bfd0 |
| 上文缓存放在会话的 TextAroundCursorContext（+0x188），每会话一份 | 确 | 0x100c5c5e4 |
| 规范化：词表内后缀 + 最后 9 个字（参数可调）| 确 | 0x100ea7d60、0x100c5c2e4–0x100c5c338 |
| 缓存内容 = 全部 token 的 KV 展平 + 下一词 log 概率 | 推 | 0x100ea62c4–0x100ea62e4、0x100ea7b60 |
| 上文 KV 挂成前缀树根 `"<s>"`，候选从它往下长 | 确 | 0x100ea64a0–0x100ea65c0 |
| 候选前缀走 LRU，容量按 token 数计、缺省 100、`"<s>"` 不淘汰 | 确 | 0x100c5bc8c、0x100f47cc8、0x100ea7444 |
| 批量前向有时间预算，超了中断 | 确 | 0x100ea8fb4–0x100ea8fd8、@0x102cf3217 |
| E2E 上文：完全相等跳过；否则按字公共前缀 L 保留旧 KV、只算新增 | 确 | 0x100eee834–0x100eee958、0x100eeebd8 |
| CopySavedCacheToTensor：L==已存长度时共享不拷贝，否则按行 memcpy 截断 | 确 | 0x100eed428–0x100eed4fc |
| 解码等 prefill 最多 15 ms（单输入）/ 32 ms | 确 | 0x100ef1920–0x100ef196c |
| 设备档位不够时不挂上文推理 | 确 | 0x1006f62a4–0x1006f62e4 |

### 3. 青简落点

#### 3.1 现状（读 crates/qingjian-neural 与 Core 的结论）

- `CharScorer`（scorer.rs）只有**一格** KV 缓存：`cache: Mutex<Option<(Vec<u32>, PrefixCache)>>`。
  - `score(context, texts)`（知微，Mac 的词级重排在用）：键 = `<eos> + 前文` 去掉最后一个 token、再从左截到 `limit - width` 的 **token 序列**，完全相等才命中，否则 `prefix_cache(head)` **从头整段重算**。
  - `score_p2c(keys, texts)`（通变）：键 = `<eos> + 拼音`。拼音每键多一个字母 → 每次都不相等 → 每次整段重算。**跨按键没有共享拼音前缀 KV。**
  - `P2c::convert`（p2c.rs）：注释说「每敲一键只需把新字母接上去」，实际每次 `prefix_cache(&prefix[..split])` 从头建，不用 `CharScorer.cache`；同一次调用里 beam 之间用 `PrefixCache::select` 共享。
  - `continue_text`（continuation.rs）每次自己 `prefix_cache`，不走缓存。
- 上文怎么拼进输入：Core `Engine::rescoring_context()`（rescoring/mod.rs）取壳给的光标前文（`set_rescoring_context`）或本会话上屏历史，`take_last_chars(…, RESCORE_CONTEXT_CHARS = 64)`；`CharScorer::score` 再拼成 `<eos> + 前文 + 候选`。P2C 不看上文。
- Core 的分数表 `NeuralCache`（rescoring/cache.rs）= WeType 的 `LookUpLMScoreInCache` 那一层：`(context, keys)` 一变整张表清掉。**词级重排（word_rescore）把 `keys = composition.scope()` 也当条件**，每键都变 → 每键清表，哪怕知微根本不看按键。
- iOS 键盘（cloud/ios/Keyboard + cloud/crates/qingjian-cloud-bridge）只加载通变 8 位（`Engine.swift loadModel` → `qj_load_model(…, p2c=true)`），没有知微、不看上文；会话层 `refresh_candidates` 每键 `set_rescoring_context(before)`。

结论：WeType 的「上文 KV 跨按键复用」在青简里对应三处欠账：①同一上文下其实已经命中（单格缓存），但**上文变了（上屏、移光标）第一次重排要在关键路径上整段重算**；②上文是在旧上文后**追加**时（刚上屏）也整段重算，没有公共前缀增量；③通变的拼音前缀每键整段重算（iOS 唯一的模型就是它）。另有一处便宜的：知微的分数表每键被清。

#### 3.2 要改的文件与新增类型

1. **新增 `crates/qingjian-neural/src/context_cache.rs`**（一个类型一个文件）：
   ```rust
   //! 一段前缀（上文或拼音）的 K / V 跨调用复用：完全相同直接用，新序列以旧序列开头就只算新增的 token，
   //! 公共前缀够长就截到公共前缀再接着算；都不行才整段重算。对应 WeType DecoderOnlyNNModel::PrefillContext。
   pub(crate) struct ContextCache {
       ids: Vec<u32>,
       cache: PrefixCache,
   }
   impl ContextCache {
       pub(crate) fn empty() -> Self;
       /// 让缓存对应 `ids`，返回复用掉的 token 数（统计用）。
       pub(crate) fn sync(&mut self, model: &CharLm, ids: &[u32]) -> candle_core::Result<usize>;
       pub(crate) fn cache(&self) -> &PrefixCache;
       pub(crate) fn clear(&mut self);
       pub(crate) fn bytes(&self, cfg: &ModelConfig) -> usize; // 2 × n_layer × n_embd × len × dtype 字节
   }
   ```
   `sync` 的规则（照 WeType E2E）：
   ```text
   if ids == self.ids                         → 命中，返回 len
   L = common_prefix_len(ids, self.ids)
   if L == 0 || L < MIN_REUSE_TOKENS          → self.cache = model.prefix_cache(ids)
   else {
       if L < self.ids.len() → self.cache = self.cache.truncate(L)      // CopySavedCacheToTensor 的截断
       if ids.len() > L      → (_, self.cache) = model.step(&self.cache, ids[L..] as [1, n])
   }
   self.ids = ids.to_vec()
   ```
   常数 `MIN_REUSE_TOKENS = 1`（初值；截断 + 增量总比整段重算省）。
2. **`crates/qingjian-neural/src/model/mod.rs`**：给 `PrefixCache` 加 `pub fn truncate(&self, len: usize) -> Result<Self>`（每层 `narrow(2, 0, len)`，形状 `[1, h, p, d]`）。`step` 现有实现已能「接着缓存推进一段」，`idx` 用 `[1, n]` 即可。
3. **`crates/qingjian-neural/src/scorer.rs`**：
   - `cache` 字段换成 `Mutex<ContextCache>`；`score` / `score_p2c` 里的「不相等就 `prefix_cache`」换成 `guard.sync(&self.model, head)`。
   - P2C 的 head = `<eos> + 拼音`：打字时是追加、退格时是截断，`sync` 天然覆盖（这就是「拼音前缀 KV 跨按键共享」）。
   - `continue_text`：把前文部分（`ids[..split]`）也走同一个 `ContextCache`（同一 `CharScorer` 的知微实例；它和 `score` 的 head 都是 `<eos>+前文` 去尾，常常相同）。
   - 新增 `pub fn prefill(&self, context: &str) -> Result<(), NeuralError>`：按 `score` 的同一套规则算 head（注意 head 的左截依赖 `width`，预填时用 `width = 1` 会和真打分时的截法不同；见 3.3 第 2 条）并 `sync`。
4. **`crates/qingjian-core/src/sentence/scorer.rs`**（`SentenceScorer` trait）：加两个有缺省实现的方法，Core 仍不知道模型细节：
   ```rust
   /// 上文变了，后台先把上文的状态算好（WeType AutoInferTextBeforeCursor）。缺省什么都不做。
   fn prefill(&self, _context: &str) {}
   /// 分数是否依赖用户按键（字级模型不依赖：按键变了分数表不必清）。缺省 true。
   fn uses_keys(&self) -> bool { true }
   ```
   `qingjian-neural/src/core_scorer.rs`：`CharScorer` 实现 `prefill`（调 `CharScorer::prefill`，出错 `tracing::warn` 后吞掉）、`uses_keys() -> false`；`P2cScorer` 保持缺省。
5. **`crates/qingjian-core/src/engine/rescoring/worker.rs`**：`Job` 里加一种「只预填」的任务（`texts` 空、`generate` None、`prefill: Option<String>`），线程里先调 `scorer.prefill(&context)`；预填任务不产生 `Scored`。排队仍「只算最新一条」——上文连续变化时只预填最后一次（等价于 WeType 的 CancellationToken）。
6. **`crates/qingjian-core/src/engine/rescoring/mod.rs`**：`set_rescoring_context` 里比较新旧上文（截过 64 字之后的串），变了且接了异步打分器 → 给两个 worker（整句、词级）各送一条预填任务。不在私密状态（`is_private()`）时才送。
   词级那条：`word_rescore/mod.rs` 里 `cache.ensure_condition(&context, &keys)` 改成 `scorer.uses_keys()` 为假时 `keys` 传空串——分数表跨按键保留（WeType `LookUpLMScoreInCache` 的那一层）。`MAX_ENTRIES = 1024` 照旧兜底。
7. **失效**：`set_private(true)`、换学习分区（set_learning_scope）、换输入框（壳 `setContext` / bridge 的 context 更新）时：Core 发一条预填空串的任务（`prefill("")` 在 `CharScorer` 里 = `ContextCache::clear()`），保证私密输入框的上文 KV 不在内存里留着。上屏与移光标走第 6 条的「上文变了」。
8. **iOS 预算**：bridge `session/model.rs` 不用改结构；内存兜底已有（`KeyboardModel.guardMemoryPressure` 余量 < 4MB 卸模型）。缓存只有一格，大小 = `2 × n_layer × n_embd × len × 4B`（f32 CPU）：8 层 / 448 宽、65 个 token ≈ 1.8 MB；通变的拼音头一般 < 40 token ≈ 1.1 MB。加常数 `CONTEXT_CACHE_MAX_BYTES`（初值 4 MB，放 scorer.rs）：`bytes()` 超了就不留缓存、每次现算（任务原文的「超了就不缓存」）。

#### 3.3 与青简约束的冲突与改法

1. **「输出与不缓存时逐位一致」做不到严格逐位**：增量 `step` 与整段前向的矩阵乘分块不同，浮点求和顺序不同；8 位权重路径（Q8_0 内核）同理。WeType 也只保证同一份 KV 的复用。改法：单测分两档——「同一序列命中缓存两次」逐位相等；「增量 vs 整段」差 < 1e-4（f32）/ 现有测试用的 0.08（P2C 8 位，与 `p2c_cached_scores_match_full_forward` 同口径），并且排序名次一致。
2. **head 的左截依赖候选长度**（`score` 里 `limit - width`）：前文 64 字 + 长候选时，同一上文换一批候选 head 就变，缓存失效。按现在的模型上下文与 64 字上限一般碰不到，但预填用的截法必须和打分一致，否则预填白算。改法：把左截改成只看前文（`head` 固定截到 `limit - MAX_TAIL`，`MAX_TAIL` 取打分时允许的最长候选，候选超了才额外截），预填与打分共用这个函数。
3. **滑动窗口无法前缀复用**：前文已满 64 字后，每上屏一次窗口左端右移，位置嵌入是绝对的，整段 KV 都变（WeType 也一样：NNLM 只留 9 个字，靠异步预填把重算移出关键路径）。所以本任务**主要收益来自「预填移出关键路径」+「追加时增量」+「通变拼音前缀增量」**，不来自滑窗。可选增强（要 CLI 评测过门槛才合）：窗口左端按 16 字一跳地挪（前文 48–64 字之间保持左端不动），换更多增量命中。
4. **Core 平台无关**：上面所有逻辑都在 Core / neural，壳只负责照旧 `set_rescoring_context`；bridge 不需要新 FFI。
5. **输入优先**：预填只在后台线程；关键路径上的打分如果预填还没完成，现状就是「整段算」——与 WeType 不同，WeType 是「不等就不用上文」。青简异步重排本来就在停键后，保持现状（等锁）即可；同步打分（CLI）不受影响。
6. **一个类型一个文件**：`ContextCache` 独立文件；`PrefixCache::truncate` 是给已有类型加方法，留在 model/mod.rs。

### 4. 复刻步骤

1. `model/mod.rs`：`PrefixCache::truncate(len)`；单测：`prefix_cache(ids).truncate(k)` 后接 `step(ids[k..])` 的 log 概率与 `prefix_cache(ids)` 后 `step` 的一致（差 < 1e-4）。
2. 新增 `context_cache.rs`（`ContextCache::{empty, sync, cache, clear, bytes}`），`lib.rs` 加 `mod context_cache;`。
3. `scorer.rs`：`cache` 换类型；`score`、`score_p2c`、`continue_text` 走 `sync`；抽出 `fn context_head(&self, context: &str) -> Vec<u32>`（固定左截，见 3.3-2）；新增 `prefill`；加 `CONTEXT_CACHE_MAX_BYTES`。
4. `core_scorer.rs`：实现 `prefill`、`uses_keys`。
5. Core：trait 加缺省方法；worker 支持预填任务；`set_rescoring_context` 比较并下发；`set_private` / 学习分区切换发清空；词级分数表按 `uses_keys` 决定条件。
6. 统计：`RescoreStats` 加 `reused_tokens`（`sync` 的返回值），bridge 的 `rescore_stats` 字符串多一列 `reused=`，真机日志据此看命中率。
7. 同步 `docs/notes/crate-notes.md`（qingjian-neural 一节：缓存规则与两个常数）。用户感知不到，不改 `docs/user/`。
8. 验收：`cargo test -p qingjian-neural -p qingjian-core`；CLI `--eval-text` 冻结集精度不变；真机（Release）同一上文连续 10 键看 `forward_ms` 与 `reused=`；`PerfRecorder` 的 footprint 峰值不超过现状 + 4 MB。

单测清单（输入 → 期望）：

| 测试 | 输入 | 期望 |
|---|---|---|
| sync 命中 | `sync(A)` 两次 | 第二次返回 `A.len()`，`cache` 指针 / 内容不变；`score` 两次逐位相等 |
| sync 追加 | `sync([eos,我,今])` 再 `sync([eos,我,今,天])` | 返回 3；之后 `score("我今天", …)` 与新建 scorer 的结果差 < 1e-4 |
| sync 退格 | `sync([eos,n,i,h])` 再 `sync([eos,n,i])` | 返回 3（截断，零前向）；P2C 分与整段算差 < 0.08 |
| sync 分叉 | `[eos,a,b,c]` → `[eos,a,x]` | 返回 2；结果与整段一致 |
| sync 无公共前缀 | `[eos,a]` → `[eos2,…]`（构造） | 返回 0，整段重算 |
| 超预算不留 | 把 `CONTEXT_CACHE_MAX_BYTES` 设成 1 | 每次都现算，结果不变 |
| P2C 逐键 | 依次 `score_p2c("j",…)`, `("ji",…)` … `("jiekou",…)` | 与每次新建 scorer 的分差 < 0.08；第 2 次起 `reused = 前一次长度` |
| 预填 | `prefill("我今天想去")` 后 `score("我今天想去", ["上海"])` | 与不预填同分；打分时 `reused == head.len()` |
| 私密清空 | `set_private(true)` 后 | 词级 / 整句 scorer 收到 `prefill("")`，`ContextCache` 为空 |
| 词级分数表跨键 | 同一上文，scope 从 `jin` 变 `jint` | `word_cache` 不清（`uses_keys=false`）；P2C 的表照旧清 |

### 5. 未知

- `TextAroundCursorChangeReason` 的 5 / 6 / 9 / 99 各是什么（猜：99 = 引擎自己上屏回写、5/6 = 选区或删除类、9 = 失焦）。下一步：`TextAroundCursorContext::SetTextAroundCursor(…, reason)` @0x1006da974 的调用者，看各调用点传的常数；或 Swift 侧 `wxime_text_around_cursor_change_reason` 的取值。
- E2E 上文长度上限（`IE2EModel` vtable+0x30 的返回值）没读；下一步找 `DecoderOnlyNNModel` 的 vtable，看 +0x30 槽的函数体。
- `SetTextBeforeCache` 的 `flag`（Entry+0x48）在 `InferTextBefore` 里没被写，恒为 0 [推]；它的用意不明。
- `PrefillContext` 里 `n_new = runes_new - L + 1` 的 +1 是重喂最后一个公共字还是起始符，没细读 `$_8` @0x100eef35c。
- `BatchInferWithCache` 的时间预算 d8 从哪来（Timer 参数）没追。

---

## B6 设备分档

### 1. WeType 怎么做的

WeType 的分档是**两层**：静态的「设备性能档」（启动时由壳给，跨会话不变）+ 动态的「本会话是否低性能」（按本会话实测最慢耗时，AB 开关控制）。

#### 1.1 静态档：`DeviceAndSvrStat`

- 单例 `wxime::utils::DeviceAndSvrStat::Instance()` @0x100f60fc8，全局指针 `gDeviceAndSvrStat_` @0x1033ec0e8 [确]。
- 由 `FuncEntries` 在初始化时（`__GLOBAL__sub_I_device_and_svr_stat.cpp` @0x100f613a4 注册，优先级 9）调 lambda `$_0` @0x100f610c0 建立 [确]：从初始化参数字典取 `_wxime_config*`，拷字段：
  ```c
  stat->level     = cfg->i32_at_0x38;     // 设备性能档（C 侧类型 _wxime_device_performance_）
  stat->cpu_score = cfg->f32_at_0xb8;     // wxime_wevis_device_score_info.cpu  (缺省 -1.0)
  stat->gpu_score = cfg->f32_at_0xbc;     // wxime_wevis_device_score_info.gpu  (缺省 -1.0)
  stat->x         = cfg->i32_at_0x50;     // 用途见下（LTR 选模型）
  stat->name      = cfg->str_at_0x18;
  ```
  `_wxime_config` 的布局来自 ObjC 类型编码（str.txt 第 9438 行）：`^? B * * * B B ^? i(0x38) B B * I i i(0x50) … {wxime_wevis_device_score_info=ff}(0xb8)`。
- 档位取值 [推]：与 protobuf 枚举 `ABTestDevicePerformance` 一致——`UltraHigh = -5, High = 0, Standard = 1, Low = 2, Invalid = 100`（枚举名与值读自 @0x102f0d727 / @0x102f0d74c）；**数越大越弱**。
  证据：模型 Init 的判据是 `level > 模型允许的最差档 → 失败`（日志 `E2E Model Init failed, device perf: ` @0x102cf38d2、`Decoder-only NN Model Init failed, device perf: ` @0x102cf518c）；Mac 壳里 `-[KBABTestMgr deviceInfo]` @0x10002f070 写 `devicePerformance = +[Utils isAppleSilicon] ? -5 : 0` [确 @0x10002f098–0x10002f0ac]。
  另有服务端测评：`GetDeviceBenchMarkRequest{device: NSString}` → `GetDeviceBenchMarkResponse{deviceInfo: DeviceInfo{device, value, deviceType, modelLevel, name}}`（ObjC 属性表 @0x1032e6708 / 0x1032e67b0 / 0x1032e6620），以及上报用的 `DictAbTestDeviceInfo{model, brand, devicePerformance, systemVersion}`、`E2EDevicePerformance{None,Low,Medium,High,Super}`。即：**机型 → 服务端查表给分 / 档**，Mac 上直接按 Apple Silicon 判 [推]。
- 判定时机与缓存：进程初始化时一次；每个 `EnvContext`（会话环境）构造时 `ldar` 拷进 env+0x278..0x284（level / cpu / gpu / x），之后会话内只读 [确 @0x1006ec79c–0x1006ec7ec]。

#### 1.2 各档开关了哪些模型 [确]

| 位置 | 判据（level = 设备档）| 效果 |
|---|---|---|
| `E2ENNModel::Init` @0x100eb724c | `level > model.max_level(+0x130)` | E2E（1 亿参数）不初始化 |
| `DecoderOnlyNNModel::Init` @0x100eeb8a0 | `level > model.max_level(+0xdc)` | decoder-only E2E 不初始化 |
| `AppendDecoderOnlyGPUNNResource` lambda @0x100be33f4 | 只有 `level < 1`（High / UltraHigh）| GPU 版 E2E 才加载 |
| `AppendDecoderOnlyNNResource` lambda @0x100be2a74 | `level < 1` 走一支；否则模型类型 ∈ {1,7} 时再拿 `cpu_score` 与 `E2EModelParams` 比 | CPU 版 E2E 按 CPU 分决定 |
| `ReloadEnglishE2EResource` @0x100be24d8 | `level > 1`（Low）| 英文 E2E 不加载 |
| `AddDictToResources` @0x100bd5200 | `level < 1`，或 `level == 1 && cfg.x(0x50) == 1` | 用「高端机 LTR 模型」（`LtrModelParams::HighDeviceLtrModelIsValid` @0x10073526c 要求高端机模型与特征表都在）|
| `NNLmModel::UpdateValidIfNeed` @0x100eac4d0 | `level > NNLMRankerParams.max_level(data+4, 缺省 0)` 或 `level` 不满足模型自带的档 | 词级 NNLM 重排失效 |
| `AutoInferTextBeforeCursor` ctor @0x1006f62a4 | 同上 `level > max_level` | 不做上文预填（B4）|
| `MixRank::Complete` → `LightGBMPredictor::HighDeviceLtrValid` @0x100ea43d4 | 读上面选好的标志 | 排序用哪套 LTR |

`SetEnable(bool)`（E2ENNModel @0x100ec38e4 / DecoderOnly @0x100eed370 / DecoderOnlyGPU @0x100efd650）只是写一个字节（+0x9c / +0x94 / +0x40），经虚表调用，调用者没定位 [推：来自会话选项 `_wxime_client_enable_e2e_option`]。
`AsyncLightGBMManager::GetAndLoad(ModelType)` @0x1006c3124 是「按类型懒加载 + 缓存」（map 命中直接返回，否则交线程池加载），本身不看档位 [推]。
`NNLMRanker::ShouldUseNNLM` @0x100c5cf64 的判据是输入场景（`GetRankingScene`、`GetNNLMParams(InputScene,…)`）与路径长度（`> NNLMRankerParams.data+0xc`，缺省 8 就不用），不看设备 [确]。

#### 1.3 动态档：`EnvContext::IsLowPerformance()` @0x100655630 [确]

AB 参数 `mmkb_cli_engine_enable_low_performance`（@0x102ce8e6e）是逗号分隔的 7 个开关，`EnvContext::Reload` @0x1006884e4 读它构造 `LowPerformanceABInfo` @0x100689b4c：少于 7 段记 `Param size < 7, invalid: `，第 i 段恰为 `"1"` 时 `flag[i] = true`，存 env+0x398..0x39e。
四个「本会话最慢耗时」高水位（毫秒，`max(旧值, 本次)`，env+0x2a8 互斥保护）：

| 字段 | 写入点 | 量的是什么 [推] |
|---|---|---|
| +0x378 | `_wxime_create_session` @0x100652fd0 | 建会话耗时 |
| +0x380 | `PinyinV2Session::ProcessInput` @0x10090f400 | 一次按键处理总耗时 |
| +0x388 | `Searcher::searchAndGetResultImpl` @0x100a63a64 | 一次检索耗时 |
| +0x390 | `searchAndGetResultImpl` @0x100a63d84（取 searcher+0x10b8 的一个 float）| 未知 |

```c
bool EnvContext::IsLowPerformance() const {
  if (byte_0x175 || byte_0x174) return false;          // 某种豁免（未知，见「未知」）
  lock(mu_0x2a8);
  int lvl = atomic_load(device_level /*0x278*/);
  bool weak = lvl >= 2;                                 // Low 档阈值放宽
  if (flag[0] && max_create_session_ms > (weak ? 35  : 25 )) return true;
  if (flag[1] && max_process_input_ms  > (weak ? 100 : 75 )) return true;
  if (flag[2] && max_search_ms         > (weak ? 200 : 150)) return true;
  if (flag[3]) return max_0x390 < 0.0;
  return false;
}
```
（常数 25 / 35 / 75 / 100 / 150 / 200 读自 @0x100655684–0x1006556f8 的浮点立即数。）

低性能时的降级 [确]：
- `SentenceSearcherV3::FeedBeam` @0x100a37a6c：`IsLowPerformance() && flag[5]` → 日志 `Low performance, skip nn re-rank`（@0x102cd1fe7），整句不做 NN 重排。
- `Searcher::searchAndGetResultImpl` @0x100a5fe20 / 0x100a5ff74：`IsLowPerformance() && flag[6]` → 不构造 `RecallE2EModelResultAsync`（不召回 E2E 候选），日志 `Low performance mode`（@0x102cd1136）。
- `PinyinV2Session::ProcessInput` @0x10090f3e8–0x10090f428：本键之前不是低性能、本键之后变成低性能时打点上报（翻转时刻）。
- `flag[4]` 的读者没找到。
- 还有一个独立的看门狗：`WatchDogV2Session::CountAndCheckAliveIfNeed` 里有 `device dynamic performance low, skip`（@0x102c92d4e）。

判定时机：每次查询现算（读高水位），高水位只增不减、跟着 `EnvContext`（会话）走，换会话就重来 [推]。

### 2. 证据表

| 结论 | 置信度 | 地址 / 字符串 |
|---|---|---|
| 设备档来自壳给的 `_wxime_config`，存进单例，再拷进每个会话环境 | 确 | 0x100f610c0、0x1006ec79c |
| 档位数越大越弱；取值同 ABTestDevicePerformance（-5/0/1/2/100）| 推 | 0x100eb7254、0x102f0d727 |
| Mac：Apple Silicon → -5（UltraHigh），否则 0（High）| 确（仅上报路径）| 0x10002f098 |
| 服务端按机型查测评分与档 | 推 | GetDeviceBenchMark* 属性表 |
| E2E / decoder-only / GPU 版 / 英文 E2E / 高端 LTR / 词级 NNLM / 上文预填都按档开关 | 确 | 见 1.2 表 |
| 动态低性能 = 本会话三段耗时高水位超阈值，AB 7 开关控制 | 确 | 0x100655630、0x100689b4c、`mmkb_cli_engine_enable_low_performance` |
| 阈值 25/75/150 ms（档 < 2）、35/100/200 ms（Low 档）| 确 | 0x100655684–0x1006556f8 |
| 低性能时跳过整句 NN 重排（flag5）、跳过 E2E 召回（flag6）| 确 | 0x100a37aa0、0x100a5ff74 |

### 3. 青简落点

#### 3.1 现状

- iOS 键盘：`KeyboardViewController.openEngine` 一启动就 `engine.loadModel()`（只有通变 8 位，`Engine.swift:101`）；内存自保 `KeyboardModel.guardMemoryPressure`：余量 < 8MB 卸英文表、< 4MB 卸模型。
- 重排：bridge `session/rescore.rs` 停键 `DEBOUNCE = 80ms` 后送后台；Core `RESCORE_PATHS = 6`（engine/mod.rs:425）；整句 beam `BEAM_WIDTH = 8`（sentence/mod.rs:86，`const`，引擎层不可调）；键盘里 `set_sentence_generation(false)`（bridge session/mod.rs:186）。
- 「知微重排」在 Mac（`apps/macos/src/host/model/word_model.rs`，`[model] scorers = "both"`），iOS 没有。
- 计时：`PerfRecorder`（Keyboard/Sources/PerfRecorder.swift）已记 `key_ms` / `rescore_ms` / footprint，但**只在 `Diagnostics.enabled` 时**；模型加载耗时已在 bridge `ModelState` 的报告里（`ok sha=… ms=…`）。
- 配置：`[model] enabled / scorers`（crates/qingjian-platform/src/config/mod.rs:317、scorer_set.rs）。

#### 3.2 落点（照 WeType 的两层：静态档 + 本会话动态降级）

**档位类型与判定放 bridge（Rust，Mac 以后也能用；不进 Core，Core 只接收结果参数）。**

1. 新增 `cloud/crates/qingjian-cloud-bridge/src/session/perf/mod.rs`（目录，三个文件各一个类型）：
   - `tier.rs`：
     ```rust
     //! 本机性能档：决定本地模型开不开、重排怎么跑。按本机实测算，不用机型白名单。
     #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
     #[serde(rename_all = "lowercase")]
     pub enum PerfTier { Low, Mid, High }
     ```
   - `probe.rs`：
     ```rust
     /// 判档的输入（壳与 bridge 量到的）。
     pub struct PerfProbe {
         /// 模型首次加载 + 预热耗时（ModelState 已有）。
         pub load_ms: Option<u64>,
         /// 最近 N 次逐键耗时的 p95。
         pub key_p95_ms: Option<f64>,
         /// 本进程可用内存（qj_available_memory_mb）。
         pub available_mb: f64,
     }
     pub fn decide(probe: &PerfProbe) -> PerfTier;
     ```
   - `key_window.rs`：`pub struct KeyLatencyWindow { samples: VecDeque<f32> }`，`push(ms)`、`p95() -> Option<f64>`，容量 `KEY_WINDOW = 64`；同时记本会话最大值（WeType 的高水位）。
   - `mod.rs`：常数初值（**全部是起点，要在最老机型上重扫**）：
     ```rust
     pub const LOW_LOAD_MS: u64 = 400;     // 首次加载超过就 Low
     pub const MID_LOAD_MS: u64 = 150;
     pub const LOW_KEY_P95_MS: f64 = 30.0;  // 按键（不含重排）p95
     pub const MID_KEY_P95_MS: f64 = 16.0;
     pub const LOW_MEMORY_MB: f64 = 12.0;   // 加载后余量
     pub const MID_MEMORY_MB: f64 = 20.0;
     /// 本会话降级：单键耗时高水位超过它，本会话按低一档跑（WeType ProcessInput 75 / 100ms）。
     pub const SESSION_DOWNGRADE_KEY_MS: f64 = 75.0;
     pub const KEY_WINDOW: usize = 64;
     ```
     `decide`：任一项落到 Low 区 → Low；否则任一落到 Mid 区 → Mid；否则 High。缺数据的项不参与（首次启动只有内存）。
2. **bridge `session/mod.rs`**：`Session` 持有 `KeyLatencyWindow` 与当前 `PerfTier`；`push` / `backspace` 里自己量 `refresh()` 的耗时（Rust 侧 `Instant`，不依赖 `Diagnostics.enabled`）写进窗口；新 FFI：
   - `qj_perf_tier(session) -> u8`（当前生效档，含会话降级）；
   - `qj_perf_probe(session) -> *char`（`load_ms= key_p95= avail=` 一行，设置页 / 日志用）；
   - `qj_set_perf_tier(session, u8 tier_or_auto)`（手动覆盖，255 = 自动）。
3. **生效**（`Session::apply_tier`）：
   - Low：不 `load_model`（已加载则 `unload_model`）——整句只用 bigram，与「内存 < 4MB」的现有分支同效果。
   - Mid：模型照载；`DEBOUNCE` 由常数改为 `Session` 字段，Mid 取 200ms；`RESCORE_PATHS` 6 → 3（需要 Core 把它从 `const` 变成 `Engine` 字段 + setter，见 3.3）。
   - High：现状。
   - beam：`BEAM_WIDTH` 改成 `Engine` 的可设参数（setter `set_beam_width`，缺省 8），Low 档 5、Mid 6、High 8（**初值，CLI 常数扫描定**，与 B3 共用）。
   - 本会话降级：`KeyLatencyWindow` 的会话最大值 > `SESSION_DOWNGRADE_KEY_MS` → 本会话按低一档（High→Mid→Low），会话结束复原；对应 WeType `IsLowPerformance`，不落盘。
4. **持久化**：App Group `SharedStore.directory` 下 `perf_tier.toml`：
   ```toml
   version = "1.2"          # CFBundleShortVersionString 的「主.次」，大版本变了就重测
   tier = "mid"
   load_ms = 120
   key_p95_ms = 14.2
   available_mb = 23.5
   measured_at = 1760000000
   ```
   写在键盘侧（Swift，`Engine.swift` 加 `perfTier` / `savePerfTier()`）；没开完全访问时写不进共享容器，就写扩展自己的容器（`SharedStore.localDirectory`，与学习数据同一规则）。读到的档在 `openEngine` 时先 `qj_set_perf_tier` 再决定要不要 `loadModel`（Low 档不加载，省掉 53–90 ms）。重测时机：文件缺失、版本不同、或连续 3 次会话降级。
5. **设置页**：主 App 设置里显示当前档与三项实测值，可选「自动 / 低 / 中 / 高」，写进 `config.toml` 的 `[model] tier = "auto"`（qingjian-platform `ModelConfig` 加字段 `tier: TierSetting`，新类型单独文件 `config/tier_setting.rs`；键盘经桥读）。同一提交改 `docs/user/` 对应的设置页说明。

#### 3.3 与青简约束的冲突与改法

1. **档位只影响性能开关，不碰学习与叠加**：`apply_tier` 只改模型加载、去抖、重排条数、beam；学习、个人 n-gram、云端都不经过它。
2. **Core 平台无关**：Core 只新增两个参数化入口（`set_beam_width`、`set_rescore_paths`），不认识「档」。判档、计时、持久化都在 bridge / 壳。
3. **不用机型白名单**（任务原文）：WeType 用的是「壳给档（Mac 按 Apple Silicon）+ 服务端按机型查测评」——青简不照搬这一半，改用本机实测；动态降级那一半照搬。
4. **输入优先**：计时只在按键回调里取两次 `Instant`，判档在会话开始与停键时做，不在按键路径上做文件 IO（写 `perf_tier.toml` 放 250ms tick 里）。
5. **常数**：WeType 的 25/75/150、35/100/200 ms 是它的进程模型（引擎 + Server）下的值，青简键盘扩展的按键耗时口径不同（`refresh()` 含候选整理），必须在真机 Release 下重扫。

### 4. 复刻步骤

1. bridge：`session/perf/{mod.rs,tier.rs,probe.rs,key_window.rs}`；单元测 `decide` 与窗口 p95。
2. bridge：`Session` 接入窗口与档；`push` / `backspace` 计时；`apply_tier`；三个 FFI（加在 `cloud/crates/qingjian-cloud-bridge/src/lib.rs`，与现有 `qj_load_model` 同处）。
3. Core：`BEAM_WIDTH`、`RESCORE_PATHS` 变成 `Engine` 字段 + setter（缺省值不变，CLI 评测零差异）。
4. 壳：`Engine.swift` 加 `perfTier` / `setPerfTier` / `perfProbe`；`KeyboardViewController.openEngine` 先读 `perf_tier.toml` 再决定 `loadModel`；`KeyboardModel` 的 250ms tick 里在档变化时写文件。
5. 平台配置：`[model] tier`；主 App 设置页；`docs/user/` 设置说明；`docs/notes/crate-notes.md` 记常数。
6. 验收：最老支持机型（或 Instruments 内存压力）上 Low 档 `key_ms` p95 达标；Mid 档 `rescore_ms` 与精度（CLI 冻结集 beam 6 / paths 3）记进 `docs/notes/constant-sweep.md`；设置页能看到并覆盖档位。

单测清单（输入 → 期望）：

| 测试 | 输入 | 期望 |
|---|---|---|
| 全快 | load 80ms, p95 8ms, avail 30MB | High |
| 加载慢 | load 500ms, 其他快 | Low |
| 内存中 | load 80, p95 8, avail 15MB | Mid |
| 缺数据 | 只有 avail 30MB | High（缺的项不参与）|
| 窗口 p95 | push 1..=100 共 100 个（容量 64） | 只看最后 64 个：p95 = 第 61 个值（按实现的分位口径写死）|
| 会话降级 | High 档，push 一次 80ms | `qj_perf_tier` = Mid；新会话复原 High |
| 手动覆盖 | 自动判 High，`set(Low)` | 生效 Low，模型被卸；`set(auto)` 后回 High 并重新加载 |
| 持久化版本 | 文件 version="1.1"，App 是 1.2 | 视为缺失，重测 |
| Core 参数 | `set_beam_width(5)` | 词图 beam 截到 5；缺省 8 时 CLI `--eval-text` 结果与改动前逐条相同 |

### 5. 未知

- `_wxime_config+0x38` 在 Mac 壳里由谁填、填什么：只看到 `KBABTestMgr.deviceInfo` 上报路径用 `isAppleSilicon ? -5 : 0`；直接写 `_wxime_config` 的 Swift 函数没定位（`_$sSo13_wxime_configVMa` 的使用者）。下一步：搜写 `[xN, #0x38]` 且附近有 `isAppleSilicon` 选择子引用的 Swift 函数。
- `cfg+0x50`（AddDictToResources 里 `level==1 && x==1` 的那个 x）的含义。
- `EnvContext+0x174 / +0x175` 两个豁免字节是什么（猜：PC 端 / 调试开关）；看 `EnvContext::EnvContext(_wxime_session_config, _wxime_config)` 里这两字节的来源。
- 高水位 +0x390（searcher+0x10b8 的 float）与 `flag[3]` 判 `< 0` 的意义，`flag[4]` 的读者。
- `DeviceInfo.modelLevel` 与 `E2EDevicePerformance` 在客户端如何映射成 `level`；`GetDeviceBenchMarkRequest` 的发送方（只有 `+descriptor` 引用了它的类）。
- `SetEnable` 的调用者（虚表调用）。
