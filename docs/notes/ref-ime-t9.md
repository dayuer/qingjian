# 参考输入法 RefIME 2.2.3 的九键（T9）拼音引擎：静态逆向笔记

分析对象与方法沿用 [ref-ime-reverse.md](ref-ime-reverse.md)：本机 `/Library/Input Methods/RefIME.app` 的 arm64 片，只做静态分析，没有运行、调试、修改 RefIME，没有网络请求。
反编译用 REA（`rea-agents@6.3.0 decompile --provider ghidra`）。REA 每次调用都会对目标做 Ghidra 全量自动分析，88 MB 的整片吃不消，
所以先用脚本把选中的函数按原地址切成小 Mach-O（只带这些函数的页、`__TEXT,__cstring`、`__TEXT,__const` 与符号表），再交给 REA。
调用关系用已有的整片反汇编查（只做 xref，不读大段汇编）。中间产物在 `/tmp/ref-ime-rea/t9/`：`carve.py`（切片）、`dec.sh`（批量反编译）、`d1/`～`d5/`（伪代码）。
读过的数据只有键盘布局一类的小表（`__DATA_CONST` 里的九键字母分组、`__TEXT,__const` 里的九键邻键位图），以及 `decgnr.bin` JSON 头里 `[T1]`～`[T9]` 的 token id；
没有读取词条、模型权重或数据库内容。

置信度：**[确]** 伪代码或数据里直接看得到；**[推]** 由多条证据推出；**[猜]** 只有名字线索。地址都是 arm64 片里的虚拟地址。

## 概览

RefIME 的九键**没有单独的引擎**，和全键盘共用同一条路：音节图（`graphs_v2::SyllableGraph` / `graphs_v3::SyllableGraphV3`）、
LOUDS trie 上的增强器搜索、整句 beam search、LTR 排序。区别全在**输入表示**上：

- 壳把每次九键按键送成**这个键上的字母串**（`"abc"`、`"def"`……，键 1 是分隔符 `","`），全键盘则是单个字母。
  引擎靠「输入元素的长度是不是 1」区分九键和全键盘，比如邻键纠错就按这一条选九键邻键表还是全键盘邻键表。[确]
- 在 trie 上搜索时，一个九键输入元素匹配它字母串里的任何一个字母，所以「数字串 → 音节」不建单独的图，**字母在搜索时现场展开**，
  切分歧义由音节图的所有 (起, 止) 边一起表达，交给整句与排序去选。[确：输入表示与邻键分支；推：精确匹配的展开逻辑，被内联进模板没有单独看到]
- 只有两处真用数字串：云端请求侧的 `T9PinyinIndex`（把全部合法拼音按长度建「数字串 → 拼音列表」哈希表，枚举所有切分）
  和端到端模型的输入（数字直接当 token `[T2]`～`[T9]`）。[确]
- 九键特有的东西：九键邻键表、九键编辑词典 `EditWordDictT9`、左侧拼音栏（`SyllableHint`）、`t9_rpl_*` 排序特征与单独一套 T9 排序参数、
  用户词的九键简拼搜索 `SearchT9JPSyllables`、单字母元音音节的特判 `IsT9AOEorIUV`。

Mac 版不提供九键键盘，这些代码是和手机端共用的 Core；Mac 的 `imeData.bundle` 里也没有九键专用数据文件（只有全键盘的 `editpinyin.bin`）。[推]

## 1. 输入表示：数字键怎样变成候选音节

### 1.1 键的表示 [确]

- `wxime::utils::T9Keyboard()::kT9Keyboard`（`GetInputForT9KeyBoard(char)` 0x100f2fc68 里懒初始化，源表在 0x1031f5bf8）是 10 个 `string_view`：
  `0:"" 1:"@#" 2:"abc" 3:"def" 4:"ghi" 5:"jkl" 6:"mno" 7:"pqrs" 8:"tuv" 9:"wxyz"`。`GetInputForT9KeyBoard('2')` 返回 `"abc"`，非 1～9 的字符原样返回。
- `T9KeyToNum()`（0x100f2f608，源表 0x1031f5b20）反过来：`{",":1, "abc":2, …, "wxyz":9}`。**分隔符键 1 在引擎里就是 `","`。**
- `StandardT9KeyboardToNumber(char)`（0x100f3cfa4）：数字原样；`'a'..'z'` → `(c-'a')/3+2`。它只对每组的首字母（a d g j m p t w）正确，
  说明引擎里约定用键串的首字母代表这个键；`AnyT9KeyboardToNumber`（0x100f3d018）才是任意字母的真实键位（s→7、t～v→8、w～z→9）。
- `ConvertT26ToT9Key(string_view)`（0x100f2fb60）把字母串逐字转成数字串（`kCharToT9Num`）；`T9PinyinToNumberKey(const string&)`（0x100f3d0e4）把一个键串（`"abc"`）转成数字字符，查不到返回 `'0'`。
- `_wxime_keyboard_type`：Swift 侧枚举名 `KeyboardTypeFullPinyin` / `KeyboardTypeNinePinyin` / `KeyboardTypeFullDoublePinyin` / `KeyboardTypeNineDoublePinyin` /
  `KeyboardTypeNineStroke` / `KeyboardTypeFullWubi` / `KeyboardTypeHandwrite` / `KeyboardTypeFindWords` / `KeyboardTypeFullEnglish`。
  `TryToConvertInput`（0x100f5de9c）里 0 = 全拼、1 = 九键拼音。**九键双拼也存在**（`T9 double mode with E2E, this should not happen`）。

### 1.2 音节图 [确 + 推]

- `SyllableGraph::ProcessKey(string_view key, unsigned, string_view)`（0x1009031a0）：输入序列最多 60 个元素（`0x3b < size` 直接返回），否则 `Insert`。
  `ProcessKeyAtEnd`（0x100903c08）为新列建所有 (i, end) 边：音节边（`InitializeSyllableEdgeWithCatchContext`，缓存键是把 i..end 的输入元素用分隔符 join 起来的串）、
  英文边、整音节补全边，以及 `Edge::AddExtraSyllable(..., EditWordDictT9*, bool)`（调用点约 0x100904xxx，见伪代码第 775 行）。[确]
- 音节边里的音节来自 `SearchImpl<LoudsTrie::Cursor, AugmentRecord, int, IterativelyInputAugmenters<…28 种…>>`，输入是 `vector<string>`——每个元素就是一个键串。
  九键的 `"abc"` 与全键盘的 `"a"` 走同一个搜索。[确：类型签名]
- 增强器在每个节点检查输入元素：长度为 1 且是 `','` 或 `';'` 时多数纠错增强器直接跳过（`callImpl<3>` 0x1009b7700 里反复出现的 `*p == 0x2c / 0x3b` 判断）。即**分隔符段不做纠错**。[确]
- 所以 94664 的歧义（`zhong` / `xing'o` / `xin'go`……）不是在解析阶段消掉的：每段 (i, j) 都可能有边，整句 beam search 与 LTR 选。[推]

### 1.3 九键编辑词典 `EditWordDictT9` [确：代码；推：用途]

`SyllableEdge::AddExtraSyllable`（0x1009a7ce0）：只在这条边的输入元素全是多字母键串时生效——把键串转成数字串，
`EditWordDictT9::LookUp(digits)`（协程，0x1008b2bf0 / .resume 0x1008b474c）返回若干「拼音串 + 被改动的位置」，拼音串里 `,` 处记一条类型 2（InsertSep）的 `AugmentRecord`，
被改动的位置记一条类型 5（与邻键纠错同一类型）的记录，然后把这个音节作为额外音节加到边上。也就是**按整段数字串查一张「常见按错 → 正确拼音」的表**，
补上逐键增强器找不到的纠错。Mac 包里没有对应数据文件，数据格式没看。

### 1.4 选拼音（左侧拼音栏点选）怎么进图 [确]

`PinyinV2Session::setSyllableHintImpl(pinyin, hintId, aux)`（0x10091c79c）：

1. `SyllableHintID::ParseFromString`（0x10099e4a0）要求 id 正好 8 字节，里面打包了起点、长度等字段。
2. 取输入序列里 [起点, 起点+长度) 这段键，连同当时的拼音栏列表，存进 `SelectedHistory::EmplaceSyllableSelection`（0x10091dc50），便于退格时还原。
3. 选中的拼音后面补一个分隔符：前一个键是 `'` 就补 `'`，否则补 `,`（`0x27` / `0x2c`）。
4. v2 图：`DropLast` 掉被选中段及其后的键 → `SyllableGraph::ProcessSelectedSyllableHint(pinyin+sep)`（0x10091e038）把拼音**逐字母**当输入元素压回序列
   （每个字母一列，`'` 统一记成 `,`）→ 把选中段之后原来的键按原样 `ProcessKey` 重放。v3 图走 `SyllableGraphV3::ProcessSyllableHintOperation` 一次做完。
5. 记一条 `actions::SelectSyllableHint` 用户行为。

即：**锁定的音节在图里变成全键盘式的单字母输入**，后面的九键键串照常展开；锁定之后的分隔符让切分不会跨过它。

## 2. 候选生成与排序

### 2.1 词、整句、简拼 [确 + 推]

- 词搜索与整句和全键盘同一套（`WordSearchBySyllableGraph`、`SentenceSearcherV3`），输入差异已经在音节图里消化掉。[推]
- `CandidateV2Collector::IsT9AOEorIUV(Syllable, n)`（0x100a4c71c）：只在 n == 1、收集器处于九键状态（`this[0x88]`）时判断；
  音节没有任何纠错 / 模糊计数、拼音只有一个字母，且是 `a e i o u v` 之一（位图 `0x304111`）才返回真。
  调用方是 `AddImpl<ISentenceSearcher::Result>` 与 `AddImpl<WordSearchResult>`。用途是特判「单个元音键当成零声母音节」这类九键首键候选。[确：判断；推：用途是降权或过滤]
- 用户词的九键简拼：见第 6 节。

### 2.2 `t9_rpl_*` 特征 [确：写入逻辑；推：含义]

`CandidateV2::SetSentenceSearcherT9ReplacementFeature`（0x100a4a970，由 `SetSentenceSearcherGlobalFeature` 调用）往特征表写：

| 特征 | 取值 |
|---|---|
| `t9_rpl_feature` | `MixRankInfo+0x208`（记为 r），路径类型不在 {4..7, 13} 时 |
| `accumulated_t9_rpl_feature` | 路径类型在 {4..7, 13} 时取 r(r+1)/2 − 1 |
| `t9_rpl_with_typo_feature` | `+0x200` − r（大于 0 才写） |
| `t9_rpl_with_comp_feature` | `+0x1fc`（补全计数，非 0 才写） |
| `context_t9_rpl_feature` | 一个每次调用加一的计数 |

r 只在 r > 0 时才写。结合 1.3 节类型 5 记录（键被换成别的字母）与 `RecordsToCorrectedQuery`（0x100b13520，九键分支把查询表示成数字串再按记录类型改写），
我们读作：**rpl = 路径上「把某个键换成另一个键」的纠错次数**，`with_typo` 是其他纠错，`with_comp` 是补全。[推]
另外 LTR 有两套参数：`LtrModelHyperParams::Impl::Impl`（0x100ea4d90）同时装 `model_params_sv_t26_` 与 `model_params_sv_t9_`（日志 `Model too large, t26 size: … , t9 size:`），
`LRModel` / `OneLayerNNModel` 读 `t9_coef`。**九键用单独一套排序参数。**[确]
`utils::T9LMParams` / `T9BackoffParams` 两个全局量说明语言模型也有九键专用的插值 / 回退参数。[猜：没找到读取点]

### 2.3 数字候选 `T9NumberRelatedCandidateID` [确：存在；未解决：生成点]

- 它是候选 ID 变体的第 9 个（下标 8，前面依次是 TrieMatching、Pinyin、MostLikelySequel、CloudPinyin、CloudMostLikely、CloudEnglish、PunctRelated、AtSymbol），
  JSON 类型名 `t9_number_related`，也列在 `PinyinIDLike::Wrap` 里（当作拼音类候选处理）。
- 壳侧有上报事件 `WtAppActionOpenTnineNumber` / `WtAppActionCloseTnineNumber`，即有一个「九键出数字」的用户开关。[推]
- 没找到构造它的函数（试过 `NumberSession`、`PunctRelatedSession::SearchImpl`——后者构造的是下标 6 的 PunctRelated；以及在 84 个 `CandidateV2` 构造调用点附近搜下标 8 的立即数）。
  Mac 上可能根本走不到。电话、日期、计算器类候选分别是 `UserHotWordHelper::PhoneNumberPrefix`、`Calendar`、`NumberCalculator`，不是九键专用。

## 3. 九键纠错

- **邻键（换键）[确]**：`KeyboardNeighbourAugmenterIteratively`（内联在 `callImpl<3>` 0x1009b7700）。输入元素长度为 1 → `VisitT26KeyBoardNeighbour`（再按配置选 Parallel / Vertical 变体）；
  否则 → `VisitT9KeyBoardNeighbour`（0x100f2f104）。后者用 `T9Neighbours()::kT9Neighbours`（0x102eb61d0）的位图，按键号取邻键，再把邻键的每个字母回调出去：

  | 键 | 邻键 | 键 | 邻键 |
  |---|---|---|---|
  | 2 | 3 5 | 6 | 3 5 9 |
  | 3 | 2 6 | 7 | 4 8 |
  | 4 | 5 7 | 8 | 5 7 9 |
  | 5 | 2 4 6 8 | 9 | 6 8 |

  只有上下左右，没有斜对角，**键 1（分隔符）不参与**。回调（0x1009bc080）在 trie 上找该字母的子节点，记一条类型 5 的记录（原键首字母 + 替换字母）。
- **漏按 / 多按 / 换位**：`InsertAtMiddle`、`Skip`、`Exchange` 是同一模板里的通用增强器，没看到九键分支，应当对九键同样生效（漏按时在 trie 上补任意一个字母）。[推]
- **整段查表**：`EditWordDictT9`（1.3 节）。[确]
- **模糊音**：`Retroflex`、`XnToXngNasal` 等作用在 trie 游标（拼音一侧），与输入元素是不是键串无关，九键下照常生效；
  分隔符段不纠错的规则同样适用。没看到九键专属的模糊音逻辑。[推]
- 与全键盘的区别：全键盘邻键是 QWERTY 几何（平行 / 垂直两种）、还有触摸坐标个性化（`MemCacheUserCoordinateDict`）；九键是 3×3 网格四邻域，外加整段数字串的编辑词典。

## 4. 左侧拼音栏

- 数据通路 [确]：`PinyinV2Session::updateSyllableHintList()`（0x100916178，由 `SearchCandidates` 调用）取 `ISyllableHintUpdater` 的当前列表，
  经 `lsignal<SyllableHintUpdateEvent>` 发出；`CCallbackHolder::EmitWithLog`（日志 `sigSyllableHintUpdated, session: %lu, size: %lu`）转成 C 回调给壳。
  中间有 `SyllableHintVersioningWrapper`、`DirtyCheckingSession::SetSyllableHint`（`dirty SetSyllableHint`）这些会话层。壳选中后调 `ISession::SetSyllableHint(pinyin, id, aux)`，
  撤销是 `DropLastSyllableHint`。每条 `SyllableHint` = {8 字节 id（起点、长度…）, 拼音串}。
- 列表怎么算 [确]：`SyllableHintUpdater::UpdateAndGetCurrentSyllableHints`（0x100add410）：
  - 起点 = 最近一次拼音选择的终点（没有选择就是 0）；
  - 若当前状态正好回到某次选择的位置，用 `UpdateSyllableHintFromHistory`（0x100add508）还原当时保存的列表；
  - 否则 `UpdateSyllableHintFromGraph`（0x100add67c）→ `SyllableGraph::VisitFirstRunePossibleSyllables(start, …)`：
    列出**从起点出发的第一段所有可能音节**（底层是 `ViewFirstRunePossibleSyllables` 0x1008e03d8：生成器 → 过滤 → 按拼音串去重 → 排序），每个音节记一条 hint。
  - 兜底：图里一条都没有、起点为 0、输入只有 2～4 个键时，把各键字母做笛卡尔积，凡 `data::IsValidUncommonPinyin` 为真的组合加进列表——
    让冷僻音节也能被选。
- 选了之后候选怎么收窄：见 1.4 节——锁定段在图里变成确定字母，所有候选的第一段只能是它；拼音栏随之从锁定终点继续往后列。

## 5. 端到端模型的 `[T1]`～`[T9]`

- `decgnr.bin` 的分词器配置里 `[T1]`～`[T9]` 是特殊 token，id 分别为 1～9（JSON `added_tokens_decoder`）。[确]
- `TryToConvertInput(Span<string>, keyboard_type)`（0x100f5de9c）：全拼要求每个元素是单个小写字母，否则日志 `E2E input T26 pinyin not supported.` 并放弃；
  九键把每个键串 `T9PinyinToNumberKey` 成数字字符，出现 `'0'` 就放弃；其他键盘类型 `E2E not supported keyboard`。[确]
- `DecoderOnlyNNModel::PreProcessInput`（0x100ef15b8）：模型版本带 `support_t26` / `support_t9` 两个标志（不匹配时日志 `Decoder-only model version do not keyboard type … support_t26 … support_t9`）；
  九键且启用时，对 pinyin 段每个元素：空格 → 结束；`'2'`～`'9'` → token id = 数字值（正好落到 `[T2]`～`[T9]`）；其他 → `Decoder-only dec model found unk t9 pinyin input:`。
  全拼则把字母经词表映射。前文（光标前文本）走 KV cache 复用（`FindCommonPrefixPos` + `CopySavedCacheToTensor`）。[确]
- 结论：**九键序列直接作为模型输入，模型自己完成「键 → 拼音 → 字」**，不经过音节图。`RecallE2EModelResultAsync` 构造时遇到九键双拼会报 `T9 double mode with E2E, this should not happen`。[确]

## 6. 九键下的用户学习

- **用户词九键简拼** [确]：`UserDictSyllableGraphV2::Impl::SearchT9JPSyllables()`（0x100cbac88，由 `InitJPSyllables` 调用）在用户词的简拼游标
  `UserDictV2::GetSyllableIdxMergeJPCursor` 上做广度搜索：每个九键键串里的**每个字母都当作一个声母首字母**去推进游标（`T9SearchNode` 前沿），
  于是只按每字一键也能召回用户词（日志 `find user word from … jp_syll_count`）。排序有 `user_t9_jp_feature` / `user_t9_jp_bias` 和超参 `userDict_t9_jp_weight` / `userDict_t9_jp_bias`，
  `PotentialUserDictSearchParams` 里有开关 `disable_t9_full_jp_`（只能 0 / 1）。
- **按键盘类型分键存储** [确：签名；推：键的拼法]：`UserSingleCnSelectDict::LookUp`、`UserEmojiSelectDict::Add/Get`、`UserHardFixDict::GetFixedInput` 都带 `_wxime_keyboard_type`，
  共用 `dict::getRealKey(keyboard_type, input, doublepin)`（0x1008a75cc）拼存储键。读作：**同一个输入在九键和全键盘下的选择记录分开存**，九键记的是键串序列。
- **置顶词** [确：调用；推：用途]：`UserWordHardFixWrapper::SetOption` / `GetFixedKeyAndPendingInput` 调 `ConvertT26CharToT9Char` 与 `GetInputForT9KeyBoard`，
  日志 `[PIN_TOP] T9 pending, syllable: [`：置顶词按拼音存，九键下现场换成键串去匹配。
- 云端：`CloudSearchByInputV2` 用 `IsValidNormalT9Pinyin`（0x1007b4c5c，把每段键串转数字、拼起来查 `valid_t9pinyin_set`）决定要不要发请求，
  `EnumT9PinyinSplits`（0x100dfcea4）/ `T9PinyinIndex::Lookup`（0x100dfedc4，6 张按长度分的「数字串 → 拼音」表，键长 1～6）把数字串枚举成拼音切分，
  再用这些切分挑用户词附在请求里（`AppendUserWordCandidates`）。[确]

## 对素笺的启发

1. 九键不必另起一套解析器：把输入元素从「单字母」推广成「字母集合」，现有的音节 trie / 纠错 / 整句全部复用；全键盘只是集合大小为 1 的特例。
2. 分隔符（键 1）与锁定的拼音都在输入序列里表示成确定字母，图上就自然收窄，不需要额外的约束机制。
3. 拼音栏 = 「从当前锁定终点出发的第一段所有音节」，加一个短输入的冷僻音节兜底。
4. 九键排序要单独调参（RefIME 连 LTR 参数都分了两套）；`t9_rpl` 这类「换键纠错计数」应进排序特征而不是硬过滤。
5. 用户词的九键简拼召回（每字一键）是九键用户最常用的快捷方式之一，代价低。

## 未解决的问题

- `T9NumberRelatedCandidateID` 的生成点没找到；Mac 版可能不生成。
- 精确匹配增强器对多字母键串的处理被内联进 `SearchImpl` 模板，没单独看到「子节点字母 ∈ 键串」那段；结论是由类型签名、邻键分支与 `StandardT9KeyboardToNumber` 的约定推出的。
- `MixRankInfo+0x200 / +0x208 / +0x1fc` 由谁累加没追；`t9_rpl` 的确切含义仍是推断。
- `T9LMParams` / `T9BackoffParams` 的读取点、`EditWordDictT9` 的数据格式与来源文件（Mac 包里没有）。
- `IsT9AOEorIUV` 为真时调用方具体做了什么（降权、过滤还是改切分）。
- 九键双拼（`KeyboardTypeNineDoublePinyin`）的键位与解析没看。
