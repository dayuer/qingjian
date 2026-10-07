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
