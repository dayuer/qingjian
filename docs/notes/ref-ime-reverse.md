# 参考输入法 macOS 版（RefIME 2.2.3）静态逆向笔记

分析对象：本机 `/Library/Input Methods/RefIME.app`（`CFBundleShortVersionString` 2.2.3，build 657，Xcode 16.2 / macOS 15.2 SDK）。
方法：只做静态分析——`lipo` 拆 arm64 片、`nm` / `c++filt` / `swift-demangle` 看符号，`strings` 看字符串，`objdump` 反汇编个别函数看常数与调用顺序，
`xxd` 看数据文件头部几十字节，`plutil` / `cat` 看 plist 与 `index.json`。没有运行、调试、抓包 RefIME，没有解密任何数据库，
没有导出词条、模型权重或分词表内容；中间产物都在 `/tmp/ref-ime-rea/`。

置信度标注：**[确]** 有直接证据（符号名、字符串、反汇编常数）；**[推]** 由多条证据推出，逻辑上很可能；**[猜]** 只有名字或体积上的线索。
地址都是 arm64 片（`RefIME.arm64`）里的虚拟地址。

## 概览

RefIME 是「C++ 跨平台引擎 + 薄 Swift/AppKit 壳」的结构，和青简的「Core 平台无关、各平台只做壳」同一路数，但体量大一个数量级。
引擎 `wxime`（源码根 `ime/wxime/`，约 1.1 万个符号）通过 49 个 `wxime_*` C 函数暴露给 Swift 壳；
引擎里同时塞着手机端才用的东西（九键 T9、手写、触摸坐标纠错、键盘热区），说明 Mac 版直接复用了移动端那套 Core。
候选生成是**音节图 + LOUDS trie 上的「增强器」组合搜索**（纠错、模糊音、补全都写成可组合的 augmenter），
整句是 **bigram 词图 beam search**，再叠三层模型：**3 层 GPT-2 词级 NNLM（TFLite）重排整句路径**、
**LightGBM lambdarank（179 维特征）做候选 LTR**、以及一个**端到端 decoder-only 模型（12 层 / 768 维 GPT-2 结构、int4 分组量化，约 1 亿参数，Metal 推理）**
异步召回整句候选。云端走 Cronet + WebSocket 长连接上的 protobuf 命令，**云候选默认 200 ms 超时**，超时后到的云结果只能插到第 4 位以后。
用户词库与学习数据存在 LevelDB，支持跨端同步（全量 + 增量）；静态词库是 30 来个自定义二进制文件，靠 `index.json` 管版本，
在线更新支持 bsdiff 增量补丁。候选窗是 AppKit 自绘 `NSPanel`（不是 `IMKCandidates`，也不是 SwiftUI）。

## 1. 引擎与模型架构

### 1.1 分层：C API 是唯一门面 [确]

- 引擎源码路径嵌在二进制里（日志宏带 `__FILE__`）：`/Volumes/data/workspace/ime/wxime/{c_api,core,engine,data,kv,network,logging,utils,base}/…`，
  壳在 `/Volumes/data/workspace/WXKeyboardMac/RefIME/Module/…`。共 896 条源码路径（`/tmp/ref-ime-rea/srcpaths.txt`）。
- 壳只能经 `ime/wxime/c_api/wxime.cpp` 导出的 49 个 C 函数进引擎：`wxime_initialize` / `wxime_create_session`（0x100652dec）/
  `wxime_process_input`（0x1006585c4）/ `wxime_candidate_next`（0x100657ee4，候选是迭代器按需拉）/ `wxime_select_candidate`（0x10065a844）/
  `wxime_drop_last` / `wxime_set_text_around_cursor` / `wxime_get_most_likely_sequel`（联想）/ `wxime_cloud_llm_search_without_session` /
  `wxime_config_dict` / `wxime_add_cell_dict` / `wxime_warm_up_session` / `wxime_translate` 等。
  事件经回调结构体送回壳，ObjC 类型编码里能看到 `_wxime_candidate_list_updated_event`、`_wxime_cloud_general_candidate_event`、`_wxime_text_around_cursor_info`。
- Swift 侧 `Engine`（`Module/Engine/Engine.swift`）是 C API 的包装：`createSession(bundleID:…)`、`processInput(session:input:)`、`textAroundCursor(session:beforeText:afterText:reason:)`、
  `addCommonDicts` / `addCellDicts`、`warmUpDict(with:)`、`searchSuggestion(beforeText:sessionId:)`。
- **会话是装饰器链**。`wxime::core::SessionCreator::operator()`（0x1006afbf8，约 50 KB）按顺序构造几十个 `ISession` 包装层，调用序列（节选）：
  `WubiSessionV2` → `StrokeSession` → `UncommonWordFilter` → `PcEmojiMatcher` → `UserHotWordSession` → `Calendar` → `MostLikelySequel` → `NumberCalculator`
  → `CloudSearchByInputEn` → `CloudLlmSearch` → `CloudTextRecommend` → `CloudTextCorrect` → `CloudReportEmitInput` → `LocalTextCorrect` → `HotFix` → `VModeV2`
  → `SessionKeystrokeReporter` → `EnglishTee` → `CloudMostLikelyRelatedMergerV2` → `AutoMostLikely*` → `SimplifiedToTraditional` → `MostLikelyFilter`
  → `SelectDefaultCandidate` → `BatchEmit` → `WatchDogV2Session` → `UserSingleCnSelectWrapper` → `UserCommonWords` → `AsyncLightGBMManager::GetAndLoad` → `PinyinV2Session`。
  每个功能（云、繁简、emoji、日历、计算器、敏感词拦截、上报）都是一层 wrapper，核心拼音搜索在最底层 `PinyinV2Session`。[确：调用序列；推：链的嵌套方向]
  上面列的是代码书写顺序；逐函数读过之后，嵌套是先写的在外、拼音核心最先构造在最里，共 105 层，见 [ref-ime-impl/a3-session-layers.md](ref-ime-impl/a3-session-layers.md)。
- **会话异步化**：`AsyncSessionV2` 用 asio `io_context` + 一个 `std::variant` 任务队列，任务类型有 `ProcessKey`、`JobSelectCandidate`、`JobGetMostLikelySequel`、
  `JobSetTextAroundCursor`、`JobGetCloudResult`、`JobWarmUp`、`JobProcessHwPoint`（手写点）、`JobSetKeyBoardLayout` 等 17 种（符号里的 mangled 类型）。
  结果以 `CandidateUpdatedEvent` / `CloudResultEvent` / `EmitInputEvent` 回调。另有 `TTLSessionManager` / `AsyncDestroySessionMgr` 管会话生命周期。[确]
- 第三方：abseil、range-v3、cppcoro（协程生成器，候选流是 `cppcoro::recursive_generator<CandidateV2>`）、capnproto（超参）、protobuf-lite（云协议）、
  leveldb、LightGBM、sentencepiece、TFLite（含 ruy / gemmlowp）、nlohmann json、loguru、asio。[确]

### 1.2 拼音解析与切分：音节图 + LOUDS trie + 增强器 [确]

- 两代图并存：`graphs_v2::SyllableGraph`（`SyllableEdge`、`SimplifiedSyllableGraph`、`FullyCompletionSyllableEdge`、`SyllableShrinker`）
  与 `graphs_v3::SyllableGraphV3` / `RuneGraph` / `DoublePinRuneGraph`（Rune = 按键字符级图，双拼单独一张图）。
  开关 `mmkb_cli_engine_graph_v3_disable`、`mmkb_cli_engine_graph_v3_force_with_fuzzy`。
- 拼音音节表存成 **LOUDS trie**（`wxime::data::trie::LoudsTrie`，2.4 万个相关符号），查找入口 `graphs_v2::SyllableEdge::SearchFromLeft`（0x1009a4b40）。
- **纠错 / 模糊音 / 补全统一成「增强器」**：`SearchImpl<LoudsTrie::Cursor, AugmentRecord, …, IterativelyInputAugmenters<…>>` 是一个模板，参数包里 28 种增强器，原样列出：
  `ExactMatch`、`InsertSep`、`PrismCompletion`（前缀补全）、`Exchange`（相邻换位）、`KeyboardNeighbour`（邻键）、`InsertAtMiddle`（漏敲，补一个字母）、`Skip`（多敲，跳过一个字母）、`Consonant`、
  `XnToXngNasal` / `XngToXnNasal`（前后鼻音）、`Retroflex`（平翘舌）、`HuiFei`（h/f）、`HuangToWang` / `WangToHuang`、`UnToOng` / `OngToUn`、`UnToIong` / `IongToUn`、
  `AnAi`、`EngOng`、`FullKBShiftKey`、`YvLike`、`LveAndNve`、`DoublePin`、`DoublePinPrismCompletion`、`DoublePinCombineFuzzy`、`ProcessUpperCase`、
  `KeyboardNeighbourForDoublePin`、`EditPinyinOnce`（查 `EditPinyinDict`，对应数据文件 `editpinyin.bin`）、`RepeatPrevChar`。
  每个增强器在 trie 游标上产生带标记的 `AugmentRecord`，记录「这条路径用了哪种纠错」，后面变成排序特征（见 1.4）。
- `graphs_v3` 另有 `FuzzyVowelHelper` / `FuzzyConsonantHelper` / `FuzzyWordHelper` 与 `TypoRecord`；`PrismV2`（`prism_v2.bin`，12 KB）是补全用的小表。[推：prism = 音节前缀补全表]
- 音节图建图有独立线程池（`AppendPinyinSession(CtxConfig)::graph_thread_pool`，静态变量 0x1033d49d8）。[确]

### 1.3 候选生成与整句 [确]

- 候选来源用一组 `*CandidateID` 类型区分（`searchers_v2`）：`PinyinCandidateID`、`TrieMatchingCandidateID`、`TrieMatchingWithSyllableCorrectionsCandidateID`、
  `CandCompletionCandidateID`、`End2EndCandidateID`、`CloudPinyinSearchCandidateID`、`CloudMostLikelySequelCandidateID`、`UserHotWordCandidateID`、`EmojiCandidateID`、
  `CalendarCandidateID`、`NumCalculateResCandidateID`、`WubiCandidateID`、`StrokeCandidateID`、`HandWritingCandidateID`、`T9NumberRelatedCandidateID` 等 30 种。
- 词搜索：`WordSearchBySyllableGraph`、`SecondLevelWordSearchBySyllableGraph`（二级词库，见第 2 节）、`CorrectedSyllableDictSearcher`、`UserDictSyllableGraph(V2)`、`PredictedWordSearcher`。
- **整句**：`SentenceSearcherV3` 的构造参数是 `(TextAroundCursorContext, lm::bi_gram::Reader, IDict, …, HyperParams, EnvContext, ModelParamsReader, DynamicCalculator, NNLmModel, threadpool)`，
  方法有 `GenerateBeamsV*`、`FeedBeam`、`BestPath`、`ShrinkBeamDuplicatedBiGram`、`InferTextBeforeByNNLM`、`RemoveNotValidUserWordsFromBeamSearch`。
  即：**bigram 词图上 beam search，按光标前文本做上下文**。语言模型 `wxime::lm::bi_gram`（`BiGramPlainPredictor`，另有 `NGramPlainPredictor`），数据 `lmcnmac.bin`（16 MB）。[推：文件对应]
- **beam 宽度是调度出来的**：`BeamSizeSchedulerFactory` 下有 `CosineDecay`、`ExpDecay`、`WarmUpCosineDecay`、`WarmUpNegativeExpDecay`、`NonlinearDecay`、`Sequence` 几种 functor，
  超参 `BeamSizeSchedulerParams`、`BeamSizeEnRatioParams`、`BeamSizeJianPinRatioParams`、`BeamSizeUserWordsRatioParams`、`FrontierShrinkParams`、`LeavesShrinkParams`。
  即 beam 宽随句中位置衰减，简拼 / 中英混 / 用户词各自调比例。[确：名字；推：语义]

### 1.4 排序：LTR + NNLM 重排 [确]

- `MixRank`（`OnlineMultiFreqLTR`）+ `MixRankerPostProcessorV2`（一串后处理器：`RankModelProcessor`、`RecallModelProcessor`、`TypoCandidateProcessor`、`CorrectedSyllableCandidateFilter`、
  `AdjustUserDictProcessor`、`FirstUserCandidateRankerProcessor`、`OnlineFreqReRankProcessor`、`RemoveDupSentenceProcessor`、`PrecisionLossProcessor`、`EnglishCandsAdjustStep` …）。
- 模型：`LightGBMPredictor`（`HighDeviceLtrValid` 说明按设备性能分档）、`features::LRModel`、`features::OneLayerNNModel`、`AsyncLightGBMManager`。
  `mdltrpc.bin`（2.2 MB，`index.json` uniqueID `mdltr`）里有字符串 `lightgbm_feat179`、`lambdarank7`：**LightGBM lambdarank，179 维特征**，PC 专用一份。[确：字符串；推：就是这个文件]
- 特征名直接在符号里（`wxime::engine::features::*_feature` / `*_abbr`）：`exact_with_typo`、`exact_after_typo`、`exact_with_fuzzy`、`multi_exact_with_typo`、`lm_exact_with_typo`、
  `fjp_comp`、`dict_fjp`、`lm_fjp`、`user_typo`、`user_sjp`、`kb_neighbour_abbr`、`insertion_abbr`、`skip_abbr`、`transfer_abbr`、`fuzzy_abbr`、`completion_abbr`、
  `coordinate_score_abbr`、`dynamic_correction_score_abbr`、`consistent_suffix_abbr`、`t9_rpl_*` 等。特征分 `SyllableFeature` / `WordFeature` / `PathFeature` / `CandidateFeature` /
  `CandidateRecallFeature` 五级，且有 `InputScene` / `kScenePriorityDict`（按场景分权）。
- **NNLM 重排整句路径**：`searchers_v2::NNLMRanker::ReRankPaths`（0x100c61f88，内部 `stable_sort`）与异步版 `AsyncNNLMRanker::ReRankPaths`（0x100c6ae58），
  还有 `ShouldUseNNLM`、`NormNNLMScore`、`LTRInfer`（NNLM 分进 LTR）。模型 `nlrgpt2_nnlm.bin`（16 MB）：12 字节自定义头 + TFLite flatbuffer（`TFL3`），
  图里只有 `h_._0`～`h_._2` 三个 block、`wte` 共享做输出头，输入 `input_ids` + `past`（KV cache）。即**3 层 GPT-2，词级（`NNLmWordMapDict` 做词 ↔ id）**。
  推理两套后端：`NNLmModelImplTFLite` 与 `NNLmModelImplXNet`；`StreamingNNLMModel` 有 `SetTextBeforeCache` / `LookUpLMScoreInCache` / `BatchInferWithCache`：光标前文本的状态缓存复用。
- 联想也用它：`model::AssociationNNLmRerank::CandidatesRerank`、`AssociationPersonalModel`（个人化联想打分）。

### 1.5 端到端整句模型（E2E / decoder-only）[确 + 推]

- 三类模型：`E2ENNModel`（encoder + decoder，另有 `ChEnMixPredictor`）、`DecoderOnlyNNModel`（CPU，走 XNet）、`DecoderOnlyGPUNNModel`（GPU，走 `xllm::LLMModel::CreateFrom` → `RciLLM` Metal 后端）。
  反汇编调用序列：`DecoderOnlyNNModel::Init`（0x100eeb504）→ `DecoderOnlyNNResourceReader::ViewConfigBuffer / ViewVocabBuffer / ViewDecModelBuffer` → `xnet::dnn::DnnModel::LoadBin` → `PrefillStartToken`；
  `DecoderOnlyGPUNNModel::Init`（0x100efb9d4）→ `ViewDecGPUModelBuffer` → `xllm::LLMModel::CreateFrom`。
  解码有 `BeamSearchDecodeText`、`PrefillContext`、`CopySavedCacheToTensor`（KV cache 复用），结果经 `searchers_v2::RecallE2EModelResultAsync` **异步**召回成 `End2EndCandidateID` 候选。
- `decgnr.bin`（74 MB，uniqueID `e2enr`）文件头魔数 `XLLM`，紧跟一段 JSON 配置（只读了配置）：`num_hidden_layers 12`、`hidden_size 768`、`num_attention_heads 12`、`vocab_size 10496`、
  `max_position_embeddings 512`、LayerNorm + 学习式位置嵌入（张量名 `pos_embeddings`、`attention.qkv_proj`、`mlp.up_proj/down_proj`、`lm_head`），即 GPT-2 结构。
  内嵌 `BertTokenizerFast` 配置，特殊 token 有 `[T1]`～`[T9]`。[确]
  - **量化 [推，中高]**：按张量名偏移量算，`tok_embeddings` 段 ≈ 10496×768×2 字节、`pos_embeddings` 段 = 512×768×2 字节（fp16）；
    每层 `qkv_proj.weight` 段 996352 字节，对应 768×2304 个参数 ≈ 0.563 字节/参数 = **int4 + 每 32 个一组的 fp16 scale**。
    总参数约 1 亿（12 层 × 7.1 M + 嵌入 / 输出头约 16 M）。
  - `[T1]`～`[T9]` 多半是九键输入的键 token，词表 10496 是字级中文 + 拼音字母。[猜]
- `mpd_e2e.bin`（3.7 MB，同为 `dict_type 5`、`special_dict_kind 1`）多半是 CPU（XNet）版的小模型或其配套资源。[猜]
- 推理框架：**XNet**（源码 `CompileXNetScript/XNet/…`、命名空间 `xnet::`，CPU 后端 `xnet::uarm` 支持 fp32 / fp16 / s8，GPU 后端 Metal / OpenGL / OpenCL / Vulkan，
  带 ONNX 转换器 `OnnxConverter` 与 CoreML 规格解析）是聊天应用自研推理框架 [推]；**RciLLM / xllm** 是其上的 LLM 运行时（Metal FlashAttn / FlashDec / Gemm / Gemv / TopK 内核，
  支持 GPT2 / Qwen / Qwen2VL / Llama2 / ChatGLM / Baichuan2 / InternLM2 / Hunyuan / RWKV7 等模型族与 LoRA、MoE）[确：符号]。
  链接了 `CoreML.framework` 且引用了 `MLModel` 类，但用在哪（XNet 的 CoreML 后端还是语音）没查清；输入法模型的加载路径里没看到它。
- 没有看到 MNN、ncnn、onnxruntime。

### 1.6 用户学习与联想 [确]

- 用户数据都在 `wxime::data::dict::User*`：`UserDictV2`（主用户词）、`UserBigramDict` / `UserAssociationRW`（用户 bigram 与联想）、`UserHotWordDict`（热词）、
  `UserWordWithSyllableCorrectionDict`（纠错后的用户词）、`UserCorrectedTextDict`、`UserHardFixDict`（固顶）、`UserBlockListDict`（屏蔽词）、`UserCommonWordsDict`、
  `UserSingleCnSelectDict`（单字选择记录）、`UserRecentWordsDict`、`UserScenesDict`、`MemCacheUserCoordinateDict`（触摸坐标，手机用）、`UserEmojiFreqDict`、`UserStrokesDict`、`UserWubiDict`。
- 清理策略：`UserDictCleanStrategyMaker` / `PotentialUserDictCleanStrategy` / `DynamicTargetUserDictCleanStrategy`，开关 `mmkb_cli_engine_user_dict_clean_strategy`、`…_roll_back`。
  `UserHotWordDetector` / `UserHotWordClassifier`：自动发现用户热词。`MostLikelyPersonalRecall`：个人化联想召回。
- 联想：`ZhMostLikelySequelSearcherV2`（`LastTextAssociation`、`PoemAssociation`、`NumberAssociation`、`EmojRecommend`、`SymbolRecommend`、`SupplementWithPersonal`、`NNLmRerank`），
  数据 `PoemAssociateKVDict`、`SyllableAssociationDict`、`AssociateKVDict`。

## 2. 词库与数据格式

### 2.1 随包数据 `Resources/imeData.bundle`（294 MB）[确]

`index.json` 列出 31 个文件，每项有 `name`、`md5`、`type`、`uniqueID`、`version`、`timestamp`、`is_cell`、`support_update`、`dict_type`、`special_dict_kind`。
文件名没有硬编码在二进制里，引擎按 `type` 识别。`type` 的高 16 位像是大类（`0x1` 词库、`0x2` 语言模型、`0x4` 模型 / 参数、`0x8` 五笔）。[推]

| 文件 | 大小 | 推测用途 | 依据 |
| --- | --- | --- | --- |
| `bwcnmac.bin` / `bwcnmacv8.bin` | 30 / 32 MB | 中文主词库（v8 新格式） | `ChineseStaticDictV2/V3`，uniqueID `bwcn` |
| `slwd_pc_second_level.bin` | 52 MB | 二级（低频）词库 | `SecondLevelDict` + `second_level_dict_value_huffman_codec.cpp`，`dict_type 7` |
| `ckvdmac.bin` | 40 MB | 大 KV 词库 | `CompactKVLargeDict` / `TrieIndexLargeDict`，`dict_type 4` |
| `lmcnmac.bin` | 16 MB | bigram 语言模型 | `type 0x20000`，`lm::bi_gram` |
| `nlrgpt2_nnlm.bin` | 16 MB | 3 层 GPT-2 词级 NNLM（TFLite） | `TFL3` 魔数，`dict_type 8` |
| `decgnr.bin` | 74 MB | 12 层 decoder-only E2E 模型（XLLM，int4） | `XLLM` 魔数 + JSON 配置 |
| `mpd_e2e.bin` | 3.7 MB | E2E 相关（猜 CPU 版） | `dict_type 5` |
| `mdltrpc.bin` | 2.2 MB | LightGBM LTR | `lightgbm_feat179` |
| `bwcjpmac_jianpin.bin` | 12.6 MB | 简拼词库 | 文件名，`dict_type 9` |
| `bwcnenmac.bin` | 6 MB | 中英混输词库 | uniqueID `bwcnen` |
| `swbd86/98/nc.bin` | 各 4 MB | 五笔 86 / 98 / 新世纪 | `type 0x80003` |
| `stt.bin` / `tts.bin` | 1.2 MB / 19 KB | 简繁转换 | `SimplifiedToTraditional` |
| `editpinyin.bin` | 0.7 MB | 拼写编辑纠错表 | `EditPinyinDict` |
| `sepd.bin` | 0.9 MB | 拼写结尾预测 | `spelling_end_predict_dict.cpp`，头部字符串 `Ime trie header` |
| `prism_v2.bin` | 12 KB | 音节补全 | `PrismV2` |
| `noun_tag_dict.bin` | 4 MB | 名词标注 | 文件名 |
| `wkvc2e.pc.bin` / `wwkvc2e.bin` | 1.1 MB / 0.1 MB | 中文 → emoji | `EmojiC2EKVDict` |
| `hpmac_base.bin` | 1.8 KB | 超参（HyperParams） | uniqueID `hp`，`dict_type 6`；超参用 capnp 读 [推] |
| `cell_dict.{sc,hf,ssrc}.zh` | 32 KB～1 MB | 细胞词库 | `is_cell: true`，ssrc 头部 `Ime trie header` |

- 容器格式是自定义二进制，**不加密** [推]：多个文件头是 `01 00 00 80` + 条目数 + 偏移表（`lmcnmac`、`wsd`、`editpinyin`、`mpd_e2e`、`noun_tag_dict`、`wkvc2e`、`stt` 相同），
  另一族头部带 `Ime trie header` 字符串。引擎里的 AES（`wxime::aes::wxime_aes_ecb_decrypt_buf`，0x100fce754）**唯一调用者**是 `UserDictDumper::Load`（用户词库导出 / 备份），不是静态词库。
- 读法是 mmap [确]：`wxime::data::MMap(unique_ptr<FileDescriptor>, bool)`、`FileMemoryBuffer`、`TmpMmapFileMgr`（`/tmp_mmap_` 临时映射文件）。
- 索引结构 [确：类名]：LOUDS trie（`LoudsTrie`、`LoudsTrieHashMapReader`、`LoudsTrieCursorListV2`）、double-array（Darts，来自 sentencepiece）、`ThreadSafeRadixTrieV2`（用户侧可写）、
  `StaticBloomFilterDict` / `LeafFrontierSeparatedBloomFilter`（先过 Bloom 再查）、值用 Huffman 编码（`large_dict_value_huffman_codec.cpp`，报错串 `missing word_huffman key`）、
  分数用 `FP16Reader` / `UInt8Reader` 量化存储。

### 2.2 运行时数据 `~/Library/Application Support/RefIME/`（251 MB）[确]

- `userDict/v5/<时间戳>/`、`userDict/user_hot_word/`：**LevelDB**（`.ldb` / `MANIFEST` / `LOG`），对应 `wxime::kv::LevelDBKV`、`RecoverableLevelDB`（损坏自动修复、上报原因）、
  `TtlDB`、`PrefixDB`、`NaiveEncryptedKV`（KV 层轻加密 [推]）、`DelayLoadedKV`。
- `DictUpdate/DownloadDicts/<名>_bin/<版本>/<名>.bin`：在线更新下来的词库，版本号与 `index.json` 对得上（如 `wsd_bin/1045`），另有随包没有的 `cell_dict_newhot_common_zh`（热词细胞词库）、`slwd_…_pinyin_expt_bin`（实验版）。
- `DataBase/common.db`：文件头不是 `SQLite format 3`，是 SQLCipher 加密库（壳侧剪贴板历史 / 反馈表，`Util/DB/ClipboardHistoryTable.swift`、`FeedbackTable.swift`）。未尝试打开。
- `mmkv/`：厂商 MMKV 存设置（`refime.settings`、`refime.features` …）；`engine/persistent_data/user_dict_clean_up.bin`、`userDict/persistent_data/user_dict_upload.bin`：同步状态。
- `~/Library/Caches/RefIME/Logs/*.xlog`、`*.mmap3`：聊天应用 xlog 格式日志（mars）。

### 2.3 更新机制 [确]

- 词库：壳侧 `DictUpdate/{DictsUpdateChecker,DictsDownloader,DictsUpdater,DictsManager}.swift`。日志串：`[Dict][Update] check dicts update`、`GetImeDictListConfigResp`、
  `GetImeNewHotDictConfigResp`、`[Dict][Download] download full dict` / `download patch dict` / `did unzip patch dict`、`[Dict][Main][reloadDicts] cellHash different or dictChanged`。
  补丁用引擎内的 `bsdiffplus::BsPatch`。还有 `maybeCrashDictsHashes`：**加载某版词库后崩溃，就记下它的哈希不再加载**。
- App 本体：Sparkle 2（`SUPublicEDKey` ed25519）+ 自己的 `PatchUpdater` 与 `RefIMEUpdater.app`，`Resources/BinaryDelta` 是 Sparkle 的二进制差分工具（给 App 包，不给词库）；
  `update.sh` 先 `codesign --verify --deep --strict` 再核对 Team 身份，然后原子换 `Contents`。更新源 `<厂商域名>`（另有 `…/localmode/updates.xml`）。

## 3. 云端交互

### 3.1 传输层 [确]

- `wetap::net::CronetNetworkApi`（Chromium 网络栈）+ `WcwssAPI`（`wcwss.framework`，内含 OpenSSL）走 **WebSocket 长连接**；
  主机在 `CronetNetworkApi::InitWssApi`（0x100586040）/ `GetHost`（0x100590edc）里引用 `<厂商域名>` 与 `<厂商域名>`（后者推测为测试 / 备用）。
- 握手有 `wetap.net.ECDHPubKeyReq_V2` / `GenUinReq_V2`：先 ECDH 换密钥、再领匿名 uin，应用层再加密一层 [推]。超时检测与重连日志：`Task Timeout idle detected … trigger reconnect`。
- 业务包是 protobuf-lite：引擎侧 `mmime.BaseCmdReq/BaseCmdResp`（`ReqBody` / `RespBody`、`LlmBaseReq`、`TextRecommendResult`、`TextCorrectorResponse`、`NamedEntityRecognitionResult`、`PredictNextNResult` …），
  用户数据侧 `mmkb.*`（词库同步、热词、`ReportEmitInputReq`），搜索 `mmkbsearchsvr.SearchReq`。壳侧还有 `request(_:subCmd:data:timeout:…)`：按 cmd / subCmd 分发。
  lite 版不带描述符，字段名看不到。

### 3.2 什么时候请求、怎么合并 [确 + 推]

- 云功能各是会话链上的一层：`CloudSearchByInputV2`（按输入搜云候选，`NeedDoRequest` 会校验 `IsValidNormalPinyin` / `IsValidNormalT9Pinyin`、`IsHotFix`，
  请求前 `EnumPinyinSplits` / `EnumPrefixCompletionSplits` 把多种切分一起发、`CancelSameReq` 去重、`TryResponseByCache` 先查本地缓存）、`CloudSearchByInputEn`、
  `CloudMostLikely` / `CloudMostLikelySerial`（云联想，按光标前文本）、`CloudTextRecommend(V2)`、`CloudTextCorrect`、`CloudLlmSearch`、`cloud_search::AskAI` / `GeneralSearch`。
  上文长度有上限开关 `mmkb_cli_engine_before_context_limit_for_cloud_input`。
- **云候选超时**：`CloudInputMerger::Impl` 构造函数（0x100c0c018）在 0x100c0c0e0 把字段 `+0xa0` 置为 **200**，随后读 AB 参数 `mmkb_cli_engine_cloud_time_out_ms`，
  `stoi` 成功且 ≥ 1 时在 0x100c0c37c 覆盖；`isOutOfTime`（0x100c0e208）用计时器纳秒值与 `timeout × 1e6` 比较。所以**默认 200 ms，可被服务端 AB 改**。
  另一开关 `mmkb_cli_engine_enable_cloud_time_out` 决定是否启用超时。
- **合并**：`CloudInputMerger::Impl` 的方法有 `Merge` / `MergeImpl` / `mergeSort` / `mergeExpPos` / `MergeTimeoutCloudFromFourth` / `LocalFirstTop` / `IsProtectLocalFirst` / `IsHardFixedByUser` / `TryUpdateCloudPendingInput`。
  `mergeExpPos(local, cloud, LocalFirstTop1Info, bool, CloudSearchByInputExtraInfo)` 的签名说明**云结果带着服务端给的期望位置**；
  **超时后才到的云结果默认丢弃；AB `mmkb_cli_engine_enable_cloud_time_out` 为 "true" 时才走 `MergeTimeoutCloudFromFourth`，从第 4 位往后插**（逐条读过，见 [ref-ime-impl/b5-late-merge.md](ref-ime-impl/b5-late-merge.md)）；
  本地首选、用户固顶词受保护（`IsProtectLocalFirst`、`IsHardFixedByUser`，AB 开关 `mmkb_cli_engine_cloud_local_merger_protect_user_expt`、`mmkb_cli_engine_pc_allow_cloud_force_top1_expt`）。
  云结果可以改写拼音栏（`TryUpdateCloudPendingInput`）。名实体结果会进「潜在用户词」（`AddNameEntityResultsToPotentialUserDict`）。
- 敏感词：`InterceptSensitiveResult` 同时拦本地与云结果（`StaticInterceptSensitiveDict` + 可远程下发的 `DynamicInterceptSensitiveDict`）。
- 壳侧还会从聊天应用 / 工作聊天应用取聊天上下文喂云端（`fetchXChatContextIfNeed`、`isChatSession`、`fetch_xchat_context_size`），并用 AX API（`AXSelectedTextRange`、`AXStringForRange`）读光标前后文本。
  聊天上下文**不是靠 AX 读窗口，而是和聊天应用进程合作的私有 IPC** [确：反汇编调用序列]：`AppCommunicator`（`connectChat` 0x10037cea4、`postNotifToChat` 0x10037e570、
  `recieveFromChat` 0x10037bfa4、`_fetchXChatCursorContext` 0x10037fb70、`handleCursorContextResponse` 0x10037de64）经 `NSDistributedNotificationCenter`（主程序导入了该类，
  字符串 `RefIME.DistributedNotification`）收发，先 `SayHello` 握手、`checkChatTokenStatus` 验令牌、载荷用 `chatEncryptKey` 加密；
  只对 `com.vendor.xinChat` / `xChat*` / `WorkChatMac` 生效，日志 `[XChat] fetch fail: Chat not connected`；拿到后 `ContextText.updateXChatContext` 写进上下文，
  再触发联想与「问 AI」建议（`showAskAIAfterFetchXChatContext`）。开关 `disable_fetch_xchat_context`、`chatSupportFetchContext`、`isChatVersionSupport`。
  同一通道还用来发表情包到聊天应用（`sendStickerToChat`）。
  - 通知名 [确]：静态初始化（0x10037b530 起）写死 `RefIME2Chat_channel` / `Chat2RefIME_channel`，工作聊天应用 `REFIME2WECOM_NOTI` / `WECOM2REFIME_NOTI`，
    设置程序 `RefIME2Settings_channel` / `Settings2RefIME_channel`；`start()`（0x10037b750）对 `NSDistributedNotificationCenter` 注册 `recieveFromChat:` 等回调，
    发送用 `postNotificationName:object:userInfo:deliverImmediately:`。
  - 握手 [确]：`WTSayHelloRequest`（`token`、`version`）→ `WTSayHelloResponse`（`emoticonDirectory`、`supportSendInput`），令牌存 `chatCommuToken`，带 `chatTokenExpiredTime`。
  - 请求 [确]：`_fetchXChatCursorContext` 建 `WTCursorContextRequest`，`setContextSize:` 取 AB 值 `fetch_xchat_context_size`，`setIsNeedCursorPost:`，
    序列化后装进 `RefIMECommunication` 信封（`requestId`、`body`、`type`），用 `CAESCrypt fastAESEncryptWithKey:Data:` 加密 body 再发。
  - 密钥 [确]：`chatEncryptKey`（0x100381c88，与工作聊天应用共用一份合并函数，`StringMixUtil` 的选择子作参数传入）= `md5(StringMixUtil.type1(chatCommuToken))` 的 UTF-8 字节，
    即不握手拿不到令牌就解不开载荷。`type1:` 是 C++ 字符串拼接后 `stringWithFormat:`，混淆细节没读。
  - 回复 [确]：`WTCursorContextResponse` 的 `cursorPos` 与 `bytesBeforeCursor`。**回复里是输入框光标前文本还是带了聊天消息，由聊天应用一端决定，RefIME 二进制里看不出**，要逆聊天应用对这两个通知的处理才知道。[未知]
  - 触发 [确：字符串]：拿联想时（`trigger contextText refetchIfNeed, from getSuggestions in chat`）与聊天应用里按 Shift；按来源节流（`allLastFetchXChatContextTimes`）；
    Qt5 版聊天应用不支持（`disable_refetch_in_xchat_qt5`）。
  - 方法：REA（Ghidra 12.1.4）对整个 arm64 片单函数反编译两次都超时（330 s），以上是 `otool -tV` 按 nm 符号边界截取的反汇编；未运行、未抓包。

### 3.3 隐私相关上报 [确：存在；未知：开关与默认]

- `CloudReportEmitInput`（`cloud_report_emit_input.cpp`）：每次上屏 `OnEmitInput` → `BuildItemFromEmitInput` → `AppendItemToKV`（先落本地 LevelDB 队列）→ `FlushToCloud`（`mmkb.ReportEmitInputReq`），
  有 `pending items limit reached`、`truncated item` 保护。**上屏内容以某种形式上传**，字段是否脱敏 / 是否需用户同意 / 是否 AB 抽样，静态看不出。
- `logging::CollectAndReportBiGram`（`ReportOnScreenTextWithPreContext`、`ReportBiGram`，日志串 `across screens bigram`）：统计上屏文本 bigram 并上报。
- `ReportEmittedInputWordsWrapper::statBigram`、`StatUserActionsWrapper`、`SessionKeystrokeReporter`、`UserActionReportSample`、`EmitAndDropReporter`：用户行为与按键统计。
- `logging::NetworkAppender`、`InfluxDbAppender`：日志 / 指标直接发网络。`wetap.business.proto.report.BusLogReq`：业务日志。
- Sentry（`Pods/Sentry`，DSN 不在字符串里，推测远程下发）做崩溃与 APM。
- **`flurry.framework` 不是 Yahoo Flurry 统计 SDK**：导出符号是 `flurry::FlurryWXP`、`IFlurryWXP2PDelegate`、`GrpcCredentials`、`UploadFileInfo`，是厂商自己的 gRPC P2P 文件传输库（「文件传输」功能用）。
- 有「本地模式」（`LocalMode.swift`、`localModeEnabled`）：开了以后 COS 上传、语音、AI 等联网功能跳过（日志 `skip reason=localMode`）。

## 4. IMK 壳与候选窗

- 入口：`Info.plist` 的 `InputMethodServerControllerClass = RefIME.InputController`，`NSPrincipalClass = RefIME.WBApplication`，`LSUIElement`。只注册一个输入模式 `…refime.pinyin`，中英切换在内部做。[确]
- `InputController`（`Module/Input/InputController.swift`，`IMKInputController` 子类）：`handle(_:client:)` 先确保会话存在（`createNewSession(inheritStickerSupport:reason:)`）、
  必要时 `ContextText.refetch(...)` 重取光标上下文，然后交给 `InputEventHandler.handle(event:from:)`。[确：反汇编调用序列]
  其它：`activateServer` / `deactivateServer`（失焦时清理残留输入）、`commitComposition`、`setMarkedText(_:selectionRange:replaceLen:)`（带「替换前文 n 个字」参数，
  闭包里用 `getMarkedRange` / `getSelectedRange` 并同步 `ContextText.replace`）、`emitInputToScreen(_:sessionID:replaceLen:containsChatEmoji:)`、`replaceBeforeText(by:beforeLen:appendText:)`、
  `smartSpaceText`（中英 / 数字之间自动空格）、`tryMapPunctuation`、`switchInputMode(isCapsLockOn:)`。
- `InputEventHandler`：`_handleKeyDown`、`_handleFlagChanged(…isFromGlobalMonitor:isInjected:…)`、`_handleShiftSwitch`（Shift 单击切中英，`lastShiftChangedTime`）、`shouldTurnPage(_:pendingText:)`、
  `dotKeyState`（点号状态机，处理网址 / 小数之类）、语音按住说话（`handlePushToTalkCallback`）。翻页键可配：`turnPageHotKeyBrackets / CommaDot / MinusEqual / ShiftTab`。
  另有 `ShortcutKeyMonitor` / `ExternalKeyboardMonitor` 用全局监听（CGEventTap，需要辅助功能权限）。
- 会话缓存：`InputSession`（`sessionID`、`bundleID`、`pendingInput`、`lastContextBefore`、`engineABParams`）与 `InputSessionCache`，按 App 维持引擎会话，`[MainInput][SessionRecreate]` 日志记录重建原因。
- 候选窗：`CandidatesPanel`（`init(contentRect:styleMask:backing:defer:)`，即 **`NSPanel` 子类**）+ `CandidatesStore`（Combine 绑定 `bindStoreCancellable`，存 `candidates`、`highlightIndex`、`expanded`、`displayMode`），
  视图是 AppKit：`CandidatesSingleLineView`（横排，`CandidateItemView` 里是 `NSTextField` 序号 / 内容 / 来源标签）、`CandidatesVerticalContentView`（竖排，`NSTableView`）、
  展开态 `CandidatesCollectionLayout`（`NSCollectionView`）。**没有 `IMKCandidates`，也没有 `NSHostingView`**（符号里零处）。[确]
  `SuggestionsPanel`（联想 / 建议）、`StickersPanel`（表情包）、`VModeMenu`（v 模式符号面板）、`ListPrefixTipsPanel` 都是各自的 panel。
- 候选拉取：`CandidatesStore.candidateIter` 对应 `wxime_candidate_next` 迭代器，翻页 / 展开时再拉。[推]
- 设置程序是独立的 `RefIMESettings.app`，和主进程靠分布式通知（`Settings2RefIMENotifName`）与共享 MMKV 通信。[推]

## 对青简的启发

对照：青简 `qingjian-core`（音节解析 + 词图 + 整句）、`qingjian-lm`（bigram）、`qingjian-neural`（字级 GPT-2 P2C，candle，重打分 + 自由生成融合）、
`qingjian-predict`（OpenAI 兼容云联想）、`.qj` mmap 容器。

**架构上已经同路、可以放心的**

- 「C API 门面 + 薄壳」：RefIME 的 49 个 `wxime_*` 与青简 `Engine` 门面同构，且 RefIME 证明了「同一 Core 跑手机和 Mac」可行（T9、手写、触摸坐标都在 Mac 二进制里）。
- bigram 词图 + beam 仍是主干，神经网络是叠在上面的重排 / 召回层，不是替代——和青简「神经分只决定名次」的取舍一致。
- 静态数据 mmap、按版本文件管理，与 `.qj` 一致。

**值得借鉴的**

1. **纠错 / 模糊音写成可组合的增强器**：RefIME 把 28 种变换（邻键、换位、漏敲、多敲、前后鼻音、平翘舌、h/f、un/ong …）做成同一个 trie 搜索模板的参数包，
   每条路径记下用了哪些变换（`AugmentRecord`），再变成排序特征。青简现在 `correction`（整段一处编辑）与 `typo`（音节级变体表）是两套机制，
   可以考虑统一成「词图边带变换标记」，模糊音、敲错、补全都只是边上的不同标记与代价。
2. **变换标记进排序特征，而不是只给固定代价**：`exact_with_typo`、`lm_exact_after_typo`、`user_typo` 这类特征交给 LTR 学权重。
   青简目前没有 LTR；短期可以先用 `apps/cli` 的评测集做手调权重表，长期可以考虑 LightGBM 这类小模型（推理成本几十微秒）。
3. **beam 宽按位置衰减**（cosine / exp decay 调度器），简拼、中英混、用户词各自调比例：对长句延迟有直接帮助，青简的整句 beam 可以先试一个简单的线性衰减并用 `p2c-latency-spike` 的场景测。
4. **云候选的时间预算与插位规则写死在合并器里**：默认 200 ms；超时后到的只能排第 4 位以后；本地首选和用户固顶受保护；服务端可以给期望位置。
   青简 `qingjian-predict` 目前是联想，若将来做云候选，这几条可以直接当设计约束（也符合「输入优先」原则）。
5. **NNLM 的光标前文本缓存**（`StreamingNNLMModel::SetTextBeforeCache`）与 decoder 的 KV cache 复用（`CopySavedCacheToTensor`、`PrefillContext`）：
   青简 `neural` 已经共享拼音前缀 KV，可以再加「上文 KV 跨按键缓存」。
6. **E2E 模型异步召回**（`RecallE2EModelResultAsync`）：模型结果晚到就晚插，不阻塞首屏候选，和青简「整句先出、模型后补」的方向一致；
   RefIME 的规模参考：12 层 / 768 维、int4 分组量化、约 74 MB、Metal 推理——青简若上 Mac GPU，candle 的 Metal 后端可以对标这个量级。
7. **词库热更新的「崩溃哈希」**（`maybeCrashDictsHashes`）：新数据加载后崩了就拉黑该版本，这个低成本保护青简发数据包时也可以做。
8. **增量补丁更新数据**（bsdiff）：青简数据包变大以后值得做，`qingjian-update` 已有签名校验，可以加 patch 类型。
9. **会话装饰器链**：繁简、emoji、计算器、敏感词等横切功能各自是一层 wrapper，核心搜索不感知。青简 Core 里如果这类功能继续增加，可以参考这种拆法。

**差距与不建议照搬的**

- 模型数量与规模：RefIME 同时跑 bigram、3 层词级 NNLM、LightGBM、1 亿参数 E2E，还有设备分档（`HighDeviceLtrValid`、`enable_low_performance`）；青简是 bigram + 一个字级小模型，差一个数量级，不必追。
- 数据体量：随包 294 MB、主词库 30 MB + 二级词库 52 MB + 大 KV 40 MB；青简按开源可再分发语料做，不可能也不需要对齐体量。
- 隐私：上屏内容上报（`CloudReportEmitInput`）、上屏 bigram 统计（`CollectAndReportBiGram`）、读取聊天应用的聊天上下文都是青简明确不做的事；青简云联想保持「用户自配端点、默认关、只发光标前文本」的现状。
- 候选窗：RefIME 用 AppKit 原生控件拼（`NSTextField` / `NSTableView`），没有主题层；青简已决定「显示面自绘」，这点不用学。

## 未解决的问题与下一步

12 条借鉴项已逐函数取证，复刻规格在 [ref-ime-impl/](ref-ime-impl/README.md)，下面只剩没覆盖到的。

- **E2E 模型输入格式**：`[T1]`～`[T9]` 的含义、拼音如何进 token 序列（`CombineTextAndRunesWithPinyin`、`kSepToken`），需要反编译 `DecoderOnlyNNModel::PreProcessInput`。
- **Mac 上 E2E 走 CPU 还是 GPU、何时触发**：`SetEnable` 的条件、`DeviceAndSvrStat` 的设备分档阈值未读。
- ~~合并规则细节~~：已读，见 [ref-ime-impl/b5-late-merge.md](ref-ime-impl/b5-late-merge.md)（`LocalFirstTop` 不是函数，是 2 字节的 `LocalFirstTop1Info`）。
- **云请求触发条件**：`NeedDoRequest` 的最短长度 / 去抖策略、请求里带多少上文，需要反编译；lite protobuf 字段名不可见，只能从 setter 调用推。
- **上报的具体字段与开关**：`BuildItemFromEmitInput` 写了哪些字段、是否哈希；`CollectAndReportBiGram::SetParam` 的抽样率来源。
- **静态词库的内部布局**：只确认了容器族与索引类型；若要量化「LOUDS + Huffman 值」的空间效率，可读 `TrieIndexLargeDict` / `CompactKVLargeDict` 的加载代码（只看结构，不读词条）。
- 方法上：本次没有对 88 MB 二进制做完整 Ghidra 分析（耗时长），所有反汇编都是 `objdump` 针对单函数；下一步若要读控制流，用 REA / Ghidra 只导入并分析上面列出的几个函数地址即可。
