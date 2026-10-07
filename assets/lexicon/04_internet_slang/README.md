# 网络用语（04_internet_slang）

词库分层的第 2 步填这个目录（规格见 `docs/plan/dictionary-layering.md`）。这里的词进 `dicts/internet_slang.qj`
（缺省关）与 `internet_slang_coarse.qj`（脏话，单独一本，也缺省关）；字母词（yyds、awsl）**不进这里**，
走 `05_english` 或 `assets/lexicon/mixed_words.tsv` 那条路，不给汉字词编假拼音。

## 来源与许可（只收许可清楚的）

| 来源 | 许可 | 拿什么 | 怎么拿 |
|---|---|---|---|
| 中文维基百科 | CC BY-SA 4.0 | `Category:互联网用语`（89 页）与子分类「网络流行语」（31 页）的条目名；条目「中国大陆网络用语列表」按年代分节，`year` 字段从它的分节推 | 已有语料（`data/corpus/` 的维基文本）或按分类抓条目名 |
| 维基词典（zh.wiktionary） | CC BY-SA 4.0 | `Category:汉语网络用语` 的词条名与释义（释义只用来判断是不是「天天会打」的真词） | dump 或 API 按分类取 |
| CC-CEDICT | CC BY-SA 4.0 | 里面已收的新词 / 网络用语（补充候选，不整表导入） | 官网下载 |
| 自建挖词 | 中文维基 CC BY-SA 4.0 + LCCC MIT | 从 `data/corpus/` 挖 n-gram 与 `dict-convert mine`，取语料频次够、词库没有的 2–4 字词 | `cargo run -p qingjian-dict-convert -- mine data/corpus/*.txt` |

LCCC 的许可原文：清华 CoAI 发布在 Hugging Face 的 [`thu-coai/lccc`](https://huggingface.co/datasets/thu-coai/lccc)，
数据集卡片上的 `license` 字段是 `mit`（2026-10-07 用 HF 的 API 核过：`license: mit`）。中文维基的许可见
[Wikimedia 的使用条款](https://foundation.wikimedia.org/wiki/Policy:Terms_of_Use/zh)（CC BY-SA 4.0）。

CC BY-SA 的 ShareAlike 与仓库的 GPL-3.0-or-later 相容（Creative Commons 的官方兼容列表），
署名写进 `.qj` 的 METADATA，与 THUOCL / Unihan 同一套做法。

**明确排除**（许可不清或不可商用）：

- 雾凇拼音（GPL 的口径与本仓库不做的事不同，且仓库此前已彻底移除，不再引入）；
- 搜狗细胞词库、各类「网络流行语大全」转载合集（来源与许可都查不到）；
- 萌娘百科（CC BY-NC-SA，NC 不可商用）；
- 小鸡词典等专有产品；
- 《咬文嚼字》这类年度榜单（榜单本身版权不明；真要引用某年的词，改从维基百科同年条目里取，出处写维基）；
- 用户自己的输入日志与云端打字记录（**只进个人词库，一律不进随包词库**）。

## 已写出的文件

**这个目录顶上只放「包」**（一个 TSV 一本包，文件名主干就是包名）：`internet_slang.tsv` 与
`internet_slang_coarse.tsv` 会被 `lexicon` 读走打成 `.qj`；工作稿（候选表、判定稿）放 `wip/` 子目录，
不会被打包（目录只读顶层的 `*.tsv`）。

| 文件 | 内容 | 状态 |
|---|---|---|
| `internet_slang.tsv` | 干净的网络用语（174 条，缺省关） | 词频、读音、年份都填好了（怎么填的见下） |
| `internet_slang_coarse.tsv` | 粗口单独一包（14 条，缺省关） | 同上 |
| `assets/lexicon/internet_base.tsv` | 够「天天会打」、准备并进基础库的（26 条） | 次数按同音竞争词定好；走 `--internet-base` 并库时仍要跑一次评估 |

### 没有语料、没有 LLM 密钥时这三列怎么定（2026-10-07）

- **词频**：规格说的「对着同音的竞争词人工定」不需要语料 —— 竞争词就是 `dict.tsv` 里同音节串的词（它自带音节与词频）。
  规则：`词频 = clamp(最强竞争词的词频, 100, 2000)`；逐条依据在 `data/generated/internet-slang-freq-evidence.tsv`。
  **基础库那批**另加一道「同音不危险」实测：竞争词强过 2000 就退回网络用语包（**真香**就是这样退的 —— 同音的真相同音词频 9162，打字时多半想要的是「真相」）。
- **读音**：由大模型（本会话）逐条给出，再放进 `lexicon` 跑一遍 —— 读音不合法（Unihan 里该字没有这个音节）的词会被引擎丢掉，
  跑完 174 + 14 条一条没丢，等于用仓库自己的 Unihan 校验器核过了。
- **年份**：维基条目正文里的「某年流行」靠不住（人物条目会抽到生年），所以只按能站住的条目写，其余一律 `unknown`；
  16 条有年份、160 条 `unknown`，依据在 `data/generated/internet-slang-year-evidence.tsv`。`unknown` 的不进以后的按年份卸载分组。
| `wip/candidates.tsv` | 从维基与维基词典拉的原始候选（270 条，未过目） | —— |
| `wip/triage.tsv` | 270 条的三类判定（人工过目稿，含理由） | 审计已审 |

词频与年份到位之前，这三个 TSV 先不接进 `lexicon`（读取格式与规则还没写）。

## 候选表（2026-10-07 拉的第一版）

`candidates.tsv`：从上面三个不依赖语料的来源取的原始候选（中文维基 `Category:互联网用语` 含子分类、
维基词典 `Category:漢語網路用語`、条目「中国大陆网络用语列表」的列表项），按 OpenCC 的 t2s 表转成简体、
去掉词库里已有的，共 **270 条**。**未经人工过目**：里面混着台/港用语、专有名词（任天堂、伊拉克这类误入的）
与并不「天天会打」的词，过一遍再决定留哪些。

另外两类没收进来：字母词 64 个（1450、233、404、yyds 这类，走 `05_english` 或 `mixed_words.tsv` 那条路，
不给汉字词编假拼音）、≥5 字汉字 36 个。

可能要给大模型标读音的：270 条里有 76 条含 `kHanyuPinlu` 记录了两个以上读音的字
（清单在 `data/generated/internet-slang-pinyin-todo.txt`；正式的待标清单由 `lexicon --emit-ambiguous` 产出，比这条粗筛精确）。

## Unihan

读音来自 `data/unihan/Unihan_Readings.txt`（不进 git）：2026-10-07 从
<https://www.unicode.org/Public/UCD/latest/ucd/Unihan.zip> 下载，
**Unicode 18.0.0**（文件头日期 2026-07-31），`Unihan.zip` 的 SHA256：
`4c93ea9c1f636451729a840978f1667a53886af37ba854fdcce109721c63d43e`。

## 收词原则（2026-10-07 审计定）

- **政治绰号与辱骂语不收，也不另立「争议包」**：输入法不随包分发这类词。用户自己要打，靠个人学习就能记住
  （`triage.tsv` 里那 25 条争议词按这条处理，理由逐条写在表里）。
- 台港用语另记一份待办（`data/generated/internet-slang-tw-hk-todo.txt`），以后要做繁体 / 台港词表时再用。
- 专有名词（厂商名、活动名这类误入的）不收。
- 词必须有干净来源，且是「整块会打出来」的词；凭印象手写不收 —— 查不到来源的等语料挖词补。

## 近年口语的缺口（2026-10-07 查）

方案里点名的那批近年口语，先查了一遍在不在词库里：**遥遥领先、上头、摸鱼、躺赢、滤镜、巨婴、内耗** 已经在基础库；
缺的这两处来源能补上（维基条目 / 分类里的 8 条，CC-CEDICT 里带读音的 12 条），已写进 `internet_slang.tsv` 与 `internet_base.tsv`：

- 维基有：内卷、躺平、搭子、整活、松弛感、真香、破防、city不city；
- CC-CEDICT 有：摆烂、绝绝子、情绪价值、电子榨菜、社死、拉满（其余同上，带 `neologism`/`slang` 标注与读音）；
- **两处都没有**：淡人、显眼包、发疯文学、破圈、卷王、边界感、向上管理、亚子、栓Q —— 等语料挖词来补，
  不凭印象手写（清单在 `data/generated/internet-slang-recent-gap.txt`）。

## 数据形态

与 `03_domains/` 同格式的 TSV（可多份按来源分文件）：

```
word	pinyin	freq	year	source
内卷	nei juan	12000	2021	wikipedia
```

- `year`：这条词开始流行的年份（过时的梗可以按年份整批卸掉，改这个文件即可，不动代码）；
- `source`：来源标识，与上表对应；
- `freq`：词频，与 `dict.tsv` 同一尺度，**对着同音的竞争词人工定**（`C盘 8000 > 裁判 5416` 那种做法，见 `QINGJIAN.md` 4e），不许批量灌；
- `pinyin`：多音字可以让 LLM 先标（`gloss-gen pinyin`），但必须逐条对照 Unihan 核对（判定明细进 `00_meta/polyphone-judgments.tsv`）。

进基础库的标准是「天天会打、同音不危险」；先只进网络用语包（缺省关），等覆盖率与两条尺子都好了再挑少数进基础库。
