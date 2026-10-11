# A3 会话包装层（SessionCreator 装饰器链）

对象：WeType 2.2.3 macOS arm64，`wxime::core::SessionCreator::operator()(const _wxime_session_config*, const ExtraOption&) &`
（0x1006afbf8–0x1006bc13c，源文件 `ime/wxime/core/session_creator.cpp`）。纯静态读：`full.asm` 反汇编 + `__DATA_CONST` 读虚表 +
`__swift5_fieldmd` 读 C 结构体字段名。工作脚本在 /tmp/a3work/scr/（`topo.py` 求构造顺序、`guards.py` 找跳过条件、`vt.py` 读虚表、
`fmd2.py` 读 Swift 字段描述符），中间产物 /tmp/a3work/topo.txt、guards.txt。

更正 wetype-reverse.md 1.1 节的两处：
- 那里按**代码布局**列的顺序（Wubi → Stroke → … → PinyinV2Session）不是套的顺序。编译器把拼音核心等块放到了函数尾部再跳回，
  按控制流求出的真实顺序是 **PinyinV2Session 最先建、在最里层**，其后逐层往外套，最外是 LogTraceID / WatchDog / BatchEmit。
- 「链的嵌套方向」现在是 [确]：当前会话放在栈槽 `sp+0x850`，每层构造前把它 move 进 `InitContext`，构造后写回同一槽（见 1.3）。

---

## 1. WeType 怎么做的

### 1.1 ISession 接口（`wxime::base::ISession`，虚表 0x103175080）[确]

对象布局：`+0x0` vptr；`+0x8 … +0x808` 16 个 `lsignal::signal<…>`，每个 0x80 字节（事件**往外推**的通道）；`+0x810`、`+0x818`
两个指针字段（`+0x818` 缓存 `ResourceMgr::Instance()`，见 0x100c82a98）。

虚表槽位（下标从虚表起点算，vptr 指向下标 2；括号内是 `ldr x?, [vptr, #off]` 里用的字节偏移）：

| 下标 | 偏移 | 方法 | ISession 自己的实现 |
|---|---|---|---|
| 2/3 | 0x00/0x08 | `~ISession()` | 有 |
| 4 | 0x10 | `ProcessInput(string_view, const AuxiliaryData*)` | 纯虚 |
| 5 | 0x18 | `PendingInput() const`（返回 string，组句串） | 纯虚 |
| 6 | 0x20 | `DropLast(DropType, uint8_t, const AuxiliaryData*)`（退格/清空，返回 DropStatus） | 纯虚 |
| 7 | 0x28 | `SearchCandidates()`（触发出候选，结果走信号） | 纯虚 |
| 8 | 0x30 | `SelectCandidate(sv, sv, sv, const AuxiliaryData*)` | 空 |
| 9 | 0x38 | `PreSelectCandidate(sv, sv, sv, const AuxiliaryData*)` | 空 |
| 10 | 0x40 | `GetMostLikelySequel()`（联想） | 空 |
| 11 | 0x48 | `SetOption(SessionOptions, bool)` | 空 |
| 12 | 0x50 | `SetOption(SessionOptions, variant<monostate, EnFontOpt, UserHardFixWordOpt, UserDictsModifierOpt, EmojiSelectionReport, u64, string>)` | 空 |
| 13 | 0x58 | `GetOption(SessionOptions) const` | 空 |
| 14 | 0x60 | `CancelPendingJobs()` | 断开自己 16 个信号 |
| 15 | 0x68 | `SetSyllableHint(sv, sv, const AuxiliaryData*)` | 空 |
| 16 | 0x70 | `SetTextAroundCursor(const TextAroundCursor&, TextAroundCursorChangeReason)` | 空 |
| 17 | 0x78 | `PreSetTextAroundCursor(sv, sv)` | 空 |
| 18 | 0x80 | `SetKeyBoardLayout(const KeyBoardLayout&)` | 空 |
| 19 | 0x88 | `SetInternalCursor(uint)` | 空 |
| 20 | 0x90 | `ProcessInput(Span<Point>)`（手写点） | 空 |
| 21 | 0x98 | `LookUpBottomSession(LookUpBottomSessionType)` | 空 |
| 22 | 0xa0 | `GetCloudResult(const CloudSearchParam&)` | 空 |
| 23 | 0xa8 | `SetSessionID(uint)` | 空 |
| 24 | 0xb0 | `GetSessionID() const` | 空 |
| 25 | 0xb8 | `WarmUp()` | 空 |

**没有 Reset 方法**。重置走 `SetOption(0x10003, true)`（见 1.6）。

16 个信号的偏移 → 事件类型（读 `BaseSessionWrapper::BaseSessionWrapper` 0x100693e34 里每次 `create_connection` 前的 `add x0, x9, #off`）[确]：

| 偏移 | 事件 | 偏移 | 事件 |
|---|---|---|---|
| 0x008 | `CandidateUpdatedEvent`（候选表，内含 `CandidateIterator` = `cppcoro::recursive_generator<CandidateV2>`） | 0x408 | `UserHotWordRecommendEvent` |
| 0x088 | `EmitInputEvent`（上屏） | 0x488 | `KeyboardHotAreaEvent` |
| 0x108 | `PendingInputEvent`（组句串） | 0x508 | `_wxime_cloud_general_candidate_event` |
| 0x188 | `SyllableHintUpdateEvent` | 0x588 | `_wxime_cloud_text_recommend_event` |
| 0x208 | `CloudResultEvent` | 0x608 | `_wxime_cloud_llm_event` |
| 0x288 | `PendingInputDetailInfoEvent` | 0x688 | 批量事件 `vector<variant<CandidateUpdated, EmitInput, PendingInput, SyllableHintUpdate>>` [推] |
| 0x308 | `CloudTextCorrectEvent` | 0x708 | `signal<void(bool)>` |
| 0x388 | `CloudTextRewriteEvent` | 0x788 | `signal<void()>` [推] |

结论：接口是「**命令往里调、结果往外推**」。命令（ProcessInput / SelectCandidate / SetOption…）是虚函数，层层下传；
候选、上屏、云结果等都是事件，由最里层发出，经每层的信号**冒泡**到最外层，最外层的 `CCallbackHolder` 转成 C 回调给壳。

### 1.2 BaseSessionWrapper：所有包装层的基类 [确]

`wxime::engine::session::BaseSessionWrapper`，虚表 0x1031abb68，构造 `BaseSessionWrapper(unique_ptr<ISession> inner)`（0x100693e34）。

- 布局：`ISession` 基类（0x820 字节）+ `+0x820 unique_ptr<ISession> inner` + `+0x828…+0xb27` 16 个 `lsignal::connection`。
  `Calendar` 分配 0xb30 字节 = 0xb28 + 一个 `unique_ptr<Impl>`（0x1006b2be0），`BatchEmit` 分配 0xb48（0x1006b965c），`SimplifiedToTraditional` 的 Impl 指针在 `+0xb28`。
- 每个命令方法都是一跳转发（0x1008da284–0x1008da4c8，形如 `ldr x0,[x0,#0x820]; ldr x8,[x0]; ldr x?,[x8,#off]; br x?`）：

```cpp
// 0x1008da284 等，全部同型
R BaseSessionWrapper::M(args...) { return inner_->M(args...); }
```

- 构造时把 inner 的 16 个信号逐个连到「原样转发到自己的同名信号」的 lambda（符号 `BaseSessionWrapper::BaseSessionWrapper(...)::'lambda'(CandidateUpdatedEvent const&)` 等 16 个）：

```cpp
BaseSessionWrapper(unique_ptr<ISession> inner) : inner_(move(inner)) {
  for k in 0..16: conn_[k] = inner_->sig[k].connect([this](auto&& ev){ this->sig[k](ev); });
}
void DisconnectSignals() {          // 0x1008da4d8
  if (!inner_) return;
  for k in 0..16: inner_->sig[k].disconnect_all();   // 断开 inner 往外的所有连接
}
void CancelPendingJobs() {          // 0x1008da5c0
  DisconnectSignals();
  ISession::CancelPendingJobs();    // 断开自己的 16 个信号（0x100634cc4）
  if (inner_) inner_->CancelPendingJobs();   // 尾调用，逐层下传
}
```

- 一个具体包装层**要改写某类事件**时，固定做法（S2T 的 Impl 构造 0x100c82a1c，Calendar 构造 0x100c6e5cc 都是这样）：
  `inner_->sig[k].disconnect_all()` 把基类装的默认转发拆掉，再 `inner_->sig[k].connect(自己的 lambda)`，lambda 里改完再 `this->sig[k](改后的事件)`；
  不关心的事件保持基类的原样转发。**只改命令**的层（NumberCalculator）则只覆写虚函数。

### 1.3 SessionCreator::operator() 的组装方式 [确]

- `x21 = const _wxime_session_config*`，`[sp+0x50] = this(SessionCreator)`，`[sp+0x900] = shared_ptr<EnvContext>`（由 `EnvContext(session_config, config)` 0x1006ec710 构造），
  `[sp+0x850] = unique_ptr<ISession> current`。
- 每层的套法（以 Calendar 为例，0x1006b2b84–0x1006b2cc4）：

```cpp
InitContext ctx;                 // 0x98 字节，+0x0 是 unique_ptr<ISession>，+0x58 起若干 shared_ptr（EnvContext、TextAroundCursorContext…）
ctx.session = std::move(current);          // ldr x19,[sp,#0x850]; str xzr,[sp,#0x850]
ctx.env     = env;                          // [sp+0x900] 拷进 ctx
auto* s = new Calendar(std::move(ctx));     // operator new(0xb30); Calendar::Calendar(InitContext) 0x100c6e7e4
current.reset(s);                           // str x24,[sp,#0x850]，旧值（已被 move 空）析构
```

  也有 `make_unique<X>(std::move(current), …)` 与 `X(unique_ptr<ISession>, …)` 的写法，语义相同。因此**执行顺序 = 由里到外**。
- 开头按 `keyboard_type`（config+0x4）跳表选核心会话（0x1006b09bc，跳表在 0x102e34eb0，`type-2` 作下标，0..6）：

| keyboard_type | 去处 | 核心会话 |
|---|---|---|
| 2 | +0xdec | `EnglishSessionV2`；`enable_english_assisted_spelling_correction`(0x3e) 时再包 `ReplayEnglishResult` |
| 3 | +0x14f0 | 手写（日志「resources->hw_model_ failed in WXIME_KEYBOARD_HAND_WRITING」）[推] |
| 4 / 8 | +0x1460 / +0x176c | `StrokeSession`（8 先走一个内联的 `ISession` 子类）[推] |
| 5 | +0x1688 | `WubiSessionV2`；`wubi_config.enable_comment_`(0x28) 时再包 `WubiComment` |
| 0、1、6、7、其他 | +0x127c | 拼音：`AsyncLightGBMManager::GetAndLoad`（排序模型）→ `PinyinV2Session` → `PinyinV2WarmUpSession` → `UncommonWordFilter` |

  各分支最后 `b 0x1006b2444`（+0x284c）汇合，进入公共的包装序列。
- 返回 `Result { unique_ptr<ISession> session, shared_ptr<TextAroundCursorContext> }`（0x1006b97b0–0x1006b97c8）。返回前若距上次预热 ≥ 121（`(now - creator[+0x110]) / 1e6 >= 0x79`）
  就调一次 `session->WarmUp()` 并打「Long time no warm up, do it, %u」（0x1006b971c）。单位按 libc++ `system_clock` 微秒算是 120 秒 [推]。

### 1.4 完整套序（由里到外）与条件

`_wxime_session_config` 字段名来自 Swift 字段描述符（0x10306d9ac，37 个字段），偏移由函数头的日志串对上
（「type: 」+4、「is_wifi: 」+0xb0、「exp mode: 」+0xaf、「most likely: 」+0x3c、「s2t: 」+0xb8、「sp: 」+0x20、「input: 」+0x41、「most likely: 」+0x42、
「correction: 」+0x40、「book_video: 」+0x43、「rewrite: 」+0x46、「safe_mode: 」+0x3b、app_name +0xc0）[确]，其余按 C 布局推出并与守卫交叉验证 [推]：
`with_async_session_wrapper`0x0、`with_search_candidate_instantly_session_wrapper`0x1、`keyboard_type`0x4、`double_pin_solution`0x20、`wubi_config`0x24、
`local_most_likely_config`0x2c–0x32（…`enable_easter_egg`0x30、`enable_most_likely_filter`0x32）、`local_process_input_config`0x33–0x39、`enable_pre_input`0x3a、
`safe_mode`0x3b、`enable_auto_most_likely`0x3c、`enable_english_assisted_spelling_correction`0x3e、`cloud_engine_opt`0x40…、`disable_user_dict`0xa8、
`enable_local_correction`0xae、`simplified_to_traditional`0xb8、`app_env`0xc0（`app_scene`0xe4）、`check_displayable`0xe8、`check_kaomoji_emoji_displayable`0xf0、
`e2e_option`0x120、`disable_batch_emit`0x124、`enable_v_mode`0x125、`wait_async_result_for_test`0x126。
`is_pc` = `env.platform ∈ {5,6}`（0x1006b0378，存 `sp+0x8cf`）[推：只用于 PcEmojiMatcher / PCMostLikely 一类]。
「res N」= `resources->GetResource(N)`（vtable+0x5c8）非空才套，即「数据文件在才挂这层」。

| # | 层 | 地址（构造调用） | 套上的条件 | 置信 |
|---|---|---|---|---|
| 0 | 核心会话（见 1.3） | — | keyboard_type | 确 |
| 1 | PunctRelatedSession | 0x1006b246c | keyboard_type ∈ {0,2} | 确 |
| 2 | PcEmojiMatcher 或 EmojiMatcher | 0x1006b2638 / 0x1006b26e0 | is_pc → Pc 版；否则 res 155 在 | 推 |
| 3 | UserHotWordSession | 0x1006b2960 | 无 | 确 |
| 4 | UserScenesSession | 0x1006b2ac8 | 无 | 确 |
| 5 | DirtyCheckingSession（内联构造，虚表 0x1006b2b34） | 0x1006b2b14 | 无 | 确 |
| 6 | Calendar | 0x1006b2c9c | `(1<<keyboard_type) & 0x23`，即 kb ∈ {0,1,5} | 确 |
| 7 | UserCoordinateDictWrapper（固定位置用户词） | 0x1006b2d18 | kb<2 且 `!disable_user_dict`(0xa8)，res 134 | 确 |
| 8 | RemoveAuxiliaryData（内联） | 0x1006b2da4 | 无 | 确 |
| 9 | MostLikelySequel（联想） | 0x1006b2f4c | 无 | 确 |
| 10 | MostLikelyFilterForPunct | 0x1006b30b8 | `enable_most_likely_filter`(0x32) | 确 |
| 11 | NumberCalculator | 0x1006b3224 | 无 | 确 |
| 12 | CloudSearchByInputEn | 0x1006b355c | 视 kb（守卫 0x1006b3478） | 推 |
| 13 | CloudSearchByInputV2 | 0x1006b3778 | 无（层内看 cloud 开关） | 推 |
| 14 | CandidateSelectSpeedInfoWrapper | 0x1006b3930 | res 139 | 确 |
| 15 | CloudMostLikelySerial 或 CloudMostLikely | 0x1006b396c / 0x1006b3984 | `enable_cloud_most_likely`(0x42)；AB `mmkb_cli_engine_cloud_most_likely_serial` 选哪个 | 确 |
| 16 | UncommonKaomojiFilter | 0x1006b3ac4 | `check_kaomoji_emoji_displayable` 回调非空 | 确 |
| 17 | CloudGeneralSearch | 0x1006b3b98 | 无 | 推 |
| 18 | CloudLlmSearch | 0x1006b3d30 | 无 | 推 |
| 19 | CloudTextRecommend 或 V2 | 0x1006b3eb8 / 0x1006b4034 | AB `mmkb_cli_engine_cloud_text_recommend_v2` | 确 |
| 20 | CloudTextCorrect | 0x1006b4214 | 无 | 推 |
| 21 | CloudReportEmitInput | 0x1006b42fc | `enable_cloud_search_by_input`(0x41) | 确 |
| 22 | LocalTextCorrect | 0x1006b4528 | `!enable_cloud_correction`(0x40) 且 `enable_local_correction`(0xae) | 确 |
| 23 | EasterEggMostLikely | 0x1006b4698 | 本地 `enable_easter_egg`(0x30) 或云 0x8d | 推 |
| 24 | UserAssociationWrapper | 0x1006b48c4 | 无 | 确 |
| 25 | AssoCandidateFeatureAppender | 0x1006b49f4 | 无 | 确 |
| 26 | TextOnCursorRecommend | 0x1006b4bd0 | 无 | 确 |
| 27 | HotFix | 0x1006b4d58 | 无 | 确 |
| 28–33 | UserDictSingleEmitWrapper → UserDictsModifierWrapper → UserSingleCnSelectWrapper → UserDictMultipleEmitsWrapper（kb<2）→ UserDictEnSendWrapper → UserCommonWords | 0x1006b9ad8 … 0x1006ba06c | `!disable_user_dict` 且用户词库有效（否则日志「user dicts not valid」）；读 AB `mmkb_cli_engine_user_dict_merge_strategy` | 确 |
| 34 | UserWordHardFixWrapper | 0x1006b57a4 | 同上 | 确 |
| 35 | TextWithSyllableRecorder | 0x1006b5068 | 无 | 确 |
| 36 | ProcessUserSpecifiedUpperCase | 0x1006b50f8 | kb ≤ 1 | 确 |
| 37 | PunctInterceptor | 0x1006b5254 | kb ≠ 2 | 确 |
| 38 | VModeV2 | 0x1006b53c4 | `enable_v_mode`(0x125) | 确 |
| 39–45 | StatUserActionsWrapper → ReportDropAfterEmit → SessionKeystrokeReporter → StatEmitAndActionInfoWrapper → ReportEmittedInputWordsWrapper → RecordProcessInputWrapper → ProcessTotalTimeRecorder | 0x1006b560c … 0x1006b5df4 | 无（统计 / 上报层） | 确 |
| 46 | UserProfileFlusher | 0x1006b5fdc | 视 is_pc | 推 |
| 47 | KeyboardHotArea | 0x1006b6050 | bigram reader 非空 | 推 |
| 48 | AtSymbolRelated | 0x1006b60c0 | 视 kb（守卫 0x1006b60a0：kb==4 或 kb==1） | 推 |
| 49 | EnCloudInputMerger | 0x1006b618c | 非 PC | 推 |
| 50 | EnglishTee | 0x1006b6334 | 无 | 确 |
| 51 | SelectCandidateWithSuffix | 0x1006b637c | 无 | 确 |
| 52 | EnFontSession | 0x1006b6470 | res 40 | 确 |
| 53 | RecordLastEmitInfo | 0x1006b6528 | 无 | 确 |
| 54 | CloudMostLikelyRelatedMergerV2 | 0x1006b66d8 | `enable_cloud_most_likely`(0x42) 或 `enable_text_recommend`(0x47) | 确 |
| 55 | AutoInferTextBeforeCursor | 0x1006b8014 | 同一分支内 | 推 |
| 56–58 | AutoMostLikelyForEn（kb==2/8）/ AutoMostLikely / AutoMostLikelyForNum | 0x1006b68ac / 0x1006b6abc / 0x1006b69b4 | 视 kb 与 `enable_auto_most_likely` | 推 |
| 59 | AutoPrefillContextForNN | 0x1006b6bc8 | 有端到端模型 | 推 |
| 60 | PCMostLikely | 0x1006b6d48 | is_pc | 推 |
| 61 | NumberCalculator（第二个，构造参数 bool 不同） | 0x1006b6eb4 | 同上分支 | 推 |
| 62 | UserAssociationAutoAdd | 0x1006b7018 | 无 | 确 |
| 63 | OnlyUserHotWordAndPunctWrapper | 0x1006b707c | `app_env.app_scene`(0xe4) == 2 | 确 |
| 64 | AddPinyinCorrectionInfoWrapper | 0x1006b7118 | res 29 | 确 |
| 65 | **InterceptSensitiveResult**（敏感词拦截） | 0x1006b72cc | 敏感词表有效（否则日志「sensitive_word_dict not valid」0x1006b73b8） | 确 |
| 66 | UserBlockListDictWrapper（用户屏蔽词） | 0x1006b7438 | 屏蔽词表有效 | 确 |
| 67 | AddWeBookTitleTagForMostLikelyResult | 0x1006b7634 | res 26 | 确 |
| 68 | **SimplifiedToTraditional** | 0x1006b7860 | `simplified_to_traditional`(0xb8) ≥ 1 | 确 |
| 69 | CandidateInfoPatcher | 0x1006b78a8 | 无 | 确 |
| 70 | MergeLatestAbTestExptInfo | 0x1006b7a10 | 无 | 确 |
| 71 | PreProcessNotEnableAdapter 或 PreProcessInputSessionV2 | 0x1006b7aac / 0x1006b9320 | kb ∈ {2,3,8}（`(1<<kb)&0x10c`）→ Adapter；否则 `enable_pre_input` → V2 | 推 |
| 72 | **MostLikelyFilter** | 0x1006b7ccc | 读 AB `mmkb_cli_engine_filter_local_most_likely_candidate` | 推 |
| 73 | CloudMostLikelyRelatedMerger（旧版） | 0x1006b7e78 | `enable_cloud_most_likely` 且未走 V2 | 推 |
| 74 | CloudInputMerger | 0x1006b80f0 | 无 | 推 |
| 75 | RecoverLastWrapper | 0x1006ba154 | `local_most_likely.enable_normal_text`(0x2f) 且 creator[+0x7c]==0 且 AB `mmkb_cli_engine_recover_last`=="1" | 确 |
| 76–78 | DistinctCandidateWrapper → EnableCombineWords → NullCandidateFilter | 0x1006b821c… | 视 is_pc | 推 |
| 79 | FilterWordsWithStroke | 0x1006b82d8 | 无 | 确 |
| 80 | ChEnMixFlagWrapper | 0x1006b83e0 | kb ≤ 1 | 确 |
| 81–84 | RecordSelectedCandidateInfoWrapper → SetPositionForSelectionWrapper → CandidateVersioningWrapper → SyllableHintVersioningWrapper | 0x1006b8420… | 无 | 确 |
| 85 | UserProfileEmojiStatisticsWrapper | 0x1006b8644 | res 139 | 确 |
| 86–90 | RecordCandidateListInfoWrapper → RecordCandidateFeatureInfoWrapper → RecordAssociationContextCandidatesWrapper → DistinctSearchResultAndUserAction → SelectionRecords | 0x1006b86a0… | 无 | 确 |
| 91 | SelectDefaultCandidate | 0x1006b8b24 | 无 | 确 |
| 92–95 | InternalCursorMaintainer → TextAroundCursorMaintainer → ResourceHolder → KeyboardLayoutHolder（均内联，靠虚表认出） | 0x1006b8c20… | 无 | 确 |
| 96 | EnvContextHolder | 0x1006b8d80 | 无 | 确 |
| 97 | DebugDetailsJsonMaintainer（内联） | 0x1006b8dec | 无 | 确 |
| 98 | TextAroundCursorInfoAppender | 0x1006b90c4 | 读 AB `mmkb_cli_engine_before_context_limit_for_cloud_input`（函数头读过一次） | 推 |
| 99 | AsyncSessionV2（asio io_context 异步化） | 0x1006b917c | `with_async_session_wrapper`(0x0) | 确 |
| 100 | AsyncSessionWaitForTest（内联） | 0x1006b9204 | `wait_async_result_for_test`(0x126) | 确 |
| 101 | SearchCandidatesInstantlyForHW / …WithPreProcess / SearchCandidatesInstantly | 0x1006b92a8 / 0x1006b92d4 / 0x1006b93b4 | `with_search_candidate_instantly…`(0x1)；kb==3 → HW；否则 `enable_pre_input` → WithPreProcess；否则普通 | 确 |
| 102 | StatusSession | 0x1006b9520 | 无 | 确 |
| 103 | LogTraceIDSession | 0x1006b9590 | 无 | 确 |
| 104 | WatchDogV2Session | 0x1006b9a48 | creator[+0x7c]==0（并受 0x0/0x1 两个开关影响） | 推 |
| 105 | **BatchEmit**（最外层） | 0x1006b96f0 | `!disable_batch_emit`(0x124) | 确 |

层序有规律：**内容层（emoji、日历、计算器、云）贴着核心 → 用户词库层 → 统计上报层 → 联想 / 自动首选 → 过滤层（敏感词、屏蔽词）→ 文本变换（繁简）→
本地联想过滤 → 记录 / 版本号 / 光标维护 → 异步化 → 批量发射**。过滤与繁简在内容层之外，所以云、emoji、日历加进来的候选也会被过滤、被转繁体。

### 1.5 三个典型包装层

**(a) SimplifiedToTraditional：改写候选文本**（构造 0x100c80dcc，Impl 构造 0x100c82a1c，候选 lambda 0x100c82f88 / 0x100c837a8）[确，细节 推]

```cpp
SimplifiedToTraditional(InitContext ctx) : BaseSessionWrapper(move(ctx.session)) {
  impl_ = new Impl(this, config);                   // +0xb28
}
Impl(owner, config) {
  dict_ = ResourceMgr->GetResource(36);             // 繁简表，mov w1,#0x24
  mode_ = (config->keyboard_type == 5);             // +0x18，五笔另一套
  busy_ = false;                                    // +0x19
  if (!dict_) return;                               // 表不在：整层纯透传
  owner->inner_->sig[CandidateUpdated].disconnect_all();
  owner->inner_->sig[CandidateUpdated].connect(on_candidates);   // 连接存 owner+0x858
  owner->inner_->sig[CloudResult].disconnect_all();
  owner->inner_->sig[CloudResult].connect(on_cloud);
  owner->inner_->sig[EmitInput].disconnect_all();
  owner->inner_->sig[EmitInput].connect(on_emit);                // 上屏文本也转
}
void ProcessInput(sv in, aux) { impl_->busy_=1; inner_->ProcessInput(in,aux); impl_->busy_=0; }   // 0x100c80ef0
void SearchCandidates()       { impl_->busy_=1; inner_->SearchCandidates();   impl_->busy_=0; }   // 0x100c80eb0
on_candidates(const CandidateUpdatedEvent& ev) {          // 0x100c82f88
  for (iter : ev 里的两个 CandidateIterator) {            // 主候选与联想各一个
    auto g = iter->MoveGenerator();
    iter->SetGenerator(convert(std::move(g)));            // 惰性：拉到才转
  }
  owner->sig[CandidateUpdated](ev);
}
recursive_generator<CandidateV2> convert(gen g) {         // 0x100c837a8
  for (auto& c : g) {
    if (DisableTranslate(c.id)) { co_yield c; continue; }   // 0x100cc46b8：某些来源不转
    auto t = Translate(dict_, c, ...);                      // 0x100cc47b0
    if (t 与已出过的重复) continue;                          // 推：循环里有字符串比较去重
    co_yield t;
  }
}
```

**(b) NumberCalculator：替换整表 / Calendar：往流里插**

```cpp
// NumberCalculator(InitContext, bool) 0x100b2f858：enabled_ 在 +0xb38，第二个 bool 在 +0xb39
void NumberCalculator::SearchCandidates() {               // 0x100b309d8 [确]
  if (enabled_ && GetNumberCalculatorRes()) return;       // 自己发了候选表，不再问里层
  inner_->SearchCandidates();
}
bool GetNumberCalculatorRes() {                           // 0x100b2f924 [确：调用；推：细节]
  auto expr = inner_->PendingInput();
  auto v = utils::Calculator(opt)(expr);                  // 0x100f41544 / 0x100f41550
  if (!v) return false;
  CandidateUpdatedEvent ev = 若干格式（_wxime_candidate_calculator_format）的结果候选;
  this->sig[CandidateUpdated](ev);                        // 0x100b30240，直接从本层发
  return true;
}
// 另覆写 GetMostLikelySequel（0x100b30a20）、SelectCandidate（0x100b30a5c）处理选中计算结果。

// Calendar(InitContext) 0x100c6e5cc：Impl(this, bool) 读时间格式表；拆掉 inner 的 CandidateUpdated 默认转发，换成 $_0
on_candidates(ev) {                                       // $_0 → 0x100c7120c
  for (iter : ev) iter->SetGenerator(insert(iter->MoveGenerator()));
  this->sig[CandidateUpdated](ev);
}
recursive_generator<CandidateV2> insert(gen g) {
  int i = 0;
  for (auto& c : g) {
    co_yield c;
    if (i++ < 9 && c 来源是词库类 && Impl::MatchWord(c.text, fmt))   // cmp x21,#0x9；MatchWord 0x100c70ed4 [推：9 是窗口]
      for (auto& d : 该词的日期 / 时间格式) co_yield d;              // TryAdd 0x100c6ea6c
  }
}
```

**(c) MostLikelyFilter / InterceptSensitiveResult：过滤** [确：结构；推：位语义]

两者都**不覆写任何命令方法**（虚表除析构外全是 BaseSessionWrapper 的），只在构造时换掉事件转发：

```cpp
MostLikelyFilter::Impl(owner, InitContext ctx) {          // 0x100aae260
  mask_ = config->most_likely_filter;                     // 字段 34，u32 位掩码 [推]
  hook(CandidateUpdated, on_candidates);                  // 0x100aaea6c
  hook(CloudResult, on_cloud);                            // 0x100aaf414
  hook(cloud_text_recommend, on_recommend);               // Filter(_wxime_cloud_text_recommend_event&,…) 0x100aaf664
}
on_candidates(ev) {
  if (ev 是联想列表(type==1/3)) {
    optional<set<u8>> drop_sources = …;
    Filter(ev.list, session_id, is_cloud, is_pc, drop_sources);   // 0x100aaeba8，两次（主表 / 联想）
  }
  owner->sig[CandidateUpdated](ev);
}
bool NeedFilter(u32 mask, bool a, bool b, u8 source) {    // 0x100aaef34：逐位判
  // bit0..bit22 各对应一种候选来源（source 取值 1,2,3,5,6,7,0xa,0xb,0xc,0xf,0x10,0x11,0x1e,0x1f…），
  // 有的位再看 a / b；任一命中即过滤
}

InterceptSensitiveResult(unique_ptr<ISession>, unique_ptr<IDict> sensitive, …)   // 0x1006e4884
on_candidates(ev) → InterceptCandidates(&iter, text_before_cursor)   // 0x100bff528：生成器包一层，命中敏感表的候选不 yield
on_cloud(ev)      → InterceptCloudCandidates(&iter)                 // 0x100bffcf0
联想结果另走 WhetherBlockMostLikelyResults / DropSensitiveWordOfMostLikelyResults（看光标前文本 GetTextBeforeCursor 0x100c00178）
```

### 1.6 私密 / 无痕 / 安全模式有没有整体旁路 [确：没有运行时旁路]

- 引擎里搜不到 incognito / private / password / secure input 相关串；`safe_mode` 只出现在函数头日志（+0x74c）与上报 / DB 的「safe_mode return」。
- 最接近「私密」的是**按会话**的 `disable_user_dict`（config+0xa8）：建会话时它一置位，`UserCoordinateDictWrapper`（#7）与整组用户词库层（#28–34）
  **根本不套**（0x1006b2cd4、0x1006b4ea8、0x1006b4f84）。也就是说 WeType 的「旁路」是**构造期决定、整层不存在**，不是运行时开关；
  要切换就销毁会话重建（Swift 壳 `InputSessionCache` / `[MainInput][SessionRecreate]`）。
- 云、统计、上报层没有类似总闸；云层各自看 `cloud_engine_opt` 的分项开关。

### 1.7 重置与切换会话时怎么清状态 [确]

- `wxime_reset_session(id)`（0x1006591cc）→ `GlobalSessionMgr` 按 id 找到会话 → lambda（0x1006a66dc）：
  `session->SetOption(SessionOptions(0x10003), true)`（`mov w1,#3; movk w1,#1,lsl#16`；vtable+0x48）。
- `BaseSessionWrapper::SetOption(opt,bool)` 原样下传；有状态的层覆写它（共 36 个类，如 AutoMostLikely、PinyinV2Session、CloudMostLikely、VModeV2、AsyncSessionV2…），
  模式统一是「**先清自己，再无条件下传**」：

```cpp
void AutoMostLikely::SetOption(SessionOptions opt, bool v) {      // 0x1006de880
  switch (opt) {
    case 9..18: …更新自己的开关…; break;
    case 0x10003:                                                 // reset
      if (enabled_ && pending_) { inner_->GetMostLikelySequel(); } else EmitEmptyCandidateUpdate();
      shown_ = false;                                             // +0xb68
      break;
  }
  inner_->SetOption(opt, v);                                      // 0x1006de9d0：一律下传
}
void PinyinV2Session::SetOption(opt, v) {                         // 0x10091ea28 起
  if (opt == 0x10003) { …清 beam / 上文…; resetGraph(); }          // 0x10091bc18
}
```

- 销毁走 `CancelPendingJobs()`：每层先断 inner 的信号、再断自己的、再下传（1.2）。
- 会话按 App 各建一个（`wxime_create_session` / `wxime_destroy_session`，`TTLSessionManager` 过期回收），不存在「同一会话里切学习分区」。

---

## 2. 证据表

| 结论 | 置信 | 地址 / 字符串 |
|---|---|---|
| ISession 虚表 26 槽、4 个纯虚、无 Reset | 确 | 0x103175080；方法符号 0x100634ca8–0x100634d88 |
| 16 个信号各 0x80 字节，偏移 → 事件类型 | 确 | ISession::CancelPendingJobs 0x100634cc4；BaseSessionWrapper 构造 0x100693e34 |
| BaseSessionWrapper：inner 在 +0x820，命令一跳转发 | 确 | 0x1008da284–0x1008da4c8 |
| 改写型层「disconnect_all 再 connect 自己的 lambda」 | 确 | S2T Impl 0x100c82b50–0x100c82d30；Calendar 0x100c6e658–0x100c6e68c |
| 当前会话槽 sp+0x850，move 进 InitContext、构造后写回 | 确 | Calendar 段 0x1006b2b84–0x1006b2cac；BatchEmit 段 0x1006b961c–0x1006b9700 |
| 拼音核心最先建，块尾 `cbz → +0x284c` 回到公共序列 | 确 | 0x1006ba3c4、0x1006ba5ec |
| keyboard_type 跳表 | 确 | 0x1006b09bc–0x1006b09e0，表 0x102e34eb0 |
| config 字段名 | 确 | `__swift5_fieldmd` 0x10306d9ac（37 字段）；嵌套结构 `_wxime_cloud_engine_option` 等 |
| 字段偏移 0x4/0x20/0x3b/0x3c/0x40–0x46/0xaf/0xb0/0xb8/0xc0 | 确 | 函数头日志串 0x1006afe74–0x1006b0354 |
| 其余字段偏移（0xa8、0xae、0x124–0x126 等） | 推 | C 布局推算 + 守卫语义吻合 |
| 构造顺序（由里到外，105 项） | 确（顺序）/ 见表（条件） | /tmp/a3work/topo.txt（CFG 逆后序） |
| AB 参数名 | 确 | 0x102c90a31、0x102c90917、0x102c90940、0x102c910c4、0x102c90968、0x102c9099b |
| 「user dicts not valid」「sensitive_word_dict not valid」 | 确 | +0x5368、+0x77c0 |
| disable_user_dict 让用户词库层整组不套 | 确 | 0x1006b2cd4、0x1006b4ea8、0x1006b4f84 |
| 重置 = SetOption(0x10003, true) | 确 | 0x1006a66dc |
| SetOption 先处理再无条件下传 | 确 | AutoMostLikely 0x1006de9d0；PinyinV2Session 处理 0x10003 → resetGraph |
| CancelPendingJobs 逐层下传 | 确 | 0x1008da5c0 |
| NumberCalculator 命中即自己发表、不问里层 | 确 | 0x100b309d8、0x100b30240 |
| S2T 惰性改写生成器；DisableTranslate 跳过部分来源 | 确（调用）/ 推（去重） | 0x100c837a8、0x100cc46b8、0x100cc47b0 |
| MostLikelyFilter 位掩码按来源过滤 | 推 | NeedFilter 0x100aaef34 |
| Calendar 只在前 9 个候选后插 | 猜 | 0x100c713c4 `cmp x21,#0x9` |
| 预热阈值 121（约 120 秒） | 推 | 0x1006b972c–0x1006b974c，日志「Long time no warm up, do it, %u」 |

---

## 3. 青简落点

### 3.1 现状：横切逻辑散在哪

| 横切功能 | 位置 | 形态 |
|---|---|---|
| 私密：不写学习 | `engine/learning/muted.rs`（`MutedLearner`，包住 `Box<dyn Learner>`） | 已是包装层 |
| 私密：不记输入日志 | `engine/input_log/muted.rs` | 已是包装层 |
| 私密：不联想 / 不发云 | `engine/prediction/mod.rs:53`、`:157` | `if self.private` 分支 |
| 私密：不记词汇、不问释义兜底 | `engine/commit/mod.rs:191`、`:208`；`engine/learning/mod.rs:61`（`record_exposures`） | 分支 |
| 私密切换时丢在飞结果与前文 | `engine/privacy.rs` `set_private_from` | 一处 |
| 繁体转换 | `engine/query/mod.rs` `query()` 末尾（`self.traditional && opencc`）；上屏反查 `commit/mod.rs:35/91`、`learning/mod.rs:154` | 分支 |
| 自定义短语、快捷项、emoji、英文插入 | `engine/extras.rs` 的 `insert_*`，由 `query/phonetic.rs:42/227/228`、`query/modes.rs:99`、`query/mod.rs:52` 调 | 函数调用散在各模式里 |
| 叠加学习（按对象分区） | 分叉的 `cloud/crates/qingjian-cloud-bridge/src/scope/scoped_learner.rs`（`ScopedLearner`，倍数 `OVERLAY_WEIGHT = 4`）；Core 侧补丁 `Learner::scoped_user_words / scoped_user_english / scope_changed`（`engine/learning/learner.rs:65-75`）、`setup.rs:546`、`extras.rs:189` | 学习器包装 + 补丁钩子 |
| 换上下文 | `engine/session.rs` `swap_session` / `discard_input` | 一处 |

与 WeType 的根本差别：青简 `query()` 是**同步拉取、返回 `Vec<Candidate>`**，没有事件冒泡，也不是 `ISession` 链。按任务约束不重写 Engine，
所以照搬的是 WeType 的**组织规矩**而不是信号机制：

1. 每个新能力是一个独立对象，只做「透传 + 改写」中自己那一件；
2. 层序固定、写在一个地方（WeType 的 SessionCreator ↔ 青简的 `OverlayStack::standard`）；
3. 重置是一条消息，**逐层广播、每层先清自己**（WeType 的 SetOption(0x10003) 一律下传）；
4. 关一层不影响别层（WeType 是构造期不套，青简用运行时掩码，便于 `--overlay` 回放逐层开关）。

WeType 没有私密总闸，青简要的「set_private 一次性旁路」是**青简自己补的**：在栈的唯一入口判一次。

### 3.2 分层与层序

```
                     读（组句 / 出候选）                                  写（上屏）
壳 ──▶ Engine::query()                                         Engine::commit_*()
        │ query_inner()  （存量：拼音 / 形码 / 英文 / 整句，不动）            │
        │   排序读 learner.weight() / user_ngram() / all_dictionaries()     │
        │   ┌─ L0 学习器链（存量 + 分叉，已是包装层）────────────────┐       │
        │   │ MutedLearner ─▶ OverlayLearner(=ScopedLearner) ─▶ FrequencyLearner │◀── record_*
        │   └ 私密：Muted 挡写；新增 suspend_overlay 挡叠加读 ────────┘       │
        │   all_dictionaries() += persona.dictionaries（私密时不加）           │
        ▼                                                                    │
   [L1 PersonaBoost.rewrite]   人设词在窗口内上提（新增）                       │
   [L2 HintLayer.hint]         只出提示行，不动候选（新增）                     │
        ▼  ── OverlayStack 入口：private ⇒ 整段跳过 ──                        │
   insert_custom_phrases（存量）  ← 用户钉的位置在叠加层之外，不被人设挤开      │
   last_query 快照（存量）                                                    │
   繁体转换（存量，最外，同 WeType S2T 在内容层之外）                           │
        ▼                                                                    ▼
      Query { candidates, hint }                         OverlayStack.on_commit（私密跳过）
```

为什么这样放（对照 WeType）：
- **人设词库与学习表在最里**：WeType 把 UserHotWord / UserCoordinateDict / 用户词库层贴着核心（#3、#7、#28–34），让后面的过滤、繁简照常作用于它们。
  青简的等价物是「词库进 `all_dictionaries()`、计数进学习器链」，候选在 `query_inner` 里就带着，排序代码一行不改。
- **人设加权放 L1 改写层而不是改排序函数**：排序打分不在 `Candidate` 上（只有 `text/kind/…`），改分数要动 `query/phonetic.rs`；
  做成「窗口内上提」的改写层，可单测、可在回放里单独关。
- **HintIndex 是旁路输出层**：WeType 的 TextOnCursorRecommend / CloudTextRecommend（#19、#26）都是另发事件、不动候选表；青简同理，结果放 `Query::hint`。
- **自定义短语、繁体在叠加层之外**：用户手钉的位置不被人设挤动；繁体最后转，人设词表只存简体。

### 3.3 新增类型与签名（全部在 Core，平台无关；一个类型一个文件）

```
crates/qingjian-core/src/engine/overlay/
  mod.rs        mod 声明、re-export、常数
  layer.rs      trait OverlayLayer
  id.rs         enum LayerId
  mask.rs       struct LayerMask
  context.rs    struct LayerContext<'a>
  reset.rs      enum ResetReason
  stack.rs      struct OverlayStack
  scope.rs      struct LearningScope
  setup.rs      impl Engine { set_learning_scope, set_persona, set_hint_index, set_overlay_enabled }
  persona/{mod.rs, spec.rs (PersonaSpec), boost.rs (PersonaBoost)}
  hint/{mod.rs, hint.rs (Hint), card.rs (HintCard), index.rs (HintIndex), layer.rs (HintLayer)}
```

```rust
// overlay/layer.rs
//! 叠加层：候选生成之后的一道「透传 + 改写」。只读 LayerContext，不碰 Engine 内部；私密由 OverlayStack 统一挡。
pub trait OverlayLayer: Send {
    fn id(&self) -> LayerId;

    /// 改写候选表（上提 / 插入 / 过滤）。不改写的层用缺省实现。
    fn rewrite(&self, _cx: &LayerContext<'_>, _items: &mut Vec<Candidate>) {}

    /// 当前该显示的提示；`None` 表示这层没有。
    fn hint(&self, _cx: &LayerContext<'_>) -> Option<Hint> {
        None
    }

    /// 上屏通知（提示的上屏后匹配、给壳的上传回调）。私密中不会被调。
    fn on_commit(&mut self, _cx: &LayerContext<'_>, _text: &str) {}

    /// 清状态。无论层是否启用、是否私密都会收到（照 WeType SetOption(0x10003) 一律下传）。
    fn reset(&mut self, _reason: ResetReason) {}
}

// overlay/id.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LayerId {
    /// 学习器链里的叠加学习表（不在 OverlayStack 里，掩码位只用来让 suspend_overlay 生效）。
    Learner,
    Persona,
    Hint,
}

// overlay/mask.rs
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LayerMask(u8);               // 置位 = 关掉
impl LayerMask {
    pub fn set(&mut self, id: LayerId, enabled: bool);
    pub fn enabled(self, id: LayerId) -> bool;
}

// overlay/context.rs
pub struct LayerContext<'a> {
    /// 当前作用域的键串（`Composition::scope()`）。
    pub keys: &'a str,

    /// 光标前已上屏的文字（`InputHistory` 的尾巴），提示匹配用。
    pub before: &'a str,

    /// 当前学习分区。
    pub scope: &'a LearningScope,
}

// overlay/reset.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResetReason {
    /// 换了学习分区（场景 / 对象）。
    ScopeChanged,
    /// 换了人设。
    PersonaChanged,
    /// 组句被丢弃（discard_input / swap_session）。
    InputDiscarded,
}

// overlay/scope.rs
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LearningScope {
    /// 分区标识（壳给的相对路径或代号；Core 只拿来比较与传给回调）。
    pub path: String,

    /// 读不读全局学习；工作场景可关。
    pub inherit_global: bool,
}

// overlay/stack.rs
pub struct OverlayStack {
    /// 由里到外。
    layers: Vec<Box<dyn OverlayLayer>>,

    disabled: LayerMask,
}
impl OverlayStack {
    pub fn new() -> Self;                                  // 空栈：三个能力都没装时零开销
    pub fn standard(persona: Option<PersonaBoost>, hint: Option<HintLayer>) -> Self;  // 层序唯一的定义处
    pub fn set_enabled(&mut self, id: LayerId, enabled: bool);
    pub fn is_enabled(&self, id: LayerId) -> bool;
    /// 私密时整段跳过；否则按层序依次改写启用的层。
    pub fn rewrite(&self, private: bool, cx: &LayerContext<'_>, items: &mut Vec<Candidate>);
    /// 私密时 None；否则由外到里取第一条。
    pub fn hint(&self, private: bool, cx: &LayerContext<'_>) -> Option<Hint>;
    pub fn on_commit(&mut self, private: bool, cx: &LayerContext<'_>, text: &str);
    /// 一律广播到每一层（不看掩码、不看私密）。
    pub fn reset(&mut self, reason: ResetReason);
    pub fn replace(&mut self, id: LayerId, layer: Option<Box<dyn OverlayLayer>>);   // 换人设 / 换提示索引
}

// overlay/persona/spec.rs
pub struct PersonaSpec {
    /// 话术词库，与附加词库一样进 all_dictionaries()。
    pub dictionaries: Vec<Dictionary>,

    /// 上提力度：0 关，1.0 为缺省；PersonaBoost 按它算上提位数。
    pub boost: f64,

    /// 语气说明，只给可选的云端改写，Core 不读。
    pub style: Option<String>,
}

// overlay/persona/boost.rs
pub struct PersonaBoost {
    /// 人设词表里的词（简体），构造时从 dictionaries 摊平。
    words: HashSet<String>,

    boost: f64,
}
impl PersonaBoost {
    pub fn new(spec: &PersonaSpec) -> Self;
}
impl OverlayLayer for PersonaBoost { fn id(&self) -> LayerId { LayerId::Persona } fn rewrite(...); }

// overlay/hint/card.rs
pub struct HintCard {
    pub id: u64,

    /// 提示行显示的文字。
    pub text: String,

    /// 关键词与权重（包里已含同义词扩展）。
    pub keywords: Vec<(String, f32)>,
}

// overlay/hint/hint.rs
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Hint {
    pub card_id: u64,

    pub text: String,

    pub score: f32,
}

// overlay/hint/index.rs
pub struct HintIndex {
    cards: Vec<HintCard>,

    /// 关键词 → (卡下标, 权重)。
    keywords: HashMap<String, Vec<(usize, f32)>>,

    /// 最长关键词的字数，扫描窗口用。
    max_chars: usize,
}
impl HintIndex {
    pub fn new(cards: Vec<HintCard>) -> Self;
    /// 在 `text` 末尾 HINT_TAIL_CHARS 个字里找关键词，同卡累加权重，最高分过 HINT_MIN_SCORE 才返回。
    pub fn lookup(&self, text: &str) -> Option<Hint>;
}

// overlay/hint/layer.rs
pub struct HintLayer {
    index: HintIndex,

    /// 刚上屏后的那条提示，下一次组句开始前一直显示；reset 清掉。
    after_commit: Option<Hint>,
}
impl OverlayLayer for HintLayer { /* hint(): 组句中用 before + 首选文字 lookup，否则返回 after_commit；
                                     on_commit(): after_commit = index.lookup(before + text)；reset(): after_commit = None */ }
```

`mod.rs` 常数（初值是青简自定，**要用 apps/cli 回放重扫**；WeType 没有对应常数可借）：

```rust
/// 人设上提只看前几个候选。
pub const PERSONA_WINDOW: usize = 8;
/// 一个人设词最多上提几位（boost = 1.0 时）；实际 = round(boost * PERSONA_MAX_LIFT)。
pub const PERSONA_MAX_LIFT: usize = 2;
/// 提示：只在光标前这么多字里找关键词。
pub const HINT_TAIL_CHARS: usize = 24;
/// 提示：同卡关键词权重和过这条线才出。
pub const HINT_MIN_SCORE: f32 = 1.0;
```

### 3.4 改哪些存量文件（每处只加一两行，都标「分叉补丁」）

| 文件 | 改动 |
|---|---|
| `engine/mod.rs` | `mod overlay;`，`Engine` 加字段 `overlay: OverlayStack`、`learning_scope: LearningScope`、`persona_dictionaries: Vec<Dictionary>`；`new()` 里给空值 |
| `engine/query/mod.rs` `query()` | `insert_custom_phrases` 之前：`self.overlay.rewrite(self.private, &cx, &mut query.candidates.items);`；之后 `query.hint = self.overlay.hint(self.private, &cx);` |
| `engine/query/result.rs` | `Query` 加 `pub hint: Option<Hint>`（`#[serde(default)]`），各构造处给 `None` |
| `engine/commit/mod.rs` | `log_commit` 之后：`self.overlay.on_commit(self.private, &cx, &candidate.text);` |
| `engine/setup.rs` `all_dictionaries()` | 附加词库之后：`if !self.private && self.overlay.is_enabled(LayerId::Persona) { all.extend(self.persona_dictionaries.iter()); }` |
| `engine/learning/learner.rs` | 新缺省方法 `fn suspend_overlay(&mut self, _suspended: bool) {}`（与 `scope_changed` 同一类补丁） |
| `engine/privacy.rs` `set_private_from` | `self.learner.inner_mut().suspend_overlay(combined); self.forget_span_cache();`（私密切换会改词库集合与计数，格子缓存必须作废） |
| `engine/session.rs` | `swap_session`、两个 `discard_input` 末尾 `self.overlay.reset(ResetReason::InputDiscarded)` |
| `cloud/crates/qingjian-cloud-bridge/src/scope/scoped_learner.rs` | 实现 `suspend_overlay`：置位后 `count()` 只读全局、`scoped_user_words/english` 返回 `None`；它就是设计里的 `OverlayLearner`，不必在 Core 再写一份 |
| `apps/cli` | `--overlay learner=on,persona=off,hint=on` 解析成 `Engine::set_overlay_enabled`；`learner=off` 走 `suspend_overlay(true)` |

`overlay/setup.rs` 的 Engine 方法：

```rust
impl Engine {
    /// 换学习分区。顺序照 WeType 的 reset：先断在飞，再逐层清，最后作废缓存。
    pub fn set_learning_scope(&mut self, scope: LearningScope) {
        if self.learning_scope == scope { return; }
        self.cancel_prediction();                 // 在飞的云结果不能落到新分区
        self.rescoring_before = None;
        self.recent_commits.clear();              // 否则「上屏后马上删掉重选」会 unrecord 到新分区
        self.chain = CommitChain::default();
        self.history.clear();                     // 前文属于旧对象
        self.learning_scope = scope;
        self.learner.inner_mut().scope_changed(); // 存量钩子：ScopedLearner 重建快照
        self.overlay.reset(ResetReason::ScopeChanged);
        self.forget_span_cache();
        *self.correction_cache.borrow_mut() = None;
        *self.last_query.borrow_mut() = None;
        self.traditional_map.borrow_mut().clear();
    }

    pub fn set_persona(&mut self, spec: Option<PersonaSpec>) {
        let boost = spec.as_ref().filter(|s| s.boost > 0.0).map(PersonaBoost::new);
        self.persona_dictionaries = spec.map(|s| s.dictionaries).unwrap_or_default();
        self.overlay.replace(LayerId::Persona, boost.map(|b| Box::new(b) as _));
        self.overlay.reset(ResetReason::PersonaChanged);
        self.forget_span_cache();
    }

    pub fn set_hint_index(&mut self, index: Option<HintIndex>) {
        self.overlay.replace(LayerId::Hint, index.map(|i| Box::new(HintLayer::new(i)) as _));
    }

    /// 回放 / 调试用：逐层开关。
    pub fn set_overlay_enabled(&mut self, id: LayerId, enabled: bool) {
        self.overlay.set_enabled(id, enabled);
        if id == LayerId::Learner { self.learner.inner_mut().suspend_overlay(!enabled || self.private); }
        self.forget_span_cache();
    }
}
```

`PersonaBoost::rewrite`：

```rust
fn rewrite(&self, _cx: &LayerContext<'_>, items: &mut Vec<Candidate>) {
    let lift = (self.boost * PERSONA_MAX_LIFT as f64).round() as usize;
    if lift == 0 { return; }
    let end = items.len().min(PERSONA_WINDOW);
    // 从前往后扫，被提过的位置不再参与；下标 0（首选）不动：输入优先，首选稳定
    let mut i = 2;
    while i < end {
        let c = &items[i];
        let eligible = matches!(c.kind, CandidateKind::Chinese | CandidateKind::Cloud) && self.words.contains(&c.text);
        if eligible {
            let target = i.saturating_sub(lift).max(1);
            items[target..=i].rotate_right(1);   // 稳定上提，其余相对顺序不变
        }
        i += 1;
    }
}
```

### 3.5 私密一次性旁路（set_private）

WeType 没有运行时总闸（只有建会话时 `disable_user_dict` 让用户词库层不套）。青简在三个入口各判一次，**不在层里判**：

1. `OverlayStack::rewrite / hint / on_commit` 第一行 `if private { return / None }`——L1、L2 及以后任何新层自动旁路；
2. 学习器链：写由 `MutedLearner` 挡（存量），读由 `suspend_overlay(true)` 挡（新增，`set_private_from` 里调）；
3. 人设词库：`all_dictionaries()` 里 `!self.private` 才加。

`reset` 不受私密影响（照 WeType 一律下传），保证私密期间换分区也能清干净。

### 3.6 与青简约束的对照

- Core 平台无关：所有类型在 `qingjian-core`；包从哪来、解密、上传队列都在壳 / cloud-bridge，Core 只收 `PersonaSpec`、`HintIndex`、`LearningScope`。
- 一个类型一个文件、子模块用目录：见 3.3 的文件表；`Engine` 的新方法放 `overlay/setup.rs`（「大类型的 impl 按职责拆」）。
- 输入优先：三层都是纯内存、同步、无 IO；`HintIndex::lookup` 只扫末尾 24 字，预算亚毫秒；`OverlayStack::new()` 空栈时 `rewrite` 是空循环。
- 不跨分区学习：`set_learning_scope` 清 `recent_commits` / `chain` / `history`，杜绝旧分区的撤销与上文落到新分区。
- 用户可感知行为（提示行、`[scene]` 配置）落地时同提交改 `docs/user/`；`docs/design/relationship-memory.md`「Core 的改动」补 3.2 的分层图与 3.3 的签名；
  `docs/notes/crate-notes.md` 记常数与 `--overlay`。
- 偏离设计文档的一处：设计写 `OverlayLearner` 在 Core，分叉已有 `ScopedLearner`（cloud-bridge）实现同一职责，建议 Core 只加 `suspend_overlay` 钩子，不再重复实现。

---

## 4. 复刻步骤

1. 建 `engine/overlay/` 目录与 3.3 的类型（先不接线），`cargo test -p qingjian-core overlay` 跑单测。
2. `Learner::suspend_overlay` 缺省方法；`ScopedLearner` 实现它（读全局、`scoped_*` 返回 `None`）。
3. `Engine` 加三个字段；`query()`、`commit`、`all_dictionaries()`、`privacy.rs`、`session.rs` 按 3.4 接线。
4. `overlay/setup.rs` 写 `set_learning_scope` / `set_persona` / `set_hint_index` / `set_overlay_enabled`。
5. `apps/cli` 加 `--overlay`、`--persona <dir>`、`--hints <file>`（叠加表与卡片先读本地明文，同设计 P0）；回放两份日志确认场景不串。
6. 用回放扫 `PERSONA_WINDOW`、`PERSONA_MAX_LIFT`、`HINT_MIN_SCORE`，结果记进 crate-notes。
7. 改 `docs/design/relationship-memory.md`「Core 的改动」与 `docs/notes/crate-notes.md`。

单测清单（输入 → 期望）：

| # | 测试 | 输入 | 期望 |
|---|---|---|---|
| 1 | `stack_skips_all_layers_when_private` | 栈里放一个把候选全清空的桩层，`rewrite(private=true)` | 候选不变；`hint` 为 `None`；`on_commit` 桩计数 0 |
| 2 | `reset_reaches_disabled_layers` | 关掉 Persona 层后 `reset(ScopeChanged)` | 桩层的 reset 计数 1 |
| 3 | `disabled_layer_is_pass_through` | `set_enabled(Persona,false)`，候选 [A,B,C,人设词D] | 顺序不变 |
| 4 | `persona_lifts_within_window` | boost=1.0（lift=2），[A,B,C,D(人设)] | [A,D,B,C] |
| 5 | `persona_never_moves_first` | [A,D(人设),B] | [A,D,B]（D 已在 1，不越过 0） |
| 6 | `persona_ignores_outside_window` | 人设词在下标 9 | 不动 |
| 7 | `persona_ignores_english_and_sentence` | 人设词的 kind 为 English / Sentence | 不动 |
| 8 | `persona_zero_boost_installs_no_layer` | `set_persona(Some(boost=0))` | 栈里无 Persona 层，但词库仍进 `all_dictionaries()` |
| 9 | `hint_requires_threshold` | 卡「她周三考科目二」关键词 {考:0.4, 科目二:0.8}，文本「考得怎么样」 | `None`（0.4 < 1.0）；文本「科目二考得怎么样」→ `Some`，score 1.2 |
| 10 | `hint_scans_tail_only` | 关键词出现在倒数第 30 字 | `None` |
| 11 | `hint_after_commit_cleared_on_reset` | 上屏命中后 `reset(ScopeChanged)` | `hint()` 为 `None` |
| 12 | `private_hides_persona_dictionaries` | 人设词库有「宝」，`set_private(true)` 后查 `bao` | 候选里没有来自人设词库的「宝」（主词库自带的除外，用词库独有词测） |
| 13 | `private_suspends_overlay_reads` | ScopedLearner 对象层把 X 计 10 次，私密后 `weight(X)` | 等于全局计数 |
| 14 | `scope_switch_drops_undo_and_context` | 在 scope A 上屏后切 B，再触发「上屏后删掉重选」 | A 层计数不变、B 层无 unrecord；`history` 为空 |
| 15 | `scope_switch_same_scope_is_noop` | 连续两次相同 `set_learning_scope` | 第二次不清 `history` |
| 16 | `traditional_applies_after_persona` | 繁体开，人设词「后来」被上提 | 输出是「後來」，且位置是上提后的位置 |
| 17 | `custom_phrase_position_wins` | 自定义短语钉在第 2 位，人设词也想上提到第 2 位 | 自定义短语在第 2 位 |
| 18 | cli 回放 | `--replay log --overlay persona=off` 与 `=on` | 两次输出只在人设词的位次上不同；`learner=off` 时与无叠加基线一致 |


---

## 5. 未知

- 部分层的套用条件只做到 [推]（表中标「推」的项），卡在：守卫读的是 `EnvContext` / `CtxConfig` 内部偏移（如 `env+0x188` 平台、`creator+0x7c`），
  没有对应日志串。下一步看 `EnvContext::EnvContext(_wxime_session_config,_wxime_config)` 0x1006ec710 把哪个字段写到 +0x188，以及 `SessionCreator` 构造里 +0x7c 的来源。
- `keyboard_type` 枚举值的名字（3 = 手写、4/8 = 笔画是推断）。Swift 侧 `_wxime_keyboard_type` 的 case 名在 `__swift5_fieldmd` 里没有（C 枚举只有 rawValue），
  可去 Swift 壳里找 `.init(rawValue:)` 的调用点对照 UI 设置。
- `most_likely_filter` 各位对应哪种候选来源：要把 `NeedFilter`（0x100aaef34）里的 source 常数与 `searchers_v2::*CandidateID` 的 variant 下标对上。
- Calendar 的「前 9 个」窗口（0x100c713c4）只是猜，需细读 0x100c7120c 的循环变量。
- 预热阈值单位：取决于 stub 0x102a71c48 是 `system_clock::now` 还是 `steady_clock::now`（查它的 bind 名）。
- 这些都不影响青简落点：青简不复刻 WeType 的具体层，只借「固定层序 + 逐层广播重置 + 单入口旁路」三条组织规矩。
