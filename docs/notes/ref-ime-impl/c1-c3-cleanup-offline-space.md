# C：用户词清理与热词（C1）、完全离线（C2）、中英自动空格（C3）

材料：RefIME 2.2.3 arm64（`/tmp/ref-ime-rea/RefIME.arm64`），纯静态。地址都是 arm64 片的 VM 地址。
置信度：[确] 直接看到的符号、字符串、指令常数；[推] 多条证据推出；[猜] 名字或经验猜测。
常数全是 **RefIME 的值，青简要自己重扫**。

---

## C1 用户词清理与热词

### 1. RefIME 怎么做的

#### 1.1 总体结构 [确]

- 三种用户词库各配一个 `UserDictCleanStrategyMaker(HyperParams, u16 kind)`，构造点都在
  `ResourceMgr::Impl::LoadUserDict`（make_shared 调用在 0x100be963c / 0x100be98cc / 0x100be9ccc）：

  | kind | 词库 | 缺省策略（Maker::Impl ctor 0x10082f8b0） |
  |---|---|---|
  | 0x80 (128) | 主用户词库 `UserDictV2`（AddLabeledDict 之后） | `DynamicTarget`，参数取 `HyperParams::UserDictCleanParams()` 的前 4 个 u32，不够补 0 |
  | 0x94 (148) | 潜在用户词 `"!potential_ud!"` | `Potential`，常数见下 |
  | 0x82 (130) | 紧接着构造的 `UserEnglishDict`（构造函数签名末参就是 `shared_ptr<IUserDictCleanStrategy>`）[推] | `StaticTarget`（固定阈值，不自适应） |

- 策略接口 `IUserDictCleanStrategy` 的虚表（Dynamic 版在 0x1031995d0）：
  `+0x10 TryToAdjustCleanUpParameters` / `+0x18 ShouldPerformCleanUpOperation` / `+0x20 ShouldCleanUpCurrentItem` / `+0x28 NextCheckPinyinNumber`。
- AB 开关 `mmkb_cli_engine_user_dict_clean_strategy`，**缺省串 `"128,2,100000,10,5,2592000,30000,1"`**（0x10082ed4c）。
  `Maker::Impl::TryUpdateStrategyByAbTest`（0x10082ecb0）最多每 600 秒（steady_clock 差 ≥ 0x8bb2c97000 ns）重读一次。

#### 1.2 触发时机：每学一个词，增量扫后面 N 个拼音键 [确]

`UserDictV2::Impl::SyncAdd`（0x10075712c）在写入一个用户词后（0x100757594 起）：

```c
if (!this->clean_disabled /*+0x170*/ && this->strategy /*+0x178*/) {
    u64 total_py = atomic_load(this->pinyin_key_count /*+0x1f0*/);   // [推] 词库里拼音键总数
    strategy->TryToAdjustCleanUpParameters({total_py, true});
    bool perform = strategy->ShouldPerformCleanUpOperation({total_py, true});
    rw->TryToRemoveNextSyllables(added_syllable, strategy, /*dry_run=*/!perform);  // 虚表 +0x78
}
```

`UserDictReaderWriterV2::Impl::TryToRemoveNextSyllables`（0x100765654）：

```c
// impl 字段：+0x108 cursor(string，上次扫到的拼音键), +0x120 laps(u32), +0x124 scanned, +0x128 reserved [推]
if (cursor.empty()) cursor = start_key;
if (laps > 1) return;                       // 一个进程生命周期最多绕两圈 [推]
it = kv.Seek(cursor);
for (i = 0; i < strategy->NextCheckPinyinNumber(); i++) {   // 缺省 10 个拼音键
    scanned++;
    if (!it.Valid()) { if (laps++ == 0) { it.Seek(""); continue; } else break; }   // 绕回开头
    key = it.Key();                          // 形如 "ni,hao" 的音节串
    if (!key_is_ascii_letters_comma_separated(key)) { record; it.Next(); continue; }  // 坏键跳过
    mixed_case = key 同时含大写与小写字母;     // 0x100765a70–0x100765ae4，用 __DefaultRuneLocale 的 0x1000/0x8000 位
    for (item in YieldFromPayload(it.Value())) {
        bool hard = false;
        if (mixed_case) {
            age = now_sec - item.last_time;
            if (age > 604800 /*7d*/) hard = true;
            else if (age >= 259201 /*3d+1s*/ && item.freq < 5) hard = true;
        }
        if ((kind == 0x80 || kind == 0x94) && item.word 全是 ASCII 字母) {   // 表 0x102eb9d50 只标了 A–Z a–z
            log("dirty value: …, key is: …"); remove(key, item); continue;     // 无条件删
        }
        if (dry_run || hard) { if (!dry_run) remove(key, item); continue; }  // dry_run 时一律保留
        CleanUpItem ci = { item.freq, item.last_time, scanned, reserved, &key,
                           is_self_making = (item.flags@+0x25 >> 1) & 1 };
        if (strategy->ShouldCleanUpCurrentItem(ci)) remove(key, item);
    }
    cursor = key; it.Next();
}
```

要点：**清理是摊在每次学词上的，每次只看 10 个拼音键**，不做全表扫描，不阻塞按键（SyncAdd 本身在词库的线程池上）。

#### 1.3 计分 / 淘汰判据 [确]

RefIME **没有「次数 × 时间衰减」的连续分**，是「频次阈值 AND 闲置天数阈值」双门槛，阈值随整体保留率自适应。

`CleanUpItem`（ShouldCleanUpCurrentItem 的参数，0x1008317e8 读法）：
`+0 u32 freq`、`+4 u32 last_time(秒)`、`+8 u32 scanned_cnt`、`+0xc u32 reserved_cnt`、`+0x10 string* pinyin`、`+0x18 bool is_self_making`。

**DynamicTargetUserDictCleanStrategyParam**（DebugInfo 0x100831260 给出字段名；默认构造 0x100830770 + 常量 0x102e4a3e0）：

| 偏移 | 字段 | 默认构造值 | AB 缺省串里的值 | DeserializeFrom 的下限校验 |
|---|---|---|---|---|
| +0x00 | `py_cnt_threshold_` | 0xFFFFFFFF | 100000 | — |
| +0x04 | `check_next_n_cnt_` | 10 | 10 | ≥ 1 |
| +0x08 | `word_freq_threshold`（baseline） | 5 | 5 | ≥ 5 |
| +0x0c | `time_threshold`（秒，日志里 /86400 显示为 day） | 2592000 (30 天) | 2592000 | ≥ 30 天（`WXIME_ONE_DAY_SECONDS*10 + kOneLevelTime*kMaxStrictLevel`） |
| +0x10 | `py_target_cnt` | 30000 | 30000 | ≥ 20000 |
| +0x14 | `no_clean_self_making_` | false | 1 | — |

策略对象状态：`+0xc8 int level`（0..5）、`+0xd0 double target_rate`（初值 1.0）、`+0xd8 cur_level_scanned`、`+0xdc cur_level_reserved`。

```c
// TryToAdjustCleanUpParameters  0x100831cf0
if (opt.valid && opt.total_py) target_rate = min(1.0, (double)py_target_cnt / opt.total_py);

// ShouldPerformCleanUpOperation  0x100831d60
return opt.valid && opt.total_py >= py_cnt_threshold_;      // 缺省：拼音键 ≥ 10 万才真删

// AdjustedStrategy(scanned, reserved)  0x100831bcc —— kOneLevelTime = 345600 秒(4 天)，kMaxStrictLevel = 5
bool adjusted = false;
if (scanned != 0 && cur_level_scanned > check_next_n_cnt_) {
    double overall = (double)reserved / scanned;
    double cur     = (double)cur_level_reserved / cur_level_scanned;
    if (min(cur, overall) > target_rate + 0.05) {            // 留得太多 → 收紧
        if (level <= 4) { level++; adjusted = true; }
    } else if (max(cur, overall) < target_rate - 0.05 && level >= 1) {   // 删得太多 → 放松
        level--; adjusted = true;
    }
}
freq_th = word_freq_threshold + level;                 // 5..10
time_th = time_threshold - level * 345600;             // 30 天 .. 10 天
return {adjusted, freq_th, time_th};

// ShouldCleanUpCurrentItem  0x1008317e8
if (no_clean_self_making_ && item.is_self_making) return false;      // 用户自造 / 手动加的词豁免
(adjusted, freq_th, time_th) = AdjustedStrategy(item.scanned, item.reserved);
lock;
if (adjusted) { cur_level_scanned = cur_level_reserved = 0; log("user_dict_clean_strategy adjusted? …"); }
cur_level_scanned++;
if (item.freq >= freq_th) { cur_level_reserved++; return false; }
if (now_sec - item.last_time > time_th) return true;                 // 低频且闲置够久 → 删
cur_level_reserved++; return false;
```

**StaticTarget**（ctor 0x10082fc98）：继承 Dynamic，`TryToAdjustCleanUpParameters` 是空函数（0x10082f714），参数 = 默认构造但
`py_cnt_threshold_ = 0`（总是执行）。target_rate 永远 1.0，level 永远 0 → 固定「freq < 5 且闲置 > 30 天 → 删」，每次学词看 10 个键，自造词不豁免。

**Potential**（kind 0x94，常数 0x102e4a3d0 = `{1296000, 5, 604800, 10}`）：

```c
// ShouldCleanUpCurrentItem  0x10082f7f4
age = now_sec - item.last_time;
if (item.freq < 5 && age > 604800 /*7d*/) return true;
if (age > 1296000 /*15d*/) return true;
if (pinyin 为空) return true;
if (pinyin 含 '#' 或 ',') return false;      // 多音节/特殊键才保留
return true;
// ShouldPerform 恒 true；NextCheckPinyinNumber = 10
```

AB 串格式（解析在 0x10082ee58 起）：`;` 分多条，每条 `,` 分：`kind,target_type,参数…`；kind 必须等于本 Maker 的 kind；
target_type 单个数字，跳转表 0x102e4a134：`0` 关闭（策略置空，不清理）、`1` Static、`2` Dynamic、`3` Potential；余下子串交给对应 Param 的反序列化。

#### 1.4 「回滚」与 `user_dict_clean_up.bin`：都不是清理的回滚 [确]

- `mmkb_cli_engine_user_dict_roll_back`（与 `_pc` 变体）只在 `UserDictSingleEmitWrapper::Impl::Impl`（0x100957050）读，配套字符串
  `TryRevertLastRecord`、`Invalid rollback strategy`、`Revert not support for type:`：这是**撤销最近一次上屏造成的学习**（上屏后马上退格之类），
  不是撤销清理。RefIME 的清理**没有清理记录、不可回滚**，被删的词直接从 KV 删掉。[确：字符串与引用点；语义 推]
- `engine/persistent_data/user_dict_clean_up.bin`（字符串 0x102ce81cd，在 `resource_holder.cpp` 的全局初始化里）是
  `group_sync::UserDictCleanUpper::Impl::HeadInfo` 的落盘（`BinaryFileReadWriter<HeadInfo>`）。它服务的是**多设备「清空用户词库」同步**：
  `ForceCleanUp(needPush)`（0x100e4e63c）= 写 HeadInfo → `ResourceMgr::ResetUserDict()` 整库清空 → `RePushIfNeed`；
  服务端推拉命令 `user_dict_clean_push` / `user_clean_up_pull`，日志 `clean done? groupId: … current version: … push_ok:`。HeadInfo 字段 = {groupId, version, push_ok}[推]。

#### 1.5 联想（bigram）里的数字条目另有分档过期 [确]

`UserAssociationRW::CheckAndRetireNumber`（0x100835d8c），只对数字/小数点构成的联想条目：
- 曝光 ≥ 20 且点击率 < 0.05 → retire；
- 否则按次数分档过期：count == 1 → 30 天，count ≤ 5 → 60 天，其余 → 90 天（`__data` 0x1033712a8 = {2592000, 5184000, 7776000}），
  `silent_days > expiry_days` 就 retire。日志 `Tier1(30` / `Tier2(60` / `Tier3(90`、`[CTR]: … silent_days= … expiry_days=`。
这就是「最近 90 天 + 全期高频」最接近 RefIME 的具体做法：**次数越多，允许闲置越久**。

#### 1.6 热词：UserHotWordDetector / Classifier [确 + 推]

RefIME 的「热词」是**用户的常用语/快捷短语**（键 → 值，如地址、手机号；`addHotWord:value:`、`HotWordsManageWindow`），
Detector 负责「发现用户反复输入的同一段长文本，提示存成热词」。

入口 C API `_wxime_add_fetch_user_hot_word_detection`（0x100664c00）→ `UserHotWordHandler::Impl::AddFetch(text, cb)`（0x10082a8b0）：

```c
type = UserHotWordClassifier::CheckType(text, nullptr);           // 0x10080a774
detector->Add(text, type);                                        // 异步投到线程池（0x10080fe88）
rec = detector->LookUp(text);                                     // 同步查；返回 optional<{count,last_time,type}>
if (!rec) return;                                                 // 第一次见：异步 Add 还没落盘 → 不推荐
if (user_hot_word_dict 里按拼音查到同一段文本) return;            // 已经是热词
Classify(text, …) → 回调 _wxime_user_hot_word_recommend_event     // 壳弹「存为热词」提示（HotWordsInputTipsView）
```

`Detector::Impl::SyncAddAndRefresh(text, type)`（0x100810794）：

```c
if (MaybeContainsChatEmoji(text)) return;
key = cityhash64(CombineHotWords(text));        // 8 字节键：**只存哈希，不存原文**
v = kv.Get(key);  count = v ? v.count + 1 : 1;
kv.Put(key, {u32 count, u32 now_sec, u8 type}); // 12 字节值
AutoCleanUp(key);
```

`AutoCleanUp`（0x100810bdc）：从游标起最多看 **20** 条，`last_time < now - 259200(3 天)` 的删掉，游标前移。
→ Detector 是一个**3 天滑动窗口的出现计数器**；推荐判据实际是「3 天内第二次出现且还不是热词」[推：阈值没有显式常数，靠 LookUp 是否命中]。
C API 里另有长度门：utf8 字数 16–100（`count_utf8 - 16 <= 0x54`，0x100664edc）且已见过时，额外记一条 `UserAssociationRecorder::SendOutRecordLong`。

`Classifier::CheckType` 的特征（正则在 0x10080ac4c 起，静态 `kRegexIdCard / kRegexBankCard / kRegexCarNumber / kLocationHotWord`）：
- 身份证 `(^\d{15}$)|(^\d{18}$)|(^\d{17}(\d|X|x)$)`；银行卡 `(^\d{16}$)|(^\d{17}$)|(^\d{18}$)|(\d{19}$)`；
  车牌：省份简称开头的正则（京津晋冀蒙辽吉黑沪苏浙…）；手机号：`UserHotWordHelper::PhoneNumberPrefix()` 前缀表；地点：`UserHotWordHelper::Locations()` 表。
- 返回的 type 枚举取值 1–8（1 地点、2 身份证、4 手机…），完整对应表见「未知」。

### 2. 证据表

| 结论 | 置信度 | 地址 / 字符串 |
|---|---|---|
| 三个 kind 0x80/0x94/0x82 分别配 Dynamic/Potential/Static | 确 | Maker::Impl ctor 0x10082f91c–0x10082fa24；LoadUserDict 0x100be962c/98bc/9cbc |
| 0x94 是 `!potential_ud!` | 确 | 0x100be9894 literal |
| 0x82 对应 UserEnglishDict | 推 | 0x100be9ccc 之后 0x100be9d50 构造 UserEnglishDict，签名带 strategy |
| AB 缺省 `128,2,100000,10,5,2592000,30000,1` | 确 | 0x10082ed4c |
| AB 每 600 秒最多重读一次 | 确 | 0x10082ecf0 常数 0x8bb2c97000 ns |
| 参数字段名与下限 | 确 | DebugInfo 0x100831318…；str.txt `py_target_cnt_ >= 20000` 等 |
| freq_th = base+level，time_th = base−level×4 天，level ≤ 5 | 确 | AdjustedStrategy 0x100831cc8（0xfffaba00 = −345600）、0x100831c54 `cmp w21,#4` |
| 自适应门 ±0.05 | 确 | 0x100831c34 常数 0x3fa999999999999a |
| target_rate = min(1, py_target/total) | 确 | 0x100831d28–0x100831d40 |
| 执行门 total ≥ py_cnt_threshold | 确 | 0x100831d90 |
| 自造词豁免看 item+0x18 / payload flags@+0x25 bit1 | 确 / 推 | 0x100831810；0x100765d3c |
| 每次学词触发，扫 10 个拼音键 | 确 | SyncAdd 0x100757594–0x100757638；ttr 0x100765790 |
| 混合大小写键的硬规则 7 天 / 3 天+freq<5 | 确 | 0x100765b6c–0x100765bbc |
| 纯 ASCII 字母的词视为脏值直接删（kind 0x80/0x94） | 确 | 0x100765bc8–0x100765c24，日志 `dirty value:` |
| Potential 常数 15d / 5 / 7d / 10 | 确 | 0x102e4a3d0 |
| roll_back 是撤销最近一次学习 | 推 | 0x100957050；`TryRevertLastRecord` |
| user_dict_clean_up.bin = 跨设备清空用户词库的同步状态 | 确/推 | 0x102ce81cd；ForceCleanUp 0x100e4e7d0 `ResetUserDict` |
| 联想数字条目 30/60/90 天分档、CTR<0.05 退休 | 确 | 0x100835d8c；0x1033712a8 |
| 热词检测键 = cityhash64，值 {count,time,type}，12 字节 | 确 | 0x100810840、0x1008108a0（w4=12） |
| 检测窗口 3 天、每次清 20 条 | 确 | 0x100810dd4（0x3f480）、0x100810df4（#0x14） |
| 推荐判据 = 已见过（≥2 次）且不在热词库 | 推 | AddFetch 0x10082a9e0–0x10082abb8 |
| 分类正则 | 确 | 0x10080ac4c / 0x10080aca0 / 0x10080acf4 |

### 3. 青简落点

现状（`crates/qingjian-learning/src/frequency_learner/`）：
- `FrequencyLearner` 六张表都**只有次数，没有最后使用时间**：`counts`（`user-frequency`：词\t次数）、`words`（user-words.tsv）、
  `choices`（超过 `MAX_CHOICE_ENTRIES = 50_000` 整体减半 `decay_choices`）、`english`（user-english.tsv：词\t次数，**只加不减**，todo.md 2026-09-24 条目）、
  `ngram`、`typos`。手动加的词目前没有单独标记（`learn_word` 记的是「词库没有、用户选过」的词）。
- 与 RefIME 的冲突与改法：
  1. **任务原文要的「次数 × 半衰期」不是 RefIME 的做法**。RefIME 是「freq < F 且 闲置 > T」双门槛 + 按目标保留率自适应收紧。
     建议照 RefIME：可解释（用户能看懂「30 天没用且用过不到 5 次」）、不需要浮点分、跟本地/云端包都同构。若坚持连续分，见步骤 7 的备选。
  2. 需要时间戳：给 `counts`、`english`、`words` 加第三/四列 `最后使用日`（Unix 天数 u32，`now/86400`）。旧文件没有这列 → 读入时填「加载当天」，
     等于从升级那天开始计闲置，不会升级即删词。
  3. 输入优先：照 RefIME 摊销——每次 `record`/`learn_english` 后从游标起检查 `check_next_n`（10）条，O(10)；不要在按键路径上全表扫。
     `BTreeMap` 的 `english`/`words` 天然有序可用 `range(cursor..)`；`counts` 是 `HashMap`，改成在 `flush()`（已经不在按键路径上）时做一次全表扫，
     或把它换成 `BTreeMap`。
  4. 不跨用户学习：全部本地，无冲突。云端分区学习表打包（任务原文）复用同一个 `RetentionPolicy::should_drop`，保证同构。
  5. 一个类型一个文件：新类型放 `crates/qingjian-learning/src/retention/` 目录。

**新增类型（签名）**

```rust
// crates/qingjian-learning/src/retention/mod.rs —— mod 声明与 re-export
// crates/qingjian-learning/src/retention/policy.rs
pub struct RetentionPolicy {
    pub min_count: u32,          // RefIME base 5
    pub max_idle_days: u32,      // RefIME base 30
    pub level_step_days: u32,    // RefIME 4
    pub max_level: u8,           // RefIME 5
    pub check_next_n: usize,     // RefIME 10
    pub start_at_entries: usize, // 达到多少条才开始真删；RefIME 10 万拼音键（主词库）/ 0（英文）
    pub target_entries: usize,   // 目标条数；RefIME 3 万；0 = 不自适应（Static）
    pub keep_manual: bool,       // RefIME 主词库 true、英文 false；青简两表都建议 true
}
impl RetentionPolicy {
    pub const STATIC_ENGLISH: Self;  // {5, 30, 4, 5, 10, 0, 0, true}
    pub const DYNAMIC_WORDS: Self;   // {5, 30, 4, 5, 10, start, target, true}，start/target 青简按 KB 预算自定
}
// crates/qingjian-learning/src/retention/entry_stat.rs
pub struct EntryStat { pub count: u32, pub last_day: u32, pub manual: bool }
// crates/qingjian-learning/src/retention/sweeper.rs —— 有状态：level、cur_level_scanned/kept、overall scanned/kept、cursor
pub struct Sweeper { policy: RetentionPolicy, level: u8, target_rate: f64, cur_scanned: u32, cur_kept: u32,
                     scanned: u32, kept: u32, cursor: Option<String> }
impl Sweeper {
    pub fn new(policy: RetentionPolicy) -> Self;
    pub fn adjust_target(&mut self, total: usize);                 // target_rate = min(1, target/total)
    pub fn should_perform(&self, total: usize) -> bool;            // total >= start_at_entries
    pub fn should_drop(&mut self, stat: EntryStat, today: u32) -> bool;  // 1.3 的 ShouldCleanUpCurrentItem
    /// 从游标起检查 check_next_n 条，返回要删的键；dry_run 时只计数不删
    pub fn step<'a>(&mut self, entries: impl Iterator<Item = (&'a str, EntryStat)>, today: u32, dry_run: bool) -> Vec<String>;
}
```

`FrequencyLearner` 改动：`english: BTreeMap<String, (String, u32, u32 /*last_day*/, bool /*manual*/)>`（或换成 `EnglishEntry` 结构体，单独文件），
`learn_english` 里更新 `last_day` 后调 `sweeper_english.step(...)` 删除返回的键并 `rebuild_english()`；`counts`/`words` 同理但在 `flush()` 里做。
偏好设置「能看到条数」：已有 `english_count()` / `word_count()` / `len()`，再加 `RetentionReport { dropped_last_flush, level }` 给壳显示。
常数初值：照 RefIME（5 次 / 30 天 / 每级 4 天 / 最多 5 级 / 每步 10 条），**用 `apps/cli --replay` 冻结日志重扫**（docs/notes/constant-sweep.md 记录）。

**热词检测在青简的对应**：青简的 `CustomPhrase`（`crates/qingjian-core/src/custom_phrase.rs`）就是 RefIME 的热词。
可选做一个 `PhraseDetector`（`crates/qingjian-learning/src/phrase_detector/`）：键只存 `blake3/foldhash(规范化文本)`，值 `{count, last_day, kind}`，
3 天窗口、每次加入顺手清 20 条过期的；同一段长文本（16–100 字）3 天内第二次上屏/粘贴、又不在自定义短语里 → 壳提示「存为自定义短语」。
分类正则（身份证/银行卡/车牌/手机）只用来给建议的码与提示文案，**不上传**。这是独立小功能，C1 可以先不做。

### 4. 复刻步骤

1. 在 `qingjian-learning` 加 `retention/{mod,policy,entry_stat,sweeper}.rs`，纯函数 + `Sweeper` 状态机，照 1.3 伪代码实现，先写单测。
2. 文件格式加列：`user-english.tsv` → `词\t次数\t最后使用日[\t手动]`，`user-words.tsv`、词频文件同理；解析容忍旧两列（last_day = 今天，manual = false）。
   保存时写新格式。同步改 `docs/notes/crate-notes.md` 的 qingjian-learning 段与 `docs/user/help/data-and-logs.md`（数据文件格式）。
3. `learn_english` / `record` / `learn_word` 更新 `last_day`；手动加词入口（若有，删候选快捷键的反向入口）设 `manual = true`。
4. `learn_english` 末尾：`sweeper.adjust_target(len)`；`step(english.range(cursor..).chain(english.range(..cursor)), today, !should_perform)` 删键，O(10)。
5. `flush()` 前对 `counts`、`words` 跑一遍 `step` 直到绕一圈（不在按键路径上），每次 flush 最多绕一圈（对应 RefIME laps ≤ 1）。
6. `apps/cli --replay` 跑清理前后首选命中率；用「把时钟拨到 +60 天」的回放验证体积上限生效、命中率不降。
7. 备选（若产品坚持连续分）：`score = count × 0.5^(idle_days / half_life)`，超过容量按分数淘汰；half_life 初值取 30 天，同样只在 flush 时做。不推荐，理由见上。

单测清单（`today = 1000`）：

| 输入 | 期望 |
|---|---|
| Static：`{count 4, last_day 969}`（闲置 31 天） | drop |
| Static：`{count 4, last_day 970}`（闲置 30 天，边界不删，RefIME 是 `>`） | keep |
| Static：`{count 5, last_day 0}` | keep（频次达标） |
| Static：`{count 1, last_day 0, manual true}`，keep_manual = true | keep |
| Dynamic：target 1000、total 10000（rate 0.1），喂 11 条全 keep | level 0 → 1，freq_th 6、idle 26 天 |
| Dynamic：level 5 后继续全 keep | level 不再升（≤ 5） |
| Dynamic：level 1，保留率 < rate − 0.05 | level 回 0 |
| `should_perform(total < start)` | false，step 的 dry_run 不删任何键 |
| step 从游标起只看 10 条，第二次调用接着第 11 条 | 游标推进、到尾绕回开头 |
| 旧格式两列文件读入 | last_day = today，不触发删除 |
| 联想分档（若做）：count 1 闲置 31 天 drop；count 3 闲置 59 天 keep；count 6 闲置 91 天 drop | 如左 |

### 5. 未知

- `UserDictV2::Impl +0x1f0` 是否确是「拼音键总数」：只看到 atomic load 后当 `total_py` 传入；下一步看 `ReLoadIndex`（0x10075d8e8，日志 `user dict total count is:`）写它的地方。
- `scanned/reserved` 的计数单位（按拼音键还是按词条）：Impl +0x124 每个键 +1，reserved 的递增点没逐行确认；看 0x100765d74 之后 keep 分支。
- `HyperParams::UserDictCleanParams()`（0x100733438）的数据来源（随包 hyperparams 文件还是云配置），决定 kind 0x80 在 AB 未下发时的真实参数。
- 热词推荐是否还有次数阈值（AddFetch 0x10082ac5c `cmp x9, x8` 一处比较未解）；`CheckType` 返回值 1–8 与类型的完整对应；
  壳侧谁调 `wxime_add_fetch_user_hot_word_detection`（Swift `WXIMEUtil queryHotWordRecommand:` 附近，0x100035c28）。

---

## C2 完全离线（LocalMode）

### 1. RefIME 怎么做的

#### 1.1 开关存哪 [确]

- MMKV 实例 `settingMMKV`（全局 0x10335b8f8，`refime.settings` [推]），键 **`localModeEnabled`**（字符串 0x102c24fd0），
  读法一律 `[settingMMKV getBoolForKey:@"localModeEnabled" defaultValue:NO]`（如 0x1003a1980），**缺省关**。
- 写入方：设置程序经 `AppCommunicator.recieveFromSettings(_:)`（0x100386ab0）收到后写入并调 `LocalMode.statusChanged()`；
  各「退出本地模式」按钮（AI 问答 / 翻译 / 文件传输的空态页、关于窗口「保持最新」、语音的独立模式弹窗）写 false 后同样调 `statusChanged()`。
- 通知名 `com.vendor.RefIME.localModeStatusChanged`（0x102c5c520），`FileTransferSessionViewController` 等订阅。

#### 1.2 状态切换时集中做一次「拆/装网络」[确]

`LocalMode.statusChanged()`（0x1003a1a20，`Module/Common/LocalMode.swift`）：

```swift
log("statusChanged()")
if localModeEnabled {
    Engine.closeNetwork()            // 0x10023c7ec → 尾调 wxime_network_logout：引擎内的云请求全停
    Business.stopAutoReport(); Business.stopReceivePush()
    SentrySDK.close()                // 崩溃上报也关
    <某单例>.close() → 成功则 AIQuestPanel.hide(...)
} else {
    if !Business.hasInit() { Business.setUpBusiness() }
    Business.startReceivePush()
    if !SentrySDK.isEnabled() { StartSentry() }
    Business.startAutoReport()
    Engine.setUpNetwork(useDebug:)
}
UpdaterController.resetSparkleAutoCheckUpdate()
(NSApp.delegate as? AppDelegate)?.updateAboutMenuItem()
InputSessionCache.recreateSessions()    // 重建会话，让引擎按新的网络状态开会话（createSession 里也查开关）
TranslateWindow.translateContentView.checkStatus(); AboutWindowController 刷新
NotificationCenter.post(localModeStatusChanged)
```

`LocalMode.temporaryEnableNetWork()`（0x1003a1928）：开着本地模式时**临时**起 Business 与推送通道，只被 `Feedback.openFeedback()` 调用——用户主动发反馈时破例联网。

#### 1.3 每个联网点各自查开关（散落，不是集中守卫）[确]

字符串 `localModeEnabled` 的全部引用点（`add x?, x?, #0xfd0` 扫描结果），每处都是现场读 MMKV：

| 函数 | 跳过时做什么 |
|---|---|
| `CosUploadManager.upload(_:completion:)` 0x10010e530 | LogWarn `[CosUpload] skip reason=localMode biz=`，`completion(.failure(CosUploadError.localMode…))`，**不入队** |
| `CosUploadError.description` 0x10010d6f0 | 错误文案 |
| `VoiceReportManager.scheduleAudioUpload(snapshot:)` 0x1004a1158 | LogInfo ` skip reason=localMode base=`，然后照常 `VoiceAudioFileStore.ensureDirectory` 落本地 [推：本地文件保留] |
| `NativeVoiceManager.openOpusWriterForSession()` 0x100463b34 | `[VoiceOpus] skip open reason=localMode`，不开录音压缩写入 |
| `VoiceInputCoordinator.validateStartAdmission(source:phase:)` 0x1004f5b3c / `showStandaloneModeAlert` | 拒绝语音输入、弹「独立模式」提示（可一键退出本地模式） |
| `BIZ.request(_:subCmd:data:timeout:success:failure:)` 0x1002bb96c | 直接走失败回调 [推] |
| `BIZ.report(...)` 0x10027c98c / 0x100287910、`BIZ.reportVoiceInputContentAction` 0x1002896fc、`BIZ.queryServerTimeIfNeed` 0x1002bc374 | 不上报 |
| `Engine.getSuggestions(by:isASCIIMode:)` 0x10022cc38 | 开着就直接跳到函数尾（不取云端建议） |
| `Engine.createSession(...)` 0x100235f3c / 0x1002363bc、`Engine.setSessionSupportSticker` 0x100236cd8、`Engine.setUpEngine` 0x10023badc | 会话参数里关掉联网能力 [推] |
| `Engine.searchAIQuest(...)` 0x100237ee0 / 0x10023854c；`AIQuestDSContentView.askAI` 0x100350f00 等 AI 视图 | 不请求，显示 `LocalModeEmptyView`（日志 `in LocalMode, return`） |
| `TranslateViewModel.requestTranslate(_:)` 0x1003f7c00 / 0x1003f7d70、`TranslateContentView.checkContentStatus` | 不翻译，显示 `TranslateLocalModeView` |
| `FileTransferMainViewController.viewWillAppear / applyLocalModeStatus / onLocalModeStatusChanged`、`FileTransferSessionViewController.onLocalModeStatusChanged` | 传输中切到本地模式：`[FileTransfer][Session] local mode enabled during transfer, force close` |
| `AppDelegate.launchInitServices()` 0x10015ae5c、`thingsAfterServerConnect` 0x100153474、`uinChanged` 0x1001585d8 | 启动时不 `setUpBusiness` |
| `AppDelegate.createMenu / updateAboutMenuItem`、`AboutWindowController.*`、`CheckUpdateStatusButton.updateViews` | 菜单与关于页显示「本地模式」 |
| `UpdaterController.feedURLString(for:)` 0x100305aac | **更新检查不停**，改用独立源 `<厂商域名>`（0x102c52b80） |
| `UpdateConfig.doCheckIfUpdateURLChange()` 0x10022722c | `[AppUpdate] check url change: LocalMode enabled` |
| `UpdaterController.resetSparkleAutoCheckUpdate / updater(_:didAbortWithError:)`、`UpdaterUserDriver.*` | 调整自动检查与报错文案 |
| `Feedback.recevieFeedbackNotif` 0x10036af08 | 反馈相关 |
| `GlobalSetting.setDefaultValues()` 0x1004286c0 | 注册缺省 false |
| `main()` 0x100059554 | 启动早期读一次 |
| `AIQuestSearchView / AIQuestDSInputView / AIQuestShareBar` 多处 | UI 置灰 |

结论：**判断是散落的**（约 60 处、30 多个函数各自读 MMKV），**动作是集中的**（`statusChanged` 一次性拆掉长连接、上报、推送、Sentry、引擎网络）。
跳过时的队列：COS 上传直接失败、不排队；语音音频照常存本地 [推]；引擎内的云请求由 `wxime_network_logout` 整体停掉。
`localmode/updates.xml` 说明本地模式下**更新检查仍然联网**（单独的更新源，大概率只给本地模式用户推送）。

### 2. 证据表

| 结论 | 置信度 | 地址 / 字符串 |
|---|---|---|
| 键 `localModeEnabled`，`settingMMKV getBoolForKey:defaultValue:NO` | 确 | 0x102c24fd0；0x1003a1980 selector；全局 0x10335b8f8 = `settingMMKV` |
| statusChanged 拆装网络的调用序列 | 确 | 0x1003a1cd8–0x1003a1f38 |
| Engine.closeNetwork → `wxime_network_logout` | 确 | 0x10023c93c |
| temporaryEnableNetWork 只给反馈用 | 确 | 唯一调用者 `Feedback.openFeedback()` |
| COS 上传跳过即失败 | 确 | 0x10010e578–0x10010e5e4（LogWarn + swift_allocError） |
| 语音上传跳过后仍落本地目录 | 推 | 0x1004a1208 → 0x1004a122c `ensureDirectory` |
| 更新源换成 localmode/updates.xml | 确 | 0x100305aec–0x100305afc |
| 引用点清单 | 确 | `xref.py` 扫 0x102c24fd0 共 61 处 |

### 3. 青简落点（素笺联网点）

素笺的联网全部在 Rust 里，Swift 侧没有 `URLSession`（`grep` 无结果）：
- `cloud/crates/qingjian-cloud-client/src/client/{mod,account,pair,reply}.rs`：`Client`（ureq）。**所有请求都经过私有 `fn url(&self, path)`**，
  方法：`whoami / push_clip / push_materials / register_contact / processor / delete_clip / events / chat / push_learning / learning / config / put_config / push_input_log / input_log / clear_input_log / stream`，
  账号与配对的 `sign_in_apple / email_* / account / put_consent / revoke_session / sign_out / delete_account / create_space / pair_*`。
- 调用方：`qingjian-cloud-bridge/src/upload.rs`（素材 2B 与输入日志上传，每轮重读 `cloud.toml`）、`lib.rs:454/486`、`session/cloud.rs`（润色/云联想代理）、
  `account/*`、`qingjian-cloud-client/src/data_sync/`（学习数据、`config.toml`、输入日志同步）、`clipboard_sync/`（剪贴板，带 `Outbox` 离线队列，最多 100 条丢最旧）、
  `qingjian-cloud-mac/src/service/account.rs`。
- Core 侧另有 `crates/qingjian-predict/src/chat_client.rs`（云联想 / 释义兜底）、`crates/qingjian-update/src/{download,index/fetch}.rs`（检查更新，Mac）。
- 开关放哪：`CloudConfig`（`cloud/crates/qingjian-cloud-bridge/src/cloud_config.rs`，App 与键盘共用的 `cloud.toml`，已有 `llm/memory/logs/sync/clipboard` 五个布尔）
  新增 `offline: bool`（缺省 false）。**不能**放在登录态清理会清掉的字段里：`clear_session` 要保留它。

与青简约束：
- 「设置项不是模式」：`offline` 是布尔设置，入口在 iOS App「我」（`cloud/ios/App/Me/`）与键盘长按菜单；不新增输入模式。
- 「每个联网点统一过一个守卫」：RefIME 是散落检查，青简应该做集中守卫——在 `Client` 里加一道闸，比 RefIME 更不容易漏。
- Core 平台无关：`qingjian-predict` 的 `ChatClient` 也要能被关，经 `PredictConfig` 传 `offline`，不在壳里拦。

**新增（签名）**

```rust
// cloud/crates/qingjian-cloud-client/src/offline.rs —— 进程内开关，键盘与 App 各自从 cloud.toml 同步
pub fn set_offline(on: bool);
pub fn is_offline() -> bool;                         // AtomicBool
// cloud/crates/qingjian-cloud-client/src/client/mod.rs
fn url(&self, path: &str) -> Result<String, ClientError> {   // 现有 url() 改成返回 Result，所有方法 `?` 一下
    if offline::is_offline() {
        tracing::info!(path, "skip reason=offline");
        return Err(ClientError::Offline);
    }
    Ok(format!("{}{path}", self.base))
}
// error.rs
#[error("offline mode")] Offline,                  // 调用方当「暂不重试、保留队列」处理，不计失败次数、不退避
// cloud_config.rs
pub offline: bool,
// qingjian-predict：PredictConfig.offline 或调用前查同一个 flag；qingjian-update：Mac 菜单项「检查更新」照常允许（用户主动），自动检查跳过。
```

队列保留：`upload.rs` 素材只在服务端 202 后才标已传 → `Offline` 时什么都不改即保留；`InputLogSync` / `LearningSync` 的进度游标同理不前移；
`clipboard_sync::Outbox` 已经落盘，`Offline` 时不出队（注意它 100 条上限会丢最旧，文档写明）。关掉离线后由现有的踢脚/轮询自然补传。
界面「离线中」：`CloudConfig.offline` 镜像给键盘，键盘 `IdleBar` 显示；App「我」页显示状态。日志统一 `skip reason=offline path=…`，便于用户抓包对照。

### 4. 复刻步骤

1. `qingjian-cloud-client` 加 `offline.rs` 与 `ClientError::Offline`；`Client::url()` 改返回 `Result`，全部方法补 `?`（编译器会逐个指出漏网的）。`stream()` 同样过闸。
2. `CloudConfig` 加 `offline`，`load_for_keyboard` / App 读到后 `offline::set_offline(..)`；每轮上传/同步开始时重读（upload.rs 已每轮重读配置）。
3. 上传与同步循环：遇 `Offline` 不退避、不计错、不改进度，直接睡到下一轮。
4. `qingjian-predict`：`CloudPredictor` / `CloudGlossFiller` 在发请求前查 offline（Core 不依赖 cloud crate，所以经 `PredictConfig` 字段传入，壳从 cloud.toml 同步）。
5. iOS：「我」页开关（写 `cloud.toml` 的 `offline`）、键盘长按菜单项；离线时状态条显示「离线中」。同步改 `docs/user/cloud/index.md`。
6. 验证：开关打开后让用户自己抓包零请求；日志里每个被拦的请求都有 `skip reason=offline`；关掉后队列补传。

单测：
| 输入 | 期望 |
|---|---|
| `set_offline(true)` 后调 `Client::whoami()` | `Err(Offline)`，不发请求（用 test_support 的假服务器断言 0 次命中） |
| 离线时跑一轮素材上传 | 素材仍标「未上传」，游标不变 |
| 离线时 `Outbox::push` 3 条 | 落盘 3 条，不出队；`set_offline(false)` 后一轮补发 3 条 |
| `CloudConfig::clear_session()` | `offline` 保留 |
| 旧 `cloud.toml` 无 `offline` 字段 | 读成 false |

### 5. 未知

- `statusChanged` 里 `<某单例>.close()`（0x1003a1d40）是哪个对象（AI 问答长连接 / 文件传输 P2P？）；看 0x1003a1d20–0x1003a1d40 的类引用。
- `Engine.createSession` 在本地模式下具体改了哪些会话字段（0x100235f9c 之后的分支）。
- `BIZ.request` 在本地模式下构造的错误对象内容（0x1002bb9c0 起的 0x4c00 常数）。

---

## C3 中英 / 数字之间自动空格（smartSpace）

### 1. RefIME 怎么做的

#### 1.1 开关与缺省 [确]

| 设置 | 存储键（UserDefaults 套件，`[x26+0x860]+0x10` 处的对象） | 缺省 |
|---|---|---|
| `GlobalSetting.smartSpaceEnglish`（中英之间） | **`settingfunction_smartSpace`**（0x102c36cb0，26 字节） | 键不存在时 `setDefaultValues` 写 **true**（0x100427f28） |
| `GlobalSetting.smartSpaceDigit`（中文与数字之间） | `smartSpaceDigit`（内联串） | 键不存在时取 `settingfunction_smartSpace` 的值（老用户迁移）[推]（0x100427f78 起） |
| `smartSpaceChEngTipsShowed` / `smartSpaceChNumTipsShowed` | 首次触发的提示是否看过 | — |

日志 `toggle smartSpaceDigi to [`；埋点 `closeChEngSmartSpace` / `closeChNumSmartSpace`。语音上屏（`VoiceTextEmitter.commitFinalText / showIntermediateText / formatFinalDisplayText`）也读同一开关 [确：xref]。
两个开关都关 → `smartSpaceText` 直接原样返回（0x10011f05c）。

#### 1.2 字符分类 `Char(Character?)`（0x1003151ec）[确]

| 类 | 判据 |
|---|---|
| 0 其他 | nil、空格、标点、其他一切 |
| 1 英文 | `alphabetSet` = `a–z A–Z`（0x102c36620） |
| 2 数字 | `digitSet` = `0–9`，**或** `kSpecialSymbolSet` = `% ‰ ° ℉ ′ ″ ℃`（0x102c54870） |
| 3 中文 | `range(of: "^[一-龥]+$", options: .regularExpression)` 命中，即 U+4E00…U+9FA5 |

#### 1.3 判定 `SmartSpace.isNeedSpace(a, b)`（0x100315400）[确]

对称真值表（a = 左边字符，b = 右边字符；跳转表 0x102e1fff0 = `0f 00 04 09`）：

| a \ b | 其他 | 英文 | 数字 | 中文 |
|---|---|---|---|---|
| 其他 | – | – | – | – |
| 英文 | – | – | – | **English 开** |
| 数字 | – | – | – | **Digit 开** |
| 中文 | – | **English 开** | **Digit 开** | – |

任一为 nil 返回 false。读设置的对象为 nil 时两开关都按 true。

#### 1.4 上屏流程 `InputController.smartSpaceText(_:replaceRange:)`（0x10011ef10）[确]

调用者：`emitInput(str:checkSmartSpace:replaceRange:)` 的主线程闭包（0x10011e390），条件 `isActive && client != nil && !str.isEmpty && checkSmartSpace`。
几乎所有上屏都传 `checkSmartSpace: true`：候选上屏 `candidateSelected`、`tryMapPunctuation`、`inputSymbol`、`emitInputToScreen`、`forceEmitPendingImmediately`、
`inputText` 的多数分支、热词与剪贴板面板；传 false 的：`inputText` 两处、`AccessibilityUtil.replaceSelectedText`、`ListPrefix.*`（列表前缀自动补）。

```swift
func smartSpaceText(_ text: String, replaceRange: NSRange) -> (String, NSRange) {
    guard english || digit else { return (text, replaceRange) }
    var ctx = contextText                         // ContextText：+0x18 isNoContext, +0x20 before, +0x30 after
    let sel = getSelectedRange()
    if ctx.before.isEmpty || (sel.location != 0 && (marked == NSNotFound || 光标与缓存不符)) {
        log("trigger contextText refetch, from smartSpaceText")
        if isChatSession && !featuresMMKV["disable_fetch_xchat_context"] && appCommunicator.chatSupportFetchContext {
            log("[XChat] cancel refetch context, QT not support now")       // 聊天应用 Qt 版不同步重取
        } else {
            contextText.refetch(from: 1, limitLen: 512, force: false, withAfter: false, extInfo: [:])  // 同步重取前文
        }
        ctx = contextText
    }
    var out = SmartSpace.addSpace(text)            // ① 上屏文本**内部**相邻字符也按同表补空格（0x100314bd4）
    if ctx.isNoContext || sel.location == NSNotFound {      // 拿不到宿主前文
        if isNeedSpace(self.lastInputChar, out.first) { out = " " + out }   // ② 用自己记的上一次上屏的末字符
        lastKeyDownDelete = false; return out
    }
    if lastKeyDownDelete {                         // 上一键是退格：不补前后空格（只保留①）
        log("disable smartSpaceText for lastKeyDownDelete"); lastKeyDownDelete = false; return out
    }
    let after = ctx.after
    if !after.isEmpty, let f = after.unicodeScalars.first, !CharacterSet.newlines.contains(f),
       !AccessibilityManager.shared.cachedPermission {          // 有后文但没辅助功能权限：只补前空格
        log("smartSpaceText AXIsProcessTrusted is false")
        if isNeedSpace(ctx.before.last, out.first) { out = " " + out }
        lastKeyDownDelete = false; return out
    }
    if ctx.before.contains("#") { goto done }                    // 前文含 #（话题/Markdown 标题/注释）：前后都不补
    if isNeedSpace(ctx.before.last, out.first) { out = " " + out }   // ③ 前空格
    if isNeedSpace(after.first, out.last) {                      // ④ 后空格：补在文本尾部，再发一个 ← 键让光标停在空格前
        out += " "; postCursorLeftEvent()                        // keyCode 123
    } else if after.count >= 2, after.first == " ", !isNeedSpace(out.last, after[1]) {
        postDeleteNextEvent()                                    // ⑤ 后面已有空格但现在不该有：发 Forward Delete（keyCode 117）删掉
    }
done:
    LogInfo(...); lastKeyDownDelete = false
    return (out, replaceRange)
}
```

- 光标前字符来源：`ContextText`（AX / 客户端 `attributedSubstring`，`refetch` 最多 512 字）为主；拿不到时退回 `lastInputChar`（InputController 自己记的上一次上屏末字符，
  `inputCommand` 等 7 处清空，`deactivateServer` 清空）。
- 已有空格不重复加：空格属「其他」类，`isNeedSpace(' ', x)` 恒 false。
- **没有 bundleID 黑名单**，也没有终端特判；唯一的场景开关是「前文含 `#`」、「上一键退格」、「聊天应用 Qt 版不重取前文」。

### 2. 证据表

| 结论 | 置信度 | 地址 / 字符串 |
|---|---|---|
| 两个开关键与缺省 true | 确 | 0x10011efb0 / 0x10011f010 读；0x100427ee8–0x100427f30 写缺省 |
| Digit 缺省继承 English | 推 | 0x100427f78 之后读 `settingfunction_smartSpace` |
| 字符四类与集合内容 | 确 | 0x1003151ec；0x102c36620；0x100112468 内联 `0123456789`；0x102c54870；正则内联 0x1003152e8 |
| 对称真值表 | 确 | 0x10031555c–0x100315598，跳转表 0x102e1fff0 |
| 文本内部也补空格 | 确 | `SmartSpace.addSpace` 0x100314bd4 逐字符循环 + `append(" ")` 0x10031510c |
| 前文含 `#` 前后都不补 | 确 | 0x10011f89c、0x10011f968 `StringProtocol.contains("#")` |
| 后空格 + ← 键；多余空格 Forward Delete | 确 | 0x10011fa3c（closure keyCode 0x7b）、0x10011fd6c（closure keyCode 0x75） |
| 无 AX 权限只补前空格 | 确 | 0x10011f7c0–0x10011f820，`smartSpaceText AXIsProcessTrusted is false` |
| lastKeyDownDelete 时只保留① | 确 | 0x10011f5f4–0x10011f768 |
| 无前文用 lastInputChar | 确 | 0x10011f4d0–0x10011f530（`+0x778` = `lastInputChar`） |
| checkSmartSpace 的调用方取值 | 确 | 24 个 `bl emitInput` 调用点前的 `mov w2,#…` |

### 3. 青简落点

- 现状：壳在每次查询前把宿主光标前文交给 Core：`Engine::set_rescoring_context(Option<String>)`（`crates/qingjian-core/src/engine/rescoring/mod.rs`，
  Mac `apps/macos/src/imk/controller/display.rs:27`、Windows `server/src/dispatch/rescore`、iOS `cloud/crates/qingjian-cloud-bridge/src/session/mod.rs:333`），
  拿不到时 Core 有 `self.history.recent(n)`（本会话最近上屏字符）兜底——**正好对应 RefIME 的 ContextText + lastInputChar 两级**。
  上屏文本由 `Engine::commit(&Candidate) -> String`（`engine/commit/mod.rs`）等返回给壳，壳 `client.insert_text(&text)`。
- 改法（Core 平台无关）：在 Core 加纯函数与开关，**只做前空格与文本内部空格**（RefIME 的①②③）；后空格要发 ← / Forward Delete 键，属于平台动作，
  且「改光标」容易出错，第一版不做（或返回一个 `trailing_space` 提示由壳决定，iOS 用 `adjustTextPosition(byCharacterOffset: -1)`）。
- 配置：`[general] auto_space`（布尔，缺省 **关**，任务要求；RefIME 缺省开）。是否拆成中英 / 中数两个开关：建议照 RefIME 拆两个
  `auto_space_english`、`auto_space_digit`，或一个布尔 + 内部两类都管——按青简「设置项少」的取向，建议**一个开关同时管两类**，数字类单测保留。
- 场景禁用：RefIME 没有黑名单；青简已有 `[apps]` 分节与终端/编辑器缺省名单（`crates/qingjian-platform/src/config/apps.rs` 的
  `DEFAULT_ENGLISH_CANDIDATES_OFF_*`），任务要求代码/终端不加 → 新增 `AppsConfig::auto_space_off`，缺省复用同一份名单；iOS 拿不到宿主 bundle，只给开关。
  这个判断在壳里做（壳知道应用标识），但**只把「本次是否启用」传给 Core**（`Engine::set_auto_space(bool)`），空格逻辑仍全在 Core。
- 字符分类：中文用 `qingjian_core::sentence::is_han`（比 RefIME 的 U+4E00–9FA5 宽，含扩展区，更合理）；英文 ASCII 字母；
  数字 `0–9` 加 `%‰°℉′″℃`（照 RefIME）。
- 一个类型一个文件：`crates/qingjian-core/src/auto_space/{mod.rs, char_class.rs}`。

```rust
// crates/qingjian-core/src/auto_space/char_class.rs
pub enum CharClass { Other, Latin, Digit, Han }
impl CharClass { pub fn of(c: char) -> Self; }
// crates/qingjian-core/src/auto_space/mod.rs
pub fn needs_space(left: Option<char>, right: Option<char>) -> bool;   // 1.3 的对称表（两类都开）
pub fn inner(text: &str) -> String;                                     // RefIME addSpace：文本内部补
pub fn apply(before: Option<char>, before_has_hash: bool, text: &str) -> String;  // 内部 + 前空格；before 含 '#' 时只做内部？见下
// engine：pub fn set_auto_space(&mut self, on: bool); commit / commit_raw / 标点上屏的出口统一过 `self.space_commit(text)`
```

`before` 取 `rescoring_before` 的最后一个字符，没有就取 `history.recent(1)`；`#` 规则：RefIME 是前文**任意位置**含 `#` 就整段不补（含内部？——内部补空格①在 `#` 检查之前已做，所以①仍生效）。
青简建议只看「当前行」含 `#`（前文最多 512 字里取最后一个换行之后），避免一篇文档里出现过一个 `#` 就永久失效；这是对 RefIME 的有意偏离，写进 docs/design。
退格规则：上一键是退格时不补前空格（RefIME 的 `lastKeyDownDelete`）——Core 知道按键序列，可在 Engine 里记 `last_key_was_backspace`。

用户文档：`docs/user/settings/preferences.md` 加一项；`docs/user/input/english-mode.md` 中英混输段落提一句；按键行为没变，`keys.md` 不用改。

### 4. 复刻步骤

1. `crates/qingjian-core/src/auto_space/`：`CharClass::of`、`needs_space`、`inner`、`apply`，先写单测。
2. `GeneralConfig` 加 `auto_space: bool`（缺省 false）；`AppsConfig` 加 `auto_space_off: Vec<String>`（缺省 = 现有终端/编辑器名单），配套 `auto_space_off(app)`。
3. `Engine` 加 `auto_space: bool` 与 `set_auto_space`、`last_key_was_backspace`；`commit`、`commit_raw`（原样上屏的英文串也要过：「中文」后原样上屏 `hello` → 前补空格）、
   标点上屏路径都在返回前过 `space_commit`。表达式/算式结果、自定义短语（`CandidateKind::Custom`，用户写死的文本）不过——对应 RefIME 传 `checkSmartSpace:false` 的那类。
4. 壳：Mac / Windows / Linux 每次焦点变化按应用标识算 `general.auto_space && !apps.auto_space_off(app)` 调 `set_auto_space`；iOS 只看开关。
5. 偏好设置加勾选框；`docs/user/settings/preferences.md` 同步。
6. 端到端：mac 用 `osascript` 往 TextEdit 发按键读回（CLAUDE.md 的验证方式）：已有「聊天应用」，再输 `chat` 原样上屏 → `聊天应用 chat`。

单测清单（`apply(before, has_hash, text)`）：

| before | text | 期望 |
|---|---|---|
| `'信'`（中） | `"Chat"`（英） | `" Chat"` |
| `'t'`（英） | `"中文"` | `" 中文"` |
| `'号'` | `"3"` | `" 3"` |
| `'%'`（数字类） | `"的"` | `" 的"` |
| `' '`（已有空格） | `"Chat"` | `"Chat"` |
| `'，'`（全角标点） | `"Chat"` | `"Chat"` |
| `'a'` | `"b"` | `"b"`（英英不补） |
| `'1'` | `"2"` | `"2"` |
| `None`（无前文且无历史） | `"Chat"` | `"Chat"` |
| 任意 | `"用iPhone拍"`（内部） | `"用 iPhone 拍"` |
| `'中'`，`has_hash = true` | `"abc"` | `"abc"`（只做内部） |
| 开关关 | `"Chat"` after `'信'` | `"Chat"` |
| 上一键退格 | `"Chat"` after `'信'` | `"Chat"` |

### 5. 未知

- `ContextText.refetch(from:1,…)` 的 `RefetchSource` 枚举 1 是哪种来源（AX 还是 IMK client），以及它在 512 字截断时取的是光标前多少字——青简不需要。
- `emitInput` 闭包里更新 `lastInputChar` 的确切语句（推测为 `str.last`）：在 0x10011e804 之后，未逐行确认。
- 设置对象 `[0x1033b5860]+0x10`（`G.setting`）是标准 UserDefaults 还是 App Group 套件，影响「键名」在磁盘上的位置，不影响复刻。
