# 词库分层：洗领域包、填网络用语（规格，待审）

**上位**：[design/lexicon-architecture.md](../design/lexicon-architecture.md)（2026-10 词库体系规划：sources.toml / cost / 分包 / 门槛 / 阶段划分）。
本页是其中「第 1、2 步」的落地规格：这里定的规则若与上位冲突，以上位为准并改这一页。

**状态**：2026-10-07 写就，同日审过。**第 1 步已实现**（`tools/dict-convert`，报告见 `data/generated/domain-report.tsv`）；
第 2 步的材料与读取已就绪（候选表、三类判定、三个 TSV、`lexicon` 的读取与两本包都做了，见下），
词频、年份与标音要等 `data/corpus/` 与 LLM 密钥。审定的两处：`--places-min-df` 缺省 500（被筛掉的地名进 places-extended，不丢）；`law` 被长度门槛删掉一半以上，接受
（机构与法条全名不是一口气打出来的，这本包又缺省关）。逐步验收：第 1 步、第 2 步各量一次，不许用第 2 步挣回来的分掩盖第 1 步掉的分（实测第 1 步没掉分，见下）。
**范围**：只做第 1 步（洗现有 11 本领域包，**只删不改引擎**）与第 2 步（把 `04_internet_slang` 填起来）。引擎、排序、语言模型的代码这两步都不动。

## 为什么做

随包领域词库 11 本 13 万条，用户在「词库」页勾一本（或默认开着的成语）之后，候选里就会冒出
「江西武夷山国家级自然保护区管理局」「疾病预防控制中心」这类词：语料里出现得多（THUOCL 的 DF 高），
但当字打的人不会整块敲。反过来，天天在打的新词（内卷、破防、yyds）词库里一条都没有 ——
`assets/lexicon/04_internet_slang` 至今是个空目录。

## 一、现状（2026-10-07 在本机量的）

`assets/lexicon/03_domains/*.tsv`，词形去重后按来源文件分 11 本；后面是「超过 6 个字」的条数与占比：

| 包 | 条数 | >6 字 | >6 字占比 | 长词是什么 |
|---|---:|---:|---:|---|
| animals | 17,287 | 2,778 | 16% | 山东省海洋与渔业厅、防止虐待动物协会、纯种德国牧羊犬 |
| medicine | 18,749 | 1,848 | 10% | 疾病预防控制中心、急性淋巴细胞白血病、新型农村合作医疗 |
| places | 44,803 | 1,415 | 3% | 新疆维吾尔自治区、江西武夷山国家级自然保护区管理局、市中级人民法院 |
| poetry_lines | 13,703 | 6,229 | 45% | 咬定青山不放松、一年之计在于春（整句诗，不是词） |
| idioms | 8,519 | 299 | 4% | 成语本身多是 4 字，长的仍是成语 |
| food | 8,974 | 296 | 3% | —— |
| law | 9,896 | 5,285 | 53% | 中华人民共和国公司法、上海证券交易所、劳动和社会保障部 |
| it_computing | 15,998 | 1,610 | 10% | assets目录、Android布局 |
| finance | 3,830 | 87 | 2% | 国家外汇管理局、中南财经政法大学 |
| automotive | 1,752 | 242 | 14% | 汽车销售服务有限公司、中国汽车技术研究中心 |
| historical_figures | 13,656 | 11 | 0% | —— |

另外两处结构性问题：

- **寄主 + 学名的拼接**（动物、医学）：`犬复孔绦虫`、`牛带绦虫`、`猫抓病`、`猪链球菌病`、`人乳头瘤病毒` 这类，
  以单字宿主开头、以虫 / 菌 / 病 结尾的共 73 条；专业，没人整块打。
- **地名里的巷弄与社区**：`places` 里以 村 / 路 / 街 / 巷 / 街道 / 社区 / 小区 / 乡 / 镇 结尾的有一万多条
  （`曹家巷`、`东湖新技术开发区`），与「只留到县级和著名地点」的意图不符。

## 二、分层怎么分

| 层 | 内容 | 随包 | 缺省 | 开关 |
|---|---|---|---|---|
| **基础库** | `dict.qj`（8.7 万条：规范字、常用词、语料挖出的高频词、短语层、品牌词、人工挑的领域词） | 是 | 常开 | 不可关 |
| **领域包** | `dicts/*.qj`（洗过之后的 10 本，见第 1 步） | 是 | 只开成语 | 「词库」页逐本勾 |
| **扩展包** | `places-extended.qj`、`poetry_lines.qj`（名句，原 poetry_lines 移过来）、`internet_slang.qj`、`internet_slang_coarse.qj`（脏话） | 是 | 全关 | 同上 |
| **个人词库** | 用户自己选的词、敲错表、云端打字记录学到的词 | 否（本机） | —— | 「高级」页可清 |

**云端打字记录只能进个人词库，不进基础库**（第 2 步也是这条）：素笺云上传的输入日志是用户私人数据，
可以进他自己的词频与用户词，但一条都不许进随包数据。

## 三、第 1 步：洗 11 本领域包（只删）

### 规则

四组规则按顺序作用在 `03_domains/*.tsv` → 拆包这一步（现在在 `tools/dict-convert` 的 `lexicon` 里，
`lexicon/pack.rs` 读源、`lexicon/mod.rs` 的 `write_domains` 写 `dicts/<领域>.tsv` 与 `.qj`）：

- **R1 长度门槛**：领域包默认丢掉 **> 6 个字**的词。白名单（不删）：
  - `idioms` 整本不看长度（成语再长也是成语）；
  - 一份人工维护的固定书名 / 专名表 `assets/lexicon/00_meta/domain-keep.tsv`（一列 `word`，可写注释行），
    先收《红楼梦》《三国演义》这类书名与少数必须留的长专名。这份表是给人审的，只增不改规则。
- **R2 动物、医学：寄主 + 学名**：以单字宿主（人猪牛羊犬猫鸡鸭鹅鼠马鱼虾蟹蛇蛙鸽蚕蜂兔猴鹿狐貂貉豕禽畜螺蚊蝇蜱蚤虱）
  开头、以 虫 / 菌 / 毒 / 病 / 蚴 / 蜱 / 螨 / 虱 / 蚤 / 蝇 / 蚊 / 螺 / 绦 / 杆菌 / 球菌 / 病毒 结尾、且 ≥ 3 个字的词删掉（现量 73 条）。
  **不另设机构后缀规则**：实现时发现按「局 / 院 / 部」这类后缀删会误伤 `医院`、`卫生局`、`颈部` 这种常用词，
  而长机构名（`疾病预防控制中心`）本来就被 R1 的长度门槛全删了 —— 于是一条机构规则都不留，报告里那部分归到 R1。
- **R3 地名**：`places` 拆成两本 ——
  - 留 `places.qj`：**县级及以上的短名**（以 省 / 市 / 县 / 区 / 镇 / 乡 / 自治区 / 自治州 / 自治县 / 特别行政区 结尾且 ≤ 6 字）
    ＋ **著名地点**（`doc_freq ≥ --places-min-df`，缺省 500，现量约 4,400 条：`北海公园`、`塞舌尔`、`黄山` 这类）；
  - 其余（社区、巷、路、村、小区、开发区、以及 DF 低于阈值的）→ **`places-extended.qj`，缺省关**。
  - 阈值是旋钮：`--places-min-df` 可调，发布报告里要写清这次用的是哪个值。
- **R3.1 领域词门槛按语料规模给**（2026-10-07 加）：`--domain-keep-min` 那个绝对次数换成
  `--domain-keep-per-10m`（每千万句出现多少次才留基础库），绝对次数由 `lm-unigram.tsv` 的 `<s>`（句数）换算。
  起源：语料从约五千万句涨到 1.207 亿句（3.9 倍）之后，老口径 50 次等于门槛被悄悄放松，一次重建多放进基础库
  1.1 万条领域词（一中、一审法院、一夫一妻制……）。缺省 12（当前语料下 145 次）：先是 15，但实测把「商务中心」这类常用实体推进了缺省关着的领域包
  （外部对话集上酒店场景直接用不上），降到 12 留在基础库，领域源词约 1.9 万，与旧库 17,606 同量级。**以后换语料不用再改这个数**。
- **R3.2 维基标题来源的实体另给一条 fame 门槛**（2026-10-08 加）：`places.tsv` 里由
  `tools/corpus/mine_place_titles.py` 从维基条目标题挖出来的那些（来源列 `wikipedia-titles`），
  df 是**清洗后语料里出现的行数**，与 THUOCL 的 df 不是一个量级 —— 不硬凑，按同口径另定一条：
  `WIKI_TITLE_MIN_MENTIONS = 50`（dev 上定），来源列由 `pack.rs` 读进 `DomainRow.source`。
  品牌词（锦江之星这类）不在这里，归 L3 品牌包。
- **R3.3 迭代切词（两遍跑，2026-10-08 补）**：语料统计要用**独立 token 计数**，而计数依赖切词用的词库 ——
  词库越全，越多词能作为独立 token 出现（否则「彼此彼此」这种词因为切不出来，计数永远是 0，也就进不了
  基础库）。所以 `lexicon → bigram（用上一轮建出的词库当切词表）→ lexicon` 要迭代。
  **收敛判据：基础词表的差集（旧有新无、旧词频 ≥ 100 的词数）变化小于 1% 就停。**
  实测：1549 → 1157 → 740（三轮）。
- **R4 诗词名句**：`poetry_lines` 整本从「词库」里拿出去 ——
  - 现在那 13,703 条（含语料 ≥ 50 次因而留在基础库的那部分）**不再进 `dict.tsv`**，
    也就是把「领域词 ≥ 50 次留在基础库」这条对诗词名句关掉；
  - 整本改名 `poetry_lines.qj`（名句包，缺省关）随包；
  - 不走快捷短语层：名句不是天天打的组合，进短语层会白占候选位（`phrases.tsv` 的门槛是「总次数与对话语料次数都 ≥ 2000」，
    名句多半过不了，逐条开后门不值当）。

### 工程要求（这一条是发布门槛）

- 规则写在 `tools/dict-convert` 里，**不是手改 TSV**：源文件 `03_domains/*.tsv` 不动，
  同一份输入必须出同一份输出（按 `word` 排序、稳定分档、不依赖文件系统顺序），可重复跑。
- 每次拆包打一份报告（新文件 `data/generated/domain-report.tsv` 或 stdout 一段），逐本列：
  `包名` / `删前条数` / `删后条数` / `按哪条规则删的（R1/R2/R3/R4 各删多少）`，
  再**每本抽 20 条被删的样本**（按 `doc_freq` 降序取，保证抽到的是最「可惜」的那些，便于审）。
- 一条命令复现（第 4 步 `lexicon` 那行原话，见 `assets/lexicon/QINGJIAN.md`）：
  ```bash
  cargo run --release -p qingjian-dict-convert -- lexicon --pinyin data/generated/pinyin-llm.jsonl \
    --frequency data/generated/lm-unigram.tsv --extra-words assets/lexicon/mined_words.tsv \
    --extra-words assets/lexicon/phrases.tsv --extra-words assets/lexicon/brand.tsv \
    --extra-words assets/lexicon/domain_words.tsv
  ```
  （新增的规则参数都带缺省值，不传就是本规格定的那一套。）
- `.qj` 的 META（许可 / 署名）不变：`DOMAIN_LICENSE = "MIT AND Unicode-3.0"`、THUOCL + Unihan 署名照旧；
  新增的 `places-extended` / 名句包沿用同一份。

### 预计删减规模（按上面的规则，用今天的源数据估）

| 包 | 删前 | 预计删后 | 说明 |
|---|---:|---:|---|
| places | 44,803 | ≈ 4,400 | 其余 4 万条进 places-extended（缺省关） |
| poetry_lines | 13,703 | 0（进名句包） | 从基础库与词库里都拿掉 |
| law | 9,896 | ≈ 4,600 | 长度门槛删掉 5,285 条里的绝大多数（法条全名没人整块打） |
| animals | 17,287 | ≈ 14,500 | 长度 2,778 + 寄主学名 |
| medicine | 18,749 | ≈ 16,900 | 长度 1,848 + 寄主学名 |
| it_computing | 15,998 | ≈ 14,400 | |
| 其余 5 本（food / idioms / historical_figures / finance / automotive） | 36,731 | ≈ 36,100 | 成语整本不删，其余删掉长度超标的 |

**实测（2026-10-07，第 1 步实现后）**：本机没有 `data/corpus` 与 `data/unihan`，跑不了整条 pipeline，
于是按同一套规则在现有产物上跑了一遍（包的成员 = 领域源里有、基础库里没有的词，报告里成语包 3,699 对得上线上 3,693）：

| 包 | 送到规则 | 留用 | 删掉（R1 / R2） | 改派 |
|---|---:|---:|---:|---:|
| animals | 17,289 | 14,462 | 2,827（2,778 / 49） | —— |
| law | 9,899 | 4,614 | 5,285 | —— |
| places | 44,808 | 10,928 | 1,415 | 32,465 → places-extended |
| poetry_lines | 13,704 | 13,704 | 0 | 整本为名句包（其中 92 条从基础库移出） |
| idioms | 8,531 | 8,531 | 0 | —— |
| historical_figures | 13,657 | 13,657 | 0（11 条长人名进了白名单） | —— |
| 其余 5 本 | 39,075 | 35,847 | 3,228 | —— |

**基础词库 93,238 → 91,426**：移出 1,753 条诗句（本来因语料 ≥ 50 次留在基础库）+ 删 59 条长机构名（`上海证券交易所`、`全国人大常委会`）。

**评估：两组数字与基线一字不差**（首选 34.2% / 前三 39.9% / 前五 39.9% / 整句候选 34.2% / 字准 77.5%；
回放 词 1,057 条 88.6% / 97.6%、整句 165 条 62.4% / 63.0%、英文 21 条 76.2% / 95.2%），−0.3 的容差没用上。

审计定的白名单（2026-10-07 加进 `assets/lexicon/00_meta/domain-keep.tsv`）：长音译人名 11 条
（`陀思妥耶夫斯基`、`奥斯特洛夫斯基`、`车尔尼雪夫斯基`、`罗日杰斯特文斯基`、`斯塔夫里阿诺斯`、`克里斯托弗哥伦布`、`叶卡捷琳娜二世`，
以及 `元太祖成吉思汗`、`元文宗图帖睦尔`、`清太祖努尔哈赤`、`辽圣宗耶律隆绪` 这几条庙号 + 名的写法 —— 这类名字就是整块打出来的，删了简拼拼不出）；
自治区与特别行政区全名 5 条（`新疆维吾尔自治区`、`广西壮族自治区`、`宁夏回族自治区`、`香港特别行政区`、`澳门特别行政区`，填表写地址常整块打）。
加完之后历史人物包 0 删、地名包删 1,410 条（其余仍然是社区、巷弄、路口这类，进 places-extended）。

### 下一次在有完整数据的机器上跑全 pipeline（交底）

本机没有 `data/corpus/` 与 `data/unihan/`，上面这些数字是绕开 pipeline 跑的（规则一致、现产物上原地过滤）。
下次在有语料的机器上跑完 `assets/lexicon/QINGJIAN.md` 的第 1–4 步之后，要确认两件事，否则装机与发版拿到的还是旧的 `dict.qj`：

1. **产物与这次一致**：`data/generated/dict.qj` 的条数应是 91,426 上下（白名单生效后），
   `dicts/` 下是 10 本领域包 + `places-extended.qj` + `poetry_lines.qj`（名句包），
   `data/generated/domain-report.tsv` 的删减数与规格这一节对得上。
2. **按发版流程走**：`tools/release/data-bundle.sh` 把产品数据打成 `data-vN` Release，更新 `tools/release/data.lock`
   （含 SHA256SUMS）；CI 与自编译再用 `tools/release/data-fetch.sh` 按锁文件取。只把 `data/generated/` 换掉而不走这一步，
   别的机器（含装机包与 CI）拿到的还是旧数据。

第 2 步要用到的东西：语料（`data/corpus/`，中文维基 + LCCC 文本）与 Unihan（`data/unihan/Unihan_Readings.txt`）本机都没有 ——
挖新词、标读音都卡在这两处，数据从哪拿要用户定；另外 `gloss-gen pinyin` 那一步要一个 LLM 的 API 密钥。

## 四、第 2 步：填 `04_internet_slang`

### 数据形态

新目录 `assets/lexicon/04_internet_slang/`，与 `03_domains` 同格式的 TSV（可多份，按来源或专题分文件）：

```
word	pinyin	freq	year	source
内卷	nei juan	12000	2021	xxx
```

- `year`：这条词开始流行的年份（判断「过时的梗」用；同一批可以整批卸掉）。
- `source`：来源标识，与 `assets/lexicon/04_internet_slang/README.md` 里的清单对应。
- 进词库的路径与领域包一致（`lexicon` 读目录 → 拆包 → `internet_slang.qj`），语言模型按 `bigram --phrases` 合成分量词的计数
  （理由见 `docs/notes/domain-words.md`：新词当 token 统计会把成分词的二元证据吸走）。

### 来源与许可（硬条件）

- 只收**许可清楚**的来源：自己按公开语料挖（`dict-convert mine` 已有的路子）、明确 CC0 / MIT / OFL 之类的词表、
  或人工整理并逐条写明出处；抓取的榜单要写清抓取日期与站点条款。
- **不用**许可不清的：雾凇拼音、搜狗细胞词库，以及任何「网上流传的合集」。
- 来源、许可、抓取日期写进 `assets/lexicon/04_internet_slang/README.md`（与 `03_domains` 的 README 同一写法）。

### 收录标准

- **以 2–4 字为主**：超过 4 字的要能说出「天天会打」的理由（如「绝绝子」这类 3 字）；超过 6 字一律不收。
- **字母词走英文那条路**：`yyds`、`awsl`、`xswl` 这类**不要编假拼音** —— 它们进 `05_english` / `mixed_words.tsv`
  那条路（英文词表或中英混写），不进中文拼音词库；中文拼音词库只收汉字词。
- **带年份**：每条都标 `year`；「过时的梗」可以按年份整批卸掉（例如一刀切掉 `year ≤ 2015` 的那批），
  卸的时候走同一个 TSV，不改代码。
- **脏话单独一个包**：`internet_slang_coarse.qj`，缺省关，与 `internet_slang.qj` 分开，
  这样「词库」页里能只开干净的、或只开粗的。
- **进基础库的标准是「天天会打、同音不危险」**：
  - 天天会打：输入日志里真会整块选出来的（对照第 5 节的覆盖率尺子）；
  - 同音不危险：这个词的拼音不能与常用词撞出严重歧义（例如某个拼音已被一个高频常用词占住，
    新词频次又不高，就不要塞进基础库，留在网络用语包里）。

### 读音与词频

- **读音**：多音字可以让大模型先标（`qingjian-gloss-gen pinyin` 那条流水线），但**必须逐条对照 Unihan 核对**
  （`data/unihan/Unihan_Readings.txt`）；判定明细写进 `00_meta/polyphone-judgments.tsv`（沿用现有格式）。
- **R5 标注只换主读音，旧读音按「错没错」分档**：`lexicon --pinyin` 拿到的每个词，标注读音按原词频当主读音入库，
  原读音默认除以 `DISPUTED_READING_DIVISOR = 8` 保留 —— 只有原读音**本身是错的**才该这么降：
  - 两个读音都是规范读音、又都常用（谁 shui/shei、重装 chong/zhong、必得 bi de/bi dei、上调、行商、重犯、见长、完了）
    → 该词写 `"keep_both": true`：两个读音同权，不降权（把常用打法降成八分之一，等于打这个字直接退步）；
  - 旧读音是错的（且 ju、一宿 su、乐感 le、藏文 cang、还给 hai）→ 不写标记，按 ÷8；
  - 旧读音不是错的、只是常有人打（露 lou/lu 家词：露脸、露面、露馅儿、露马脚、露肩、露背……）→ 写 `"divisor": N`
    （露家 N=2），降权放轻。
  标记与判定分开存放、合成一个产物：`00_meta/polyphone-marks.tsv`（词 / 标记 / 理由）＋ `polyphone-verdicts.tsv` ＋
  `polyphone-auto.tsv` → `tools/lexicon/polyphone-apply.py` → `pinyin-corrections.jsonl`（**产物，不手改**）。
- **单字条目一律不进修正文件**（2026-10-07 复跑实测后定的）：单字走规范字表那条路（`readings.weighted` 按 Unihan
  读音频次分摊），不查标注 —— 谁 的 shui 29 / shei 1 就是字表分摊出来的，正合适；195 条单字修正全是空转。
  所以 `polyphone-apply.py` 把单字挡在门外（不删源里的判定记录，只是不写进 `pinyin-corrections.jsonl`），
  「把 谁 的主读定成 shei」这类单字判定要落地得先让字表那条路认标注，这一步没做之前不混进修正文件。
- **词频**：**对着同音的竞争词人工定**，不许批量灌一个魔数。写法沿用 `domain_words.tsv` / `mixed_words.tsv` 的规矩：
  次数与 `dict.tsv` 词频同一尺度，参照同音高频词的次数定（例：`C盘 8000 > 裁判 5416` 这种对着定；见 `assets/lexicon/QINGJIAN.md` 4e 步）。
  给不出理由的词，就当它进不了基础库。

## 五、验收（两步共用）

### 5.1 两条尺子的基线（2026-10-07 量的，改前改后都要对这两组数字）

命令（`tools/eval/offline.toml` 是本规格新加的：把 `[predict] enabled = false` 固定住，
CLI 不吃用户配置里的素笺云设置；领域包按产品缺省只开成语）：

```bash
cargo run --release -p qingjian-cli -- --config tools/eval/offline.toml \
  --eval-text data/eval/sentences.tsv --extra-dict data/generated/dicts/idioms.qj
cargo run --release -p qingjian-cli -- --config tools/eval/offline.toml \
  --replay data/eval/input-log-2026-10-04.jsonl --extra-dict data/generated/dicts/idioms.qj --misses 20
```

**整句评测**（`data/eval/sentences.tsv`，1878 句，冷启动）：

| 句子 | 首选 | 前三 | 前五 | 整句候选 | 字准确率 | 平均查询 |
|---:|---:|---:|---:|---:|---:|---:|
| 1878 | 34.2% | 39.9% | 39.9% | 34.2% | 77.5% | 0.7 ms |

**回放**（`data/eval/input-log-2026-10-04.jsonl`，内存学习不落盘）：

| 来源 | 条数 | 首选 | 前五 | 不在候选 | 平均名次 |
|---|---:|---:|---:|---:|---:|
| 词 | 1057 | 88.6% | 97.6% | 9 | 1.32 |
| 整句 | 165 | 62.4% | 63.0% | 61 | 1.01 |
| 英文 | 21 | 76.2% | 95.2% | 0 | 1.48 |

### 5.2 装机门槛（2026-10-07 加）

两条尺子过了才写 `data/generated/GATE_PASSED`（`tools/release/gate-pass.sh` 跑尺子并写标记，容差 −0.3 个点）。
装机脚本（`apps/macos/scripts/bundle.sh`、`cloud/ios/scripts/build-bridge.sh`）**没有这个标记就回退**到上次发版那份
`data/generated.shipped/`（那份从已装的 `Sujian.app/Contents/Resources/` 里另存：`dict.qj`、`lm.qj`、`dicts/*.qj`、
`glossary-*.qj`、`english.tsv`），两份都没有就拒绝装机。这样词库/语言模型没门槛的期间，谁在本机装机都不会把
不合格的数据带上去。已实测：缺标记时打出来的 app 里 `dict.qj` 与发版那份逐字节相同。

**判据**：第 1 步与第 2 步合并后，两边都不能比上表差。容差：首选 / 前三 / 字准确率各允许 **−0.3 个点**以内
（洗包只删不加，掉一点属正常；网络用语补进来的新词要能把这 0.3 个点挣回来，挣不回来就说明收的词不对）。
单次运行的抖动远小于 0.3 个点（同一份输入两次跑的数字一致），所以不设「多跑几次取平均」。

### 5.2 网络用语额外一把尺子：覆盖率

「选过两次以上但词库里没有」的那批词，看覆盖率有没有上去。现有流程是人工的，两步走：

1. 统计：`uv run cloud/scripts/input-profile.py`（读 `~/Library/Application Support/Qingjian/input-log.jsonl`，
   出 `missing_words` / `phrase_candidates` 两张表）；冻结日志那份在 `docs/notes/domain-words.md`
   有历史结论（262 条，按语料次数分三档：≥20 次 131、1–19 次 73、0 次 58）。
2. 人工挑：按 `docs/notes/domain-words.md` 的挑法过一遍，进 `domain_words.tsv`（基础库）或
   `04_internet_slang`（网络用语包）。

**判据**：`04_internet_slang` 落地后，同一份日志里「选过 ≥2 次且词库与短语层都没有」的条数要下降；
下降的名单里至少有第一档（语料 ≥ 20 次）的多数。**注意**：0 次那档（只在日志里出现的私密片段）不许进库，
留在个人词库（与 `domain-words.md` 现行规矩一致，也是「云端打字记录只进个人词库」那条）。

**这一条现在没有自动化命令**（`--replay` 只报命中率，`input-profile.py` 是产品日志侧的分析）。
本规格不要求在第 1、2 步里补，但两步做完要能说出这几组数字是怎么来的。

### 5.3 文档同步

- `docs/notes/crate-notes.md`：`tools/dict-convert` 那段补新规则与新参数（`--places-min-df` 等）、
  `data/generated/domain-report.tsv` 的位置；`[dictionaries] domains` 那段补新包名（缺省仍是只开成语）。
- `docs/user/settings/dictionaries.md` 与 `docs/user/settings/preferences.md` 的「词库」页描述：
  列出新的包（地名扩展 / 名句 / 网络用语 / 网络用语-粗口，全缺省关），说明「勾了才进候选」。
- `assets/lexicon/README.md` 与 `QINGJIAN.md`：目录与第 4 步的描述按新规则更新；
  `04_internet_slang/README.md` 写来源与许可（第 2 步的硬条件）。
- 版本号、CHANGELOG 不动（发版时统一处理）。

## 六、风险与回滚

- **删过头**：长度门槛对 `law` 最狠（删一半以上）。若回放里出现「当时整块选了某个法规名」这种条数上升，
  就把那个词补进 `domain-keep.tsv` 白名单，或按 `doc_freq` 给该类留一条更高的门槛；白名单只增不减地审。
- **覆盖率与排序打架**：新词进基础库会挤掉同音常用词（历史上 `词库` 当 token 统计时把 词 + 库 的证据吸走，
  整句掉 0.1 个点，见 `domain-words.md`）。所以新词一律先走 `bigram --phrases` 合成路，
  并且**先只进网络用语包（缺省关）**，等覆盖率与两条尺子的数字都好了，再挑「天天会打、同音不危险」的进基础库。
- **回滚**：两步都只改 `tools/dict-convert` 的参数与 `assets/lexicon/` 的数据文件；
  回滚 = 恢复这两个位置 + 重跑第 4 步。引擎与排序代码不动，所以不存在「回滚一半」的状态。

## 六点五、第 2 步的实施进度（2026-10-07）

材料（都不依赖语料，已入库）：

- `assets/lexicon/04_internet_slang/wip/candidates.tsv`：从维基 `Category:互联网用语`（含子分类）、
  维基词典 `Category:漢語網路用語`、条目「中国大陆网络用语列表」拉的 270 条候选（转简体、去掉词库已有的）；
- `assets/lexicon/04_internet_slang/wip/triage.tsv`：三类判定（基础库 20 / 网络用语包 176 / 粗口 14 / 台港待办 26 / 不收 9 / 争议词 25 —— 争议词按审定不收）；
- `assets/lexicon/04_internet_slang/internet_slang.tsv`（179 条，含近年补充）、
  `internet_slang_coarse.tsv`（14 条）、`assets/lexicon/internet_base.tsv`（27 条，等实测后并入基础库）。

读取（`lexicon/internet.rs`，已带单测：年份列解析、粗口与干净靠分文件分流、没填词频的兜底）：

- `lexicon --internet-dir <目录>`（缺省 `assets/lexicon/04_internet_slang`）读那些 TSV，一本打一本 `.qj`，两本都缺省关；
- `--internet-base <文件>` 才把「天天会打」那批并进基础词库（同音不危险没实测前不并）；
- 网络用语包的 META 用 `CC-BY-SA-4.0` 与维基 / 维基词典 / CC-CEDICT 的署名，不套 THUOCL 那套。

三列都已经填好（2026-10-07，**没有用上语料，也没用 LLM 密钥**）：

- **词频**：「对着同音竞争词定」不需要语料 —— 竞争词就在 `dict.tsv` 里（自带音节与词频）。包内那批 `clamp(最强竞争词, 100, 2000)`；
  基础库那批（审计改过）竞争词 < 200 才取 +1 拿首选、≥ 200 退回包（拉满、白给、老六、真香 因此退回）。逐条依据 `data/generated/internet-slang-freq-evidence.tsv`。
- **读音**：由大模型逐条给，再跑一遍 `lexicon`（读音不合法的会被丢掉，177 + 14 条一条没丢）；但那只证明字有这个读音、
  不证明词里读对了 —— 含多音字的 48 条另列抽查表 `data/generated/internet-slang-polyphone-check.tsv`（词 / 选用读音 / 该字还读什么）。
- **年份**：维基条目正文能站住的按条目写（16 条），其余 `unknown`（160 条，不进以后的按年份卸载分组），依据 `data/generated/internet-slang-year-evidence.tsv`。

验收（两本包都缺省关，所以与基线一致）：`--eval-text` 首选 34.2% / 前三 39.9% / 字准 77.5%，
`--replay` 词 1057 条 88.6% / 97.6%、整句 165 条 62.4%、英文 21 条 76.2%。
开包冒烟：`yunwanjia` 在不开包时只有整句候选，开包后「云玩家」成为词候选且排第一。

仍然要等语料的：① 那 9 个「两处都没有」的近义词（淡人、显眼包、发疯文学、破圈、卷王、边界感、向上管理、亚子、栓Q）靠 `mine` 挖；
② 第 1 步的正式重跑（要语料算词频与拆包）；③ 基础库那 26 条并库后的评估。

## 七、做完这两步之后再说什么

（不在本次范围，记一笔以免跑偏）：基础库自身的再分层（常用 / 次常用按频段拆）、
按用户画像推荐领域包、领域包体积与内存（现在 7 MB / 13 万条）、
网络用语的自动更新（按年份与语料频次定期重跑挖词）。
