# D2 云联想的请求量：何时发、发什么、怎么去重缓存、何时排第一

范围：WeType 2.2.3 arm64，源文件 `wxime/engine/session/cloud_search_by_input_v2.cpp`（串 0x102cee9a7）。纯静态阅读。
常数一律是 **WeType 的值，青简要自己重扫**。B5 已写过的「云结果晚到后怎么并进候选」不重复，本篇只补请求侧与 bit22 的来源。

要解决的上游 issue：
- #177「只要停顿就调模型」：几百字输入 100 多次请求、十几万 token，要防抖与按需触发。
- #341：希望大模型整句结果排第一，或给个选项。
- #441：要能关掉联想。青简已有：`[predict] enabled`（偏好设置「启用云联想」/ 菜单栏开关）与 `[predict] slots = 0`（只留整句补全，不要云端词）。
  如果 #441 说的是本地候选里的长词补全，那青简没有对应开关，不在本篇范围。

---

## 1. WeType 怎么做的

### 1.0 一图看懂

```
按键 ProcessInput(key) 0x100df31b0
  └─ Impl::SearchCandidates(Some(key))  —— 先于本地引擎处理这个键（"Pre search"）
       ├─ TrafficControl 锁定期内（服务端下发）               → 不发
       ├─ pending_inputs ≥ 50 项                              → 不发
       ├─ 临时把这个键压进 pending_inputs（函数退出时弹掉）
       ├─ ctx = 光标前文本，从尾部截 64 个字（UTF-8 字符）
       ├─ CancelSameReq(ctx)：和在途请求完全相同              → 留着在途的，不再发
       │                       不同 → 作废在途请求的记录（它回来时会被丢掉）
       ├─ NeedDoRequest(ctx, level) == false                  → 不发
       ├─ 组请求 → TryResponseByCache：命中 → 立即用缓存结果（同步合并，B5 §1.2）
       │                               且 +0x23c==0 时到此为止，不发
       └─ delay = level>1 ? max(Req min interval, 150) : Req min interval
             delay==0 或尾音预测说「拼完了」→ 立刻发
             否则 → delay 毫秒后发（序号防抖：期间又有新请求就作废）
退格 DropLast 0x100df321c：只作废在途请求，不发新请求
```

### 1.1 SearchCandidates（0x100df243c）

```c
void SearchCandidates(optional<string_view> pre_key) {
  if (TrafficControl::ValidTime<CloudSearchByInputV2>())        // 0x100df24cc
      { log("in traffic control, valid time ", t, ", now is "); return; }
  if (pending_inputs.size() >= 50)                              // 0x100df25e4 cmp #0x32
      { log("pending_inputs len exceeds limit ["); return; }
  if (pre_key) { log("Pre search"); pending_inputs.push({text=*pre_key, kind=2}); }   // 0x100df2818
  defer { if (pre_key) pending_inputs.pop_back(); }             // 0x100df2e4c..e94
  ctx = text_ctx->TruncatedTextBeforeCursor(this->ctx_limit /*+0x18*/);   // 0x100df28f8
  if (CancelSameReq(ctx)) { log("Current req is valid"); return; }       // 0x100df2914
  level = TrafficControl[CloudSearchByInputV2].level /*Impl+0x24*/ or 0;
  delay = level > 1 ? max(this->min_interval /*+0x248*/, this->+0x268 /*=150*/) : this->min_interval;  // 0x100df2a50
  if (!NeedDoRequest(ctx, level)) { log("NOT NeedDoRequest"); cloud_info.Reset(); return; }  // 0x100df2a6c
  req = GenSimpleReq(); AppendFullReqData(ctx, req);
  hit = TryResponseByCache(req);                                // 0x100df2ab4
  if (hit && this->+0x23c == 0) return;                         // 0x100df2ac0（+0x23c 的含义见未知）
  immediate = false;
  if (delay >= 1 && tail_predictor /*+0x250*/) {
      keys = 拼接 pending_inputs 里 kind==2 的键（九宫格先转数字）;
      r = TailPredictor(keys, keyboard_type /*+0xec*/);         // 0x100df2c5c
      immediate = r.has_value() && *r;  log("do_req_immediately by predict:", immediate);
  }
  if (delay < 1 || !tail_predictor || immediate)
      CallCloudEngine(hit, network, req);                       // 0x100df2cfc 立刻发
  else {
      seq = ++this->seq /*+0x244，初值 100*/;
      RunAfter(io, cancel_token, delay ms, [seq, req, hit] {    // lambda0 0x100e03e40
          if (this->seq == seq) CallCloudEngine(hit, network, req);   // 0x100e03ea0 只有最新的那次发
      });
  }
}
```
- 入口：`ProcessInput` 每个键都调一次，**在键交给本地引擎之前**（0x100df31e8 再 0x100df3204 才转发），所以是「按键时发」，不是「停顿时发」。[确]
- `SearchCandidates()` 无参版本（0x100df23f4）在非按键的输入变化时调用（谁调未查）。[确：存在；推：用途]
- 退格 `DropLast` 只 `CloudSearchInfo.Reset()`（0x100df3244）然后转发，**不发请求**。[确]
- 上屏之后的「下文联想」不是这个类（另有 CloudTextRecommend / CloudMostLikely），本篇不覆盖。[推]

### 1.2 NeedDoRequest（0x100df8f24）

```c
bool NeedDoRequest(const string& ctx, int level) const {
  if (IsHotFix(ctx)) { log("hint hotfix, do not launch cloud request"); return false; }  // 0x100df8f44
  len = 0;
  for (it : pending_inputs)                                     // 元素 0x50 字节，+0x18 是 kind
      len += it.kind == 1 ? it.text.size()                      // kind 1：已选定的一段（按字节）
           : it.kind == 2 ? 1 : 0;                              // kind 2：一个键
  if (level <= 1 && !ctx.empty()) return true;                  // 0x100df907c：有光标前文且未被限流 → 一个键也发
  if (len < 4 * max(level, 1)) return false;                    // 0x100df9090..909c：无前文时至少 4 个键；限流等级越高门槛越高
  if (len > 6) return true;                                     // 0x100df90bc cmp #6
  if (any(it.kind != 2)) return true;                           // 混了已选定的段 → 发
  switch (keyboard_type /*+0xec*/) {
    case 0: return !IsValidNormalPinyin(concat(keys));          // 全键盘：整串恰好是一个合法音节 → 不发
    case 1: return !IsValidNormalT9Pinyin(keys);                // 九宫格
    default: return true;
  }
}
```
- `IsValidNormalPinyin`（0x1007b4c28）= 在 `wxime::data::valid_pinyin_exclude_special_set`（0x1033d5770）里查整串：**整串是不是一个合法单音节**。所以「本地已经够好」的判定只有一条：4～6 个键且恰好是一个完整音节（`zhuang`、`xiang`）就不问云。[确]
- **没有**本地首选置信度门槛、没有按键速度/间隔判断、没有 App 黑名单（是否开云由会话配置 +0x41 决定，壳按 bundleID 建会话时定，未细查）。[确：NeedDoRequest 内无这些；推：黑名单在壳]
- `IsHotFix`（0x100dfa744）：拿前文 + 待输入键去查服务端下发的 `HotfixHardDict`（`LookUp` 0x100865ad0），命中说明本地已有强制答案，不发。[确]

### 1.3 防抖、尾音预测、限流

| 项 | 值 | 证据 |
|---|---|---|
| `Req min interval`（+0x248） | 来自会话配置 `_wxime_session_config+0x58`；Mac 壳 `Engine.createSession` 里写 **0**（0x1002365b8 `str wzr,[sp,#0x250]`，结构基址 sp+0x1f8） | 构造 0x100df4ebc / 日志串 `"Req min interval:"` |
| 限流时的最小间隔（+0x268） | **150 ms**，常数 | 0x100df4ecc `mov w8,#0x96` |
| 尾音预测阈值 | `SpellingEndPredictDict::GetScore(keys) > 0.35`（全键盘）/ `> 0.45`（九宫格）→ 立即发 | 0x10085be3c `0x3fd6666666666666` / `0x3fdccccccccccccd` |
| pending 上限 | 50 项 | 0x100df25e4 |
| TrafficControl::LockFor(秒) | `valid_until = max(now + s, 旧值)`；在 OnResponse 里按服务端回包调用 | 0x100e029e8、0x100e01d54 |

结论：Mac 上缺省**不防抖，每个键都可能立刻发**（只受 NeedDoRequest、去重、缓存约束）；只有服务端把等级调到 >1 时才变成 150ms 防抖 + 「至少 4×level 个键」。这对 WeType 成立，是因为它的服务端便宜、且一次回包会预填后续几个键的结果（§1.5），青简的 LLM 不能照抄。[确：常数；推：动机]

### 1.4 在途请求：CancelSameReq / OnResponse

```c
bool CancelSameReq(const string& ctx) {                     // 0x100df8db4
  same = false;
  lock(+0x88); cloud_info(+0x30).Visit([&](CloudSearchInfo& info){
      same = info.IsSameRequest(ctx, pending_inputs);           // 0x100e2f5b0
  }); unlock;
  if (!same) cloud_info.Reset();                                // 作废在途记录
  return same;
}
IsSameRequest = IsSameRemainedSearchInput(pending_inputs)       // 0x100e2f358：键序列逐项相等
             && 已选定段逐项相等 && req.before_context == ctx;  // 0x100e2f6c8 起的字符串比较
OnResponse(task_id, …)：                                        // 0x100e019a0
  stale = (info.task_id != task_id)                             // lambda 0x100e02b78 cmp w8,w9
  if stale → 丢；否则清在途记录、算耗时、TrafficControl 处理、ParseResp、CacheResult、发 CloudResultEvent
```
- 新按键来时**不取消网络请求**，只把在途记录作废，旧回包到了按 task id 对不上丢掉。在途请求与新输入完全相同时不重发。[确]
- 网络：`Network::DoRequest(4329, "input", req, …, 5000, 2)`（0x100df9dd4），5000 推为超时毫秒。[确：常数；推：语义]

### 1.5 缓存：TryResponseByCache / CacheResult

```c
bool TryResponseByCache(shared_ptr<Req>& req) {                 // 0x100df9804
  if (!cache /*+0x228，CloudContext 带来的共享 KV*/) return false;
  canon = EraseReqDuplicatedInfo(req);                          // 0x100dfadb4，去掉易变字段
  bytes = canon.SerializeAsString();
  v = cache->Get(cityhash64(bytes));                            // lambda 0x100dfafec，8 字节键
  if (!v || v[0..len(bytes)] != bytes) return false;            // 存的值里带原请求，防哈希碰撞
  derived = v[len] == '1';                                      // 1 = 由别的回包预填的条目
  resp.ParseFromArray(v[len+1..]); ParseResp(resp, …);          // 立即生成候选
  cloud_info.Visit(info.result = parsed);                       // 0x100dfbed0：同步交给合并器
  log("Cache was found, use it"); return true;
}
EraseReqDuplicatedInfo：清空 id、user_id、+0x68、body+0x110/+0x118/+0x10c、
  session_config.app_name / text_place_holder / +0x64、两个重复字段里每项的 +0x20（推：时间戳/坐标）
CacheResult(req, resp)：                                        // 0x100e02538
  if (resp.ret != 0) return;
  put(canon(req) → resp, flag '0');
  for (E : resp.body.<续打结果列表>):                           // 0x100e02620 循环
      r2 = canon(req) + remain_input_sequences.append(E.keys);  // 0x100e026f0..0x100e027b0
      put(r2 → resp{candidates = E.candidates, extra = E.extra}, flag '1');   // 0x100e028e4
```
- **键**：去掉身份与时间类字段后的整份请求（前文、已选段、逐个键、选项）。**不做前缀截断复用**；对应的机制是**服务端一次回包附带「再敲这几个键时的结果」**，客户端逐条预填进缓存——用户下一个键直接命中，不发请求。[确]
- **容量与失效**：`ResourceMgr::LoadUserDict` 里建 `TtlDB(PrefixDB("!uch!", leveldb))`（0x100beb8c4 / 0x100beb990），落盘、跨会话共享；TTL 取资源配置里的值，某个字节数（推：库大小）> 50MB 时除以 5、> 100MB 时除以 15（0x100beb8f8..0x100beb934）。TTL 原值在数据文件里，静态看不到。远程命令 `clean_cloud_cache` 可清空。[确：结构；推：字节数语义]

### 1.6 一次请求带什么（AppendFullReqData 0x100df94d4 / CallCloudEngine 0x100df9bf4）

- 光标前文：`TruncatedTextBeforeCursor(limit)`，limit 缺省 **64 字**（`TextAroundCursorContext` 构造 0x1006ed0b8 `mov w9,#0x40`），
  AB `mmkb_cli_engine_before_context_limit_for_cloud_input` 可改（`SessionCreator` 0x1006b04b4，解析失败保持缺省）。[确]
- 上次上屏的文字（`FillLastSendOutToReq`）。[确]
- **逐个原始键**（`remain_input_sequences`，每个 kind-2 项一条）+ 已选定段（`Selection`）；**不发本地切分**，切分由服务端做。[确]
- AB `mmkb_cli_engine_cloud_input_with_user_word` 打开时 `AppendUserWordCandidates`：用 `EnumPinyinSplits` / `EnumPrefixCompletionSplits`（上限 **500** 种切分，0x100dfc544/0x100dfc574）在用户词典里找词，最多约 **20** 条（0x100dfc750 `cmp w27,#0x14`）随请求发，让服务端知道用户自造词。键长 > 25 时不枚举（0x100dfc0d8 `cmp #0x19`，推）。[确：常数；推：20 的确切语义]

### 1.7 云词排首位：bit22 从哪来（补 B5 §1.8）

- `CandidateID::GetCandidateFlag` 对云词（variant 下标 3）直接返回 `CloudPinyinSearchCandidateID+0x10`（0x10066eb9c `ldr x0,[x1,#0x10]`）。[确]
- `ParseResp`（0x100dfb2f4）把这个 u64 **原样从回包拷进来**：`resp+0x50` 子消息的 `+0x20` 字段（0x100dfb79c..0x100dfb7b4，`sp+0xa8` 就是 `BaseCmdResp`，0x100dfb444）。客户端不计算 bit22，是**服务端逐个回包决定**、同一回包内的云词共享。[确：拷贝；推：回包级字段]
- 所以 WeType 的「云词抢第一」= 服务端打标 **且** 客户端没把本地首选判为受保护（B5 §1.11）**且** 不在 PC 禁止名单。
  `mmkb_cli_engine_pc_allow_cloud_force_top1_expt`（B5 §1.12）：PC（平台码 5/6）非内测号**缺省禁止**，只有 AB 值为 `"1"` 才放开。Mac 版就是 PC：**缺省云词永远拿不到第 1 位**。[确]

---

## 2. 证据表

| 结论 | 置信度 | 地址 / 字符串 |
|---|---|---|
| 每个键在交给本地引擎前触发云请求 | 确 | 0x100df31e8 → 0x100df3204 |
| 退格只作废在途、不发 | 确 | 0x100df3244 |
| 服务端限流锁定期内不发 | 确 | 0x100df24cc；串 `"in traffic control, valid time "` |
| pending ≥ 50 不发 | 确 | 0x100df25e4 `cmp x8,#0x32` |
| 有前文且未限流：一个键也发 | 确 | 0x100df907c–0x100df9088 |
| 无前文：至少 4×max(level,1) 个键 | 确 | 0x100df9090 `csinc`、`lsl #2` |
| > 6 个键必发；4～6 个键且恰好是一个合法音节不发 | 确 | 0x100df90bc `cmp #6`；0x100df9180；`valid_pinyin_exclude_special_set` 0x1033d5770 |
| 热修词典命中不发 | 确 | 0x100df8f44；串 `"hint hotfix, do not launch cloud request"` |
| 与在途请求相同不重发 | 确 | 0x100df2914；串 `"Current req is valid"` |
| 同一 = 键序列 + 已选段 + 前文逐项相等 | 确 | 0x100e2f5b0、0x100e2f358 |
| 旧回包按 task id 丢 | 确 | 0x100e02b84 |
| 防抖间隔来自会话配置 +0x58，Mac 壳写 0 | 确（偏移对应为推） | 0x100df4ebc；0x1002365b8 |
| 限流时最小间隔 150ms | 确 | 0x100df4ecc |
| 序号防抖：只有最新一次真正发出 | 确 | 0x100e03ea0 `cmp w8,w9` |
| 尾音预测阈值 0.35 / 0.45 → 立即发 | 确 | 0x10085be3c–0x10085be64 |
| 前文截尾 64 字，AB 可改 | 确 | 0x1006ed0b8；串 `mmkb_cli_engine_before_context_limit_for_cloud_input` |
| 缓存键 = 去易变字段后的请求序列化，cityhash64 + 原文校验 | 确 | 0x100dfadb4、0x100dfb028、0x100dfb0e0 |
| 回包附带续打结果并预填缓存 | 确 | 0x100e02620–0x100e028e4（flag '1'） |
| 缓存落盘 leveldb、带 TTL、`!uch!` 前缀、库大时 TTL 缩短 | 确（字节数语义推） | 0x100beb8c4、0x100beb990、0x100beb8f8 |
| 命中缓存时立即同步给合并器 | 确 | 0x100dfbed0 |
| 用户词随请求发：切分上限 500 | 确 | 0x100dfc544 `mov w3,#0x1f4` |
| 云词 flag 原样取自回包 | 确（回包级为推） | 0x10066eb9c；0x100dfb7a0–0x100dfb7b4 |
| PC 缺省禁止云词抢首位 | 确 | B5：0x100c0c194、0x100c0fce4 |

---

## 3. 青简落点

### 3.1 现状（读代码核实）

- 触发：每次缓冲变化（含退格、分段上屏后剩余部分）壳都调 `Engine::request_prediction`——Mac `apps/macos/src/imk/controller/display.rs:60`、
  Windows `apps/windows/server/src/dispatch/composed/mod.rs:33`（`recompose`）、素笺 iOS `cloud/crates/qingjian-cloud-bridge/src/session/cloud.rs:172`。
- Core 门槛（`crates/qingjian-core/src/engine/prediction/mod.rs`）：私密 / 辅码 / 英文 / 表达式 / 直通不发；`MIN_PREDICTION_LETTERS = 2`（`engine/mod.rs:413`）；
  简拼只问整句（`max_items = 0`）；关了整句又是简拼时不发。**没有**「本地已够好」判断、**没有**退格判断、**没有**同一段组句内的增量门槛。
- 防抖：`qingjian-predict/src/worker.rs` 一个后台线程，`recv_timeout(debounce)` 收到最后一个请求（`[predict] debounce_ms = 300`）；
  网络调用 `block_on` 期间排队的请求，下一轮防抖时只留最后一个。没有速率上限，没有 429 处理（`PredictError` 只有 `Api(OpenAIError)`）。
- 缓存：`cache.rs`，容量 `CACHE_CAPACITY = 64`（`worker.rs`），键 = `kind / before / after / letters / pinyin / candidates / max_items / want_sentence / text / target_language`。
  `candidates`（本地前 5）随学习变化，`pinyin` 由 `letters` 决定，两者都让同一输入命中不了；不做前缀复用；不落盘。
- 用户文档 `docs/user/cloud/index.md`「输入拼音时停顿 300 毫秒后」。

结果：一段组句里，最后选词前的那次停顿几乎必然 ≥ 300ms，所以**基本每段组句至少一次请求**，中途停顿、退格各再加一次。

### 3.2 新规则（带初始常数）

全部门槛放 Core（平台无关，四个壳不用改判定），防抖与限流留在 `qingjian-predict`。

| # | 规则 | 初值 | 出处 |
|---|---|---|---|
| R1 | 字母数 < `min_letters` 不发 | **4** | WeType `NeedDoRequest` 无前文时 4×level（0x100df9090）；青简有前文也不放宽到 1——LLM 一次请求的成本比 WeType 服务端高得多 |
| R2 | 字母数 ≤ `short_full_letters` 且能整串切成**完整**音节（不含简拼、不含未完音节）→ 不发 | **6** | WeType 只跳「恰好一个音节」（0x100df9180）；青简扩成多音节全拼：本机日志里这类组句本地首选命中 **95%**（全体 90% 左右），云端几乎帮不上 |
| R3 | 退格（新字母串是上次请求字母串的真前缀，或比上次短）→ 不发，只换序号作废在途 | — | WeType `DropLast`（0x100df3244） |
| R4 | 同一段组句里，距上次**实际发出**的请求，字母至少多 `min_growth` 个才再发 | **3** | 青简设计（对应 WeType 预填续打结果省掉的那几次） |
| R5 | 整句补全前缀复用：上一次回包有整句 S，新字母串延长了上次的字母串，且 S 的读音仍能覆盖新字母串（逐字检查读音，末音节允许前缀）→ 直接复用，不发 | — | 青简设计；WeType 的对应物是服务端回包预填续打结果（`CacheResult` 0x100e02620） |
| R6 | 缓存键去掉 `candidates` 与 `pinyin`；容量 64 → **256** | 256 | WeType `EraseReqDuplicatedInfo`（去易变字段）；容量青简自定 |
| R7 | 在途请求与新请求完全相同（缓存键相同）→ 不重发，等它回来 | — | WeType `CancelSameReq`（0x100df8db4） |
| R8 | 速率上限：令牌桶，突发 6、每 5 秒补 1（≈ 12 次/分） | 6 / 5s | 青简设计；WeType 是服务端 `TrafficControl` 下发 |
| R9 | 收到 HTTP 429 → 锁定 `Retry-After` 秒（没有就 30 秒），期间不发 | 30s | WeType `TrafficControl::LockFor`（0x100e029e8） |
| R10 | 防抖保持 300ms（`debounce_ms`）；被 R9 锁过之后的 10 分钟内防抖抬到 `max(debounce_ms, 600)` | 300 / 600 | WeType「限流等级 >1 时 max(间隔,150)」（0x100df2a50）；数值按 LLM 重定 |
| R11 | 按需模式：`[predict] trigger = "auto" \| "manual"`，manual 时只在快捷键 `[shortcut] predict_now` 按下时发（绕过 R1/R2/R4/R5，不绕过隐私门槛） | auto；Mac `control+option+p`、Win `ctrl+alt+p` | #177 用户诉求；WeType 无对应 |

不采纳：WeType 的「有前文就一个键也发」「Mac 缺省不防抖」「尾音预测立即发」——前提是服务端便宜且能预填续打结果，青简的 OpenAI 兼容接口两条都不满足。

### 3.3 改哪些文件

**Core**（`crates/qingjian-core/src/engine/prediction/`，一类型一文件）：

```rust
// trigger.rs —— 新文件
/// 云联想什么时候发请求。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum PredictionTrigger {
    /// 组句中按门槛自动发（缺省）。
    #[default]
    Auto,

    /// 只在用户按快捷键时发。
    Manual,
}

// policy.rs —— PredictionPolicy 加字段（仍是 Copy）
pub trigger: PredictionTrigger,
/// 少于这么多字母不发（WeType 4，0x100df9090）。
pub min_letters: usize,           // 4
/// 不超过这么多字母且整串是完整音节时不发（WeType 只跳单音节，0x100df9180）。
pub short_full_letters: usize,    // 6
/// 同一段组句里比上次发出的多几个字母才再发。
pub min_growth: usize,            // 3

// gate.rs —— 新文件：Engine 里记「上次实际发出了什么」，判断 R2～R5
/// 本段组句里上一次真正发出的联想请求，用来判断退格、增量与整句复用。
#[derive(Debug, Clone, Default)]
pub(crate) struct PredictionGate {
    /// 上次发出时的字母串（解码后的全拼，不含 `'`）。
    letters: String,

    /// 上次回包里的整句补全（复用判断用）。
    sentence: Option<String>,
}
pub(crate) enum GateVerdict { Send, Skip(SkipReason), Reuse(Prediction) }
impl PredictionGate {
    pub(crate) fn judge(&self, letters: &str, policy: &PredictionPolicy, full_syllables: bool,
                        covers: impl Fn(&str, &str) -> bool) -> GateVerdict;
    pub(crate) fn note_sent(&mut self, letters: &str);
    pub(crate) fn note_reply(&mut self, prediction: &Prediction);
    pub(crate) fn reset(&mut self);      // 组句清空 / 上屏整段时
}

// skip_reason.rs —— 新文件（日志与 CLI 统计用）
pub enum SkipReason { TooShort, ShortFullPinyin, Deletion, TooLittleGrowth, Manual }
```

- `prediction/mod.rs` `request_prediction`：在现有门槛之后、`submit` 之前调 `gate.judge`；`Reuse` 时把复用结果放进一个 `pending_local: Option<Prediction>`，
  `poll_prediction` 先吐它（序号用新序号）；`Send` 才 `submit` 并 `note_sent`。新增：
  ```rust
  /// 快捷键「现在问云」：绕过长度、增量与复用门槛，隐私与模式门槛照旧。
  pub fn request_prediction_now(&mut self, surrounding: Option<SurroundingText>, candidates: &[Candidate]) -> Option<u64>;
  ```
- `covers(sentence, letters)`：用现有 `char_reads` 逐字对读音，`letters` 末段允许是音节前缀，允许 `tolerance(letters)` 个不符（与 `validate_cloud_words` 同一把尺）。放 `prediction/cover.rs`（自由函数）。
- `engine/mod.rs`：`MIN_PREDICTION_LETTERS` 删掉，改读 policy；Engine 加字段 `prediction_gate: PredictionGate`，组句清空时 `reset`。
- 双拼、问字：问字模式（`?` 开头）不受 R1/R2/R4/R5 约束（问题本身短），只受 R3/R6～R10。

**qingjian-predict**：
- `config.rs` `PredictConfig` 加 `trigger: String`（`"auto"` / `"manual"`）、`min_letters = 4`、`short_full_letters = 6`、`min_growth = 3`、`rate_burst = 6`、`rate_refill_ms = 5000`；`policy()` 透传前四个。
- `cache.rs` `PredictionCache::key` 去掉 `request.pinyin` 与 `request.candidates`；`worker.rs` `CACHE_CAPACITY` 64 → 256。
- 新 `rate.rs`：`pub struct RateLimiter { burst: u32, refill: Duration, tokens: f64, last: Instant, locked_until: Option<Instant> }`，
  `fn try_take(&mut self, now: Instant) -> bool`、`fn lock_for(&mut self, d: Duration)`。`worker.rs` 防抖后、查缓存未中时 `try_take`，拿不到就丢这个请求（记 debug 日志，不排队——过时的联想没有价值）。
- `error.rs` 加 `RateLimited { retry_after: Option<u64> }`；`chat_client.rs` 遇 429 返回它；`worker.rs` 收到后 `lock_for`。
- `worker.rs` R7：记下「在途请求的键」，防抖期间收到同键请求直接合并（现有 `debounce` 已是只留最后一个，补一条：最后一个与在途同键时不再发，等在途回包按新序号回）。

**壳**：只加 `predict_now` 快捷键的识别与调用（Mac `apps/macos/src/imk/controller/`、Windows `apps/windows/server/src/dispatch/`、iOS bridge 可不做）。快捷键进 `[shortcut]`，模板在 `crates/qingjian-platform/src/config/mod.rs` 的 `template_shortcut_keys!`。

### 3.4 可选「云整句排第一」（#341）与 B5 晚到规则统一

- 开关：`[predict] sentence_first = false`（缺省关）。理由：WeType Mac/PC 缺省同样**禁止**云词抢首位（`pc_allow_cloud_force_top1_expt` 非 `"1"` 即禁止），只有服务端打 bit22 + AB 放开才抢。
- 青简没有 bit22，用本地条件替代：整句补全 S 的字数 == 本地切出的音节数，且 `covers(S, letters)` 无不符（是「这段拼音的整句转换」，不是续写；续写仍只在 Tab 位）。
- 并入 B5 的 `engine/late/`：新增 `LateSource::CloudSentence`；`LatePolicy` 加 `cloud_top1: bool`（= `sentence_first`）。`merge_late` 里：
  ```rust
  // 云整句抢第 1 位的条件：开关开、用户没动过（!state.touched）、本地首选不受保护（!protect0，B5 is_protected_top1）、
  // S 是纯转换（上面两条）。满足时 S 放第 0 位、原首选退第 1 位；不受 grace_ms 限制——
  // LLM 回包通常 > 1s，按 200ms 时限这个开关永远不生效，用户打开它就是愿意接受首选晚到被换。
  // 不满足时按 B5 原规则：时限内最多插 1 条、时限外从 frozen+1 位起插。
  ```
  首位是云整句时 `Space` 走 `accept_prediction`（来源 `CloudSentence`，切词记个人 n-gram，已有）。
- 风险：用户在回包到达瞬间按空格会上屏被换掉的词。缓解沿用 B5 `note_candidates_touched`，并加一条：S 到达后 `late_top1_guard_ms = 150` 内按下的 Space 仍上屏到达前的首选（壳上报按键时刻，Core 比较）。

### 3.5 用户文档要改的页

- `docs/user/cloud/index.md`：「云端候选词」一节的「停顿 300 毫秒后」改为写清何时发（≥4 个字母、短全拼不问、退格不问、多敲几个字母才再问、每分钟上限）；新增「按需模式」「整句排第一」两小节；`slots = 0` 与总开关写进「不联想的情况」（顺带回答 #441）。
- `docs/user/settings/preferences.md`：「云服务 → 高级」加「触发方式（自动 / 按快捷键）」「云整句排第一」。
- `docs/user/getting-started/keys.md` 与 `docs/user/input/shortcuts.md`：加 `predict_now` 快捷键一行。
- `docs/user/help/data-and-logs.md`「云联想发什么」：发送内容不变，补一句「多数按键不发请求」的条件。
- `docs/notes/crate-notes.md`：qingjian-predict 的缓存键与容量、限流常数；core 的 `PredictionGate` 与四个常数。

### 3.6 与青简约束的冲突与改法

- 平台层不做判定：门槛全在 Core `request_prediction`，壳不动判定逻辑，只多一个快捷键。
- 输入优先：门槛都是 O(字母数) 的字符串检查；`covers` 每个字查一次词库，组句通常 < 20 字，在按键路径上可接受（上限：S 超过 32 字时直接判不覆盖）。
- 不跨用户：缓存、令牌桶都只在本机内存；不落盘（WeType 落盘是因为它的结果与用户无关，青简的请求带前文，落盘有隐私代价）。
- 一个类型一个文件：新增 `trigger.rs`、`gate.rs`、`skip_reason.rs`、`cover.rs`（自由函数）、`rate.rs`。

---

## 4. 复刻步骤

1. Core：加 `PredictionTrigger`、`PredictionPolicy` 新字段、`PredictionGate`、`SkipReason`、`cover.rs`；`request_prediction` 接 gate；加 `request_prediction_now`。先写单测（下表 1～10）。
2. predict：缓存键与容量、`RateLimiter`、429 处理、配置字段。单测 11～15。
3. CLI：`apps/cli` 加 `--predict-sim <input-log.jsonl>`：按日志重放，用 Engine 的真实门槛（不是 Python 近似）数「会发几次 / 被哪条规则拦下」，输出各 SkipReason 计数。合并前跑一次，结果记 `docs/notes/`。
4. 壳：Mac、Windows 接 `predict_now`；文档按 3.5 同一提交改。
5. `sentence_first` 等 B5 `engine/late/` 落地后再接（依赖 `LatePolicy` / `merge_late`）。
6. 真机：开云联想打 300 字，对比改前改后服务商后台的请求数与 token；目标请求数降 ≥ 50%，云词被选次数不降超过 10%（日志 `prediction` 后紧跟的 `commit` 来源）。

单测清单：

| # | 输入 | 期望 |
|---|---|---|
| 1 | 字母 `nih`（3） | 不发，`TooShort` |
| 2 | `nihao`（5，全拼完整） | 不发，`ShortFullPinyin` |
| 3 | `zhuang`（6，单音节） | 不发，`ShortFullPinyin` |
| 4 | `nihaoa`（6，完整）→ `nihaoaw`（7） | 前者不发；后者发 |
| 5 | `fhyq`（4，简拼） | 发（不是完整全拼），且 `max_items = 0` 只问整句 |
| 6 | 已发 `suoyiwoxiang`，接着 `suoyiwoxian`（退格） | 不发，`Deletion`；序号 +1 |
| 7 | 已发 `suoyiwo`，接着 `suoyiwox`、`suoyiwoxi` | 都不发，`TooLittleGrowth`；`suoyiwoxia` 发 |
| 8 | 已发 `suoyiwoxiangq` 回包整句「所以我想去吃饭」，接着 `suoyiwoxiangqu` | 不发，复用，`poll_prediction` 立刻吐整句 |
| 9 | 同 8 但接着 `suoyiwoxiangm` | 发（「去」读 qu，不覆盖 m） |
| 10 | `trigger = Manual`，任意输入 | `request_prediction` 不发；`request_prediction_now` 发（3 个字母也发） |
| 11 | 同字母、不同 `candidates` 的两次请求 | 缓存键相同 |
| 12 | 令牌桶突发 6：连续 7 次未命中缓存 | 第 7 次被丢 |
| 13 | 丢弃后过 5 秒 | 再拿到 1 个令牌 |
| 14 | 服务商返回 429 + `Retry-After: 20` | 20 秒内 `try_take` 全部 false |
| 15 | 在途请求与新请求同键 | 不重发，回包按新序号交付 |

---

## 5. 收益估算（已跑）

脚本 `proto/d2-request-sim.py`（Python 近似 Core 规则），输入本机 `~/Library/Application Support/Qingjian/input-log.jsonl`：
1199 次上屏、2134 个汉字、1116 段组句。日志只有每段「第一键到上屏」的毫秒数 `ms`，没有逐键时间，所以这样建模：
键间隔取 100ms（日志 ms/键 的 p25，偏保守），最后一键到上屏的余量 ≥ 300ms 算一次停顿；中途停顿按概率 p 随机撒在键之间；
退格重打（`retype` 事件）现规则各算一次；缓存键现规则按「字母串」算（实际还含候选，**偏向现规则**）。没模拟 R5（整句复用）与 R8（令牌桶），所以新规则的数是上限。

| 中途停顿概率 p | 现规则：触发 / 实发（扣缓存） | 新规则 R1+R2+R3+R4+R6：触发 / 实发 | 实发降幅 |
|---|---|---|---|
| 0 | 688 / 501 | 230 / 188 | 62% |
| 0.1 | 1061 / 843 | 322 / 278 | 67% |
| 0.2 | 1413 / 1150 | 395 / 336 | 71% |

拆开看（p = 0.1）：去掉 R2 实发回到 473（R2 是最大的一条）；R1 改回 2 个字母 312；去掉 R4 305。
只照抄 WeType 的 R2（单音节才跳）实发 446，降幅 47%。
现规则约每 2.5～5 个汉字一次请求，与 #177「几百字 100 多次」吻合。
质量代价：被 R2 拦下的「≤ 6 字母完整全拼」组句 356 段，本地首选命中 95%、前五 100%（全体：91% / 99%），云端基本帮不上。

换成 Core 真实门槛后的精确数：做复刻步骤 3 的 `--predict-sim`，把 Python 近似换掉；有逐键时间戳后（输入日志加一个可选 `key_ms` 数组）再精确模拟防抖与令牌桶。

---

## 6. 未知

- `Impl+0x23c`（= 会话配置 +0x54；Mac 那条分支里 cfg+0x50/+0x54 写的是常数 3 / 24，取自 0x102e1dfc0）：缓存命中后它非 0 就照样走发送路径并带 `hit=true`。
  按这个读法 Mac 上命中缓存后**仍会再发一次**（刷新用？），与「缓存省请求」矛盾；也可能 0x1002365a8 前另一分支写的是 0。下一步：看 `CallCloudEngine` 里 `hit` 存到回调（0x100df9d30 `strb w24,[sp,#0x138]`）之后在 `OnResponse` 怎么用。
- `TrafficControl` 的 level（`Impl+0x24`）由谁写：只看到 `LockFor`，没找到设 level 的函数。下一步：`TrafficControl` 其他成员与 `mmime.TrafficControlOption` 的解析。
- 缓存 TTL 原值：在资源 capnp 配置里，静态读不到；`[res+0x80]` 是否库大小未证。
- `SearchCandidates()` 无参版本的调用者（选段 / 光标移动？）未查。
- `AppendUserWordCandidates` 的 20 条上限是否就是 `cmp w27,#0x14` 的含义，未细读循环。
- 是否有按 App 关闭云请求：只看到会话配置 +0x41（壳 `createSession(bundleID:…)` 0x100235890 构造），没细读壳里的判断。
