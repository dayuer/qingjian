# 输入法词库构建体系对标（2026-10）

给青简 / 素笺词库流水线找参照：业界成熟体系怎么分层、怎么定词频、怎么发现新词、怎么评测，再对照青简现状列落地清单。

调研方式说明：本次 WebFetch 不可用，一手资料靠 `curl` 直接拉官方仓库原文（下面标「已核」）；没拉到原文、只凭公开常识的标「待核」；完全查不到的写「未查到」。数字只写已核的。

## 1. Google Mozc（BSD-3；词典数据另有 IPAdic 等许可）

出处：<https://github.com/google/mozc/tree/master/src/data/dictionary_oss>（README.txt 已核）

做法要点：

- **词典源格式（已核）**：`dictionary00.txt`…`dictionary09.txt`，TSV 五列 `读音 \t 左ID \t 右ID \t cost \t 表记`，例如 `ああると 1851 1851 7129 アアルト`。同一表记可因词性不同出现多行（不同 lid/rid、不同 cost）。
- **词表来源（已核，README 原文要点）**：开源版词表基本等同 IPAdic（mecab-ipadic-2.7.0-20070801）；用冲绳词典（o-dic）补专名；Google 手工补了一些形容词 / 动词；大量片假名词是在 Web 语料处理中「作为未登录词收集」得来；为提高转换质量，用 MeCab+IPAdic 在 Web 上收集了「社員証、再起動」这类复合词。**开源版不含 Google 日语输入的 Web 大词表**——即商业版与开源版词表分层。
- **cost 口径（待核）**：沿用 MeCab/IPAdic 体系，词条 cost 是「生起成本」，约等于 `-log P(词|词性)` 乘一个比例常数后取整；连接成本矩阵 `connection_single_column.txt`（由左右 ID 两两组成）约等于 `-log P(右ID|左ID)` 的缩放。转换是 Viterbi 求「词条 cost + 连接 cost」最小路径。要点：**所有成本是同一把对数尺子**，人工补词也要折算到这把尺子（通常给一个按词性的默认 cost）。
- **segmenter / 品词（待核）**：`id.def` 定义 POS ID，`segmenter.def`/`boundary.def` 定义哪些相邻 ID 之间可以断开、边界惩罚多少；后缀 / 前缀词典单独一份。
- **新词与专名**：Web 语料未登录词收集 + 专名词典（地名、邮编词典可选生成，README 给了 `gen_zip_code_seed.py` 生成命令）；用户词典单独一层，带词性，cost 由引擎按词性给默认值（待核）。
- **对青简的启示**：中文没有日语那样的形态学连接，但「词条 cost 与 LM 同一对数尺子」「开源版与商用 Web 大表分层」「生成命令写进仓库可复现」三点可直接照搬。

## 2. fcitx5 libime / fcitx5-chinese-addons（LGPL-2.1+）

出处：<https://github.com/fcitx/libime>，`data/CMakeLists.txt` 与 `src/libime/pinyin/pinyindictionary-text-format_zh.md`（均已核）

- **数据是版本化的外部产物（已核）**：仓库不放数据源，CMake 从 `download.fcitx-im.org` 下载带日期的 tarball 并校验 SHA256：语言模型 `lm_sc.arpa-20260629.tar.zst`，词典 `dict-20260907.tar.zst`（内含 `dict_sc.txt`、`dict_extb.txt`），码表 `table-20240108.tar.zst`。
- **LM 构建（已核）**：ARPA 文本 → `slm_build_binary -s -a 22 -q 4 trie lm_sc.arpa sc.lm`（trie 结构、量化参数 `-a 22 -q 4`，与 KenLM `build_binary` 参数同形）；再用 `prediction` 工具由 `sc.lm` + arpa 生成 `sc.lm.predict`（联想表）。安装名 `zh_CN.lm` / `zh_CN.lm.predict`。
- **ARPA 如何训练（未查到）**：仓库内没有训练脚本与语料说明；业内普遍推测用 KenLM `lmplz`，但官方没写，不引用。
- **词典格式（已核）**：每行 `<汉字> <全拼用'分隔> [weight]`，例 `倪辉 ni'hui 0.0`；weight 是可选浮点数，缺省时保存回文本会显式写出；坏行跳过不中断。`pinyindict` 把文本编成二进制 `sc.dict`；生僻字扩展 B 区单独一本 `extb.dict`。
- **分层**：系统词典（sc）+ 扩展 B（extb）+ 用户词典 + 用户历史 `libime_history`（用户输入句子单独存、单独打分，不改系统 LM）。词条选择以 LM 打分为主，词典 weight 只是附加项——**排序主力在 LM，不在词典人工频**。
- **对青简的启示**：(1) 产物带日期 + SHA256，与青简 `SHA256SUMS` 门禁一致，可再把「日期版本号」写进文件名；(2) LM 二进制化与量化参数写死在构建脚本；(3) 用户历史与基础 LM 物理隔离。

## 3. librime / rime-ice 雾凇拼音（GPL-3.0；只学方法，不用数据）

出处：<https://github.com/iDvel/rime-ice> README 与 `others/docs/Credits.md`（已核）；作者博客 <https://dvel.me/posts/rime-ice/>（未拉取）

- **分层（已核）**：
  - `8105`：常用字表，《通用规范汉字表》+基本扩充；另有 Unihan 4 万大字库。
  - `base`：基础词库，含两字词及调频。
  - `ext`：扩展小词库，**含多音字注音**。
  - `tencent`：扩展大词库，**无注音（由 Rime 自动注音）**，只收非多音字、只发一种音的多音字、同义多音字——即把「注音会出错」的词排除在自动注音层之外。
  - `en_ext`：英文扩展，多为缩写与互联网词；另有 `cn_en` 中英混合层（README 段落未完整拉到，具体规模未查到）。
- **数据来源（已核，Credits）**：通用规范汉字表、Unihan（Unicode License V3）、现代汉语常用词表、华宇野风词库（Public Domain）、rime-essay-simp 八股文（LGPL-3.0）、THUOCL（MIT）、腾讯词向量词表（标注 CC BY 3.0）。
- **人工流程（已核）**：维护内容是「异形词、错别字的校对，错误注音的修正，缺失常用词的增添，词频的调整」；通过共建 issue（#666）收集；参照「校对标准论坛」。
- **词频口径（部分待核）**：base 的词频继承自八股文 / 野风等来源再人工调频；作为 Rime 词典，权重是整数，Rime 内部取对数后与 essay 语法模型叠加。具体折算公式未查到。
- **更新节奏（未查到精确值）**：仓库按 Changelog 持续滚动发布，无固定周期说明。
- **对青简的启示**：(1) **按「注音可信度」分层**是最值得学的一条：自动注音的大表只收单音或同义多音词，多音词必须进手工注音层；(2) 字表层与词表层分离；(3) 校对有统一标准（异形词、错别字）和公开 issue 通道。

## 4. 搜狗 / 讯飞 / Gboard 等公开做法

- **新词发现：凝固度（互信息）+ 自由度（左右熵）+ 频次**。最常引用的是顾森（Matrix67）2012 年《互联网时代的社会语言学：基于 SNS 的文本数据挖掘》（matrix67.com 博客，待核原文数字）：候选 n-gram 需同时满足出现频次、内部凝固度 `P(w)/max(P(a)P(b))` 与左右邻字信息熵的最小值都超过阈值。原文阈值与语料规模强相关，本报告不转述具体数字。
- **搜狗**：公开资料多为新闻稿与专利层面（「细胞词库」按领域分包、用户下载启用；云输入在本地候选之外补一条云候选）。具体算法论文未查到。**分层结论**：基础库 + 领域细胞词库（默认不装）+ 云候选 + 个人词库，与青简「领域包默认关」同构。
- **讯飞**：公开技术分享未查到可引用的词库构建细节。
- **Gboard**：公开论文集中在联邦学习：Hard et al. 2018《Federated Learning for Mobile Keyboard Prediction》（arXiv:1811.03604），以及 Chen et al. 2019《Federated Learning of Out-of-Vocabulary Words》（arXiv:1903.10635，用联邦方式发现 OOV 新词，不上传原文）。要点：**个性化与新词在端侧学，基础模型只接收聚合、差分隐私后的更新**；基础库与个人数据隔离。中文 Gboard 词库构建细节未查到。
- **热词时效**：各家公开材料只说「云词库每日 / 定期更新热词」，衰减公式未查到。通行做法是给词条记「首见年份 / 最近活跃时间」，在本地库中按时间降权或下沉到可选包。

## 5. 开源数据源与许可（与 GPL-3.0-or-later 的兼容性）

| 数据源 | 许可 | GPL 兼容 / 可随包分发 | 备注 |
|---|---|---|---|
| THUOCL | MIT（已核，README/Credits） | 兼容，可分发，保留版权声明 | 词频是 **DF（含该词的文档数）**，语料分领域：CSDN、新浪新闻、搜狗语料等（已核）。DF 不是词次，不能与语料词次直接混用 |
| 腾讯 AI Lab 词向量词表 | rime-ice 标注 CC BY 3.0（已核其标注；腾讯官方页面未拉取，待核） | CC BY 3.0 与 GPL-3 单向兼容一般被认为可行（署名即可），只用词表不用向量 | 词表无词频、无注音，只能当候选来源，频次需另算 |
| CC-CEDICT | CC BY-SA 4.0（待核，官方 MDBG 页面） | CC BY-SA 4.0 → GPL-3 有 CC 官方声明的单向兼容 | 有拼音注音，可作多音字校验源 |
| 中文维基百科 | CC BY-SA 4.0（新文本）/ GFDL（待核） | 可用其统计量；分发原文需 BY-SA | 从它导出的 n-gram 计数一般视为事实统计，风险低 |
| OpenCC | Apache-2.0（待核） | 与 GPL-3 兼容 | 繁简、异体词映射 |
| Unihan | Unicode License V3（已核，rime-ice Credits 所引） | 宽松许可，兼容 | 字级读音 kMandarin / kHanyuPinyin，用于单字注音与多音字表 |
| LCCC（清华 CDial-GPT） | 未查到明确许可（README 未写，仓库 LICENSE 待核） | **待核**；在核实前只用统计量，不随包分发原文 | 口语对话语料，适合补口语 bigram |
| 现代汉语常用词表 / 通用规范汉字表 | 国家标准 / 规范文件，版权状态未查到明确说法 | 只用「是否收录」与读音事实，不复制排版 | rime-ice 也在用 |
| 华宇野风词库 | Public Domain（rime-ice 标注，已核其标注） | 兼容 | 来源较老，需人工审 |
| rime-essay-simp | LGPL-3.0（已核） | 兼容 GPL-3，可用 | 是词频 + 语法表 |
| 雾凇拼音数据 | GPL-3.0 | 许可上兼容，**但青简已决定彻底不引入**（CLAUDE.md），只学方法 | — |
| mozc dictionary_oss | IPAdic 许可等（日语） | 与中文无关，只学格式 | — |

## 6. 最佳实践综合

1. **分层**：字表（通用规范 8105 + Unihan 扩展，带读音）→ 基础词库（高频、手工注音或高可信自动注音）→ 扩展大表（只收单音 / 同义多音，可自动注音）→ 领域包（默认关）→ 网络 / 时效包（带年份）→ 云 / 联想（不落盘进基础库）→ 用户层（词频、用户词、个人 n-gram，物理隔离，可清空）。
2. **一把尺子**：词条分数统一成 `cost = -ln P(w)`（或 log10，按引擎），P 来自同一套语料的平滑计数；不同来源的「频次」必须先换算：THUOCL DF 需按「DF→词次」回归或只当收词依据不当频次；人工加词不给自定义数字，而是给「参照词 + 相对档位」（例如等于同音组第 k 名的 cost + δ），由脚本算出 cost。Mozc 与 libime 都是 LM / 对数成本主导，词典权重是附加项。
3. **新词发现**：候选 n-gram（2–6 字）→ 频次门槛 → 凝固度门槛 → 左右熵门槛 → 去停用字开头结尾 → 词表已有过滤 → 人工过目。阈值随语料规模定，要在开发集上扫（青简 cli 有常数扫描能力可复用）。
4. **注音与多音字校验**：多音字表来自 Unihan + CC-CEDICT；自动注音只允许无歧义词；多音词必须人工注音或由 CC-CEDICT 交叉验证，两源不一致进人工队列。
5. **质量门槛**：长度（中文 2–7 字为主，长于此进短语层）、频次下限、熵 / 凝固度下限、字符集（只收 8105 + 白名单字）、黑名单（学名、地名长尾、敏感词）、同音竞争检查（新词进入后同音组前 3 名是否被异常顶替）。
6. **评测与回归**：整句评测集（固定文本，记录首选准确率、前 3 命中率）、输入日志回放（逐键首选变化）、同音组排序快照（diff 可审）、性能（加载时间、内存）。每次词库改动出一份评测 diff 才合。
7. **发布与版本化**：数据产物带日期版本号 + SHA256（libime 做法）；生成命令与数据源版本写进仓库；用户层格式与基础库版本解耦，升级不清用户数据。

## 7. 对照青简现状的差距与落地清单（按优先级）

现状：基础 dict 约 9.1 万词；10 本 THUOCL 领域包默认关、已做长度 / 学名 / 地名清洗；网络用语 178 条，词频按 dict.tsv 同音竞争词定；bigram LM（中文维基 + LCCC），短语 / 专名走合成计数；本机无语料、无 LLM 密钥；已知问题是词频尺子不统一、同音竞争阈值曾定错。

**P0-1 统一词频尺子**
- 输入：现有 dict.tsv 各词条的来源标记（语料次数 / jieba 折算 / THUOCL DF / 人工值）。若无来源列，先补。
- 做法：定义唯一口径 `cost = -ln((c+α)/(N+αV))`，c 只取 LM 同一语料（维基 + LCCC）的词次；没有语料计数的词按「参照词相对档位」由脚本算，不写裸数字；THUOCL DF 只作收词依据。
- 产出：dict.tsv 增加 `source` 列；一个换算脚本（放 `tools/`）；`crate-notes.md` 写明口径。
- 验收：dict.tsv 里没有无来源的数值；整句评测与回放首选准确率不下降（或下降项逐条可解释）。

**P0-2 同音竞争回归测试**
- 输入：所有同音组（按无调拼音分组）。
- 做法：导出每组前 5 名快照入库；新增 / 调频后跑 diff；对「新词把高频常用词挤下首位」报警（阈值：被挤词语料频次 > 新词 N 倍，N 在开发集上定）。修正「白给压败给」这类反例并加进固定用例。
- 产出：快照文件 + cli 子命令或测试。
- 验收：已知反例（白给 / 败给）进入用例且通过；CI 能跑。

**P0-3 多音字注音校验**
- 输入：Unihan kMandarin/kHanyuPinyin、CC-CEDICT。
- 做法：仿 rime-ice 分层——词条含多音字且读音非唯一时，必须有人工注音或 CC-CEDICT 一致；不一致的进人工队列文件。
- 产出：校验脚本 + 待审清单。
- 验收：基础 dict 中多音字词条 100% 有来源可追（人工 / CEDICT 一致）。

**P1-4 评测集固化**
- 输入：现有整句评测文本、回放日志。
- 做法：固定一份评测集（分新闻 / 口语 / 网络用语三份），每次词库改动输出指标 diff。
- 产出：`apps/cli --eval-text` 的基线结果文件。
- 验收：词库 PR 附评测 diff。

**P1-5 数据源许可清单**
- 输入：第 5 节表格。
- 做法：核实 LCCC、腾讯词表、CC-CEDICT、维基的官方许可原文，写进 `assets/*/README`。LCCC 许可未核实前只用统计量，不随包分发原文。
- 验收：每个随包数据有许可与出处行。

**P1-6 新词发现流水线（等有语料再做）**
- 输入：本地语料（维基 dump / LCCC 已有部分即可开始）。
- 做法：n-gram 统计 → 频次 / 凝固度 / 左右熵三门槛 → 去已有词 → 人工过目稿（沿用网络用语那套过目流程）。
- 产出：候选表 + 阈值记录。
- 验收：在开发集上抽查 200 条，可收率记录在 notes。

**P2-7 时效层**
- 网络用语包已有年份列：定义按年份降权或「超过 N 年未见移出默认包」规则；验收为规则写进 crate-notes 并有测试。

**P2-8 产物版本化**
- 数据产物文件名带日期版本，与 SHA256SUMS 一起发布（libime 做法）；验收：发版脚本校验通过。

## 出处

- Mozc dictionary_oss README：<https://raw.githubusercontent.com/google/mozc/master/src/data/dictionary_oss/README.txt>
- libime 数据构建：<https://github.com/fcitx/libime/blob/master/data/CMakeLists.txt>
- libime 词典格式：<https://github.com/fcitx/libime/blob/master/src/libime/pinyin/pinyindictionary-text-format_zh.md>
- rime-ice README / Credits：<https://github.com/iDvel/rime-ice>
- THUOCL：<https://github.com/thunlp/THUOCL>
- LCCC / CDial-GPT：<https://github.com/thu-coai/CDial-GPT>
- Gboard 联邦学习：arXiv:1811.03604，arXiv:1903.10635
- Matrix67 新词发现：<http://www.matrix67.com/blog/archives/5044>（待核）
