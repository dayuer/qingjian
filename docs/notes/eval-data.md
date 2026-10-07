# 评测数据与口径（2026-10-08）

词库/语言模型的改动拿什么量、数据从哪来、哪些数能用来调参、哪些只能用来验收。数据本身放在
`data/eval/`（`/data` 在 `.gitignore` 里，**不进 git、不随包**），这份文档记录来源、许可与口径。

## 一、几份数据

| 文件（`data/eval/`） | 是什么 | 用途 |
|---|---|---|
| `sentences.tsv` | 主语料上的整句评测集（1878 句，三列 `句子\t拼音\t前文`） | 整句评测的老基线 |
| `input-log-2026-10-04.jsonl` | 冻结的输入日志 | 回放评测（词级 / 整句 / 英文） |
| `{dialog,prose}-holdout.txt` | LCCC / 维基切出的留出集各 1000 句（纯文本一行一句） | 验收（不许调参） |
| `{dialog,prose}-dev.txt` | 同上的开发集各 2000 句 | 调参、挑权重 |
| `external-holdout.txt` / `external-frozen.tsv` | CrossWOZ 用户侧 1000 句 / 其三列冻结题 1779 条 | 验收（三边都没见过的一档） |
| `polyphone-disputed.txt` | 仍有分歧的多音字词（150 个） | 造题时剔掉含这些词的句子 |
| `dialog-*.jsonl` / `prose-*.jsonl`（在 `data/train/`） | 造题原料：`{id, register, prev, text}` | 「解码器在环」的判别式训练 |

**切分规则**：三份互不重叠，且都从训练语料里剔掉（按精确匹配，Python 做，剔除条数写进各脚本的报告）。
对话语料里一行是分词后的「哈 哈」形式，留出集里是句子，两边按**去空格后的形式**对比。

## 二、许可（原文只在本机用，不随包）

| 来源 | 许可 | 核实方式 |
|---|---|---|
| LCCC（清华 CoAI，`thu-coai/lccc`） | MIT | Hugging Face 数据集卡片的 `license` 字段（2026-10-07 核） |
| 中文维基百科（`wikimedia/wikipedia` 20231101.zh） | CC BY-SA 4.0 | 数据集卡片；署名随 `.qj` 的 META |
| CrossWOZ（清华 CoAI） | **Apache-2.0** | `https://api.github.com/repos/thu-coai/CrossWOZ/license` |
| KdConv（清华 CoAI，备着未用） | **Apache-2.0** | 同上仓库 |
| THUOCL | MIT | 许可原文已随仓库（`assets/lexicon/00_meta/THUOCL_LICENSE.txt`） |
| Unihan | Unicode License v3 | `data/unihan/` |
| CC-CEDICT | CC BY-SA 4.0 | 只作读音审计的参考，不整表入库 |

明确不用：微博来源、IndustryCorpus2、许可不明的转载合集。

## 三、口径（防作弊）

- **调参只能用**：`{dialog,prose}-dev.txt`、回放日志的**前一半**。
- **只能用来验收**（每次改动跑一次，不许回头挑参数）：`{dialog,prose}-holdout.txt`、`external-frozen.tsv`、
  回放日志的**后一半**、以及 `sentences.tsv`。
- 回放分**原始口径**与**干净口径**（剔被撤销的、上屏后删掉重打的、汉字夹拉丁字母且拼不出来的，
  规则与实现在 `apps/cli/src/replay/clean/`）；+10% 的线按干净口径算，原始口径只要求不低于旧库。
- 评测各系统读**同一份三列冻结题**；不许各自按自己的读音现转拼音，否则题目本身就不一样了。
- 旧库在 `{dialog,prose}-holdout` 上是**见过**这些句子的（它训练时用的是完整 LCCC/维基），
  所以那两档的基线要用 `old-retrained`（按旧配方、在剔掉留出集与开发集的语料上重建）；
  外部集（CrossWOZ）旧库没见过，可以直接用旧库当基线。

## 四、怎么生成

```bash
# 1. 语料（Python + sqlite，别用 shell 工具数中文：见 tools/corpus/clean_corpus.py 的文件头）
PYTHON=/tmp/corpus-venv/bin/python tools/corpus/build_corpus.sh

# 2. 切分与造题原料（留出 / 开发 / 造题；留出与开发会从训练语料里剔掉）
python3 tools/corpus/make_task_material.py --corpus data/corpus/lccc.txt --register dialog \
  --exclude-words data/eval/polyphone-disputed.txt --min-len 6 --min-han 4 --han-ratio 0.5 --max-foreign 3
python3 tools/corpus/make_task_material.py --corpus data/corpus/zhwiki.txt --register prose \
  --exclude-words data/eval/polyphone-disputed.txt --max-foreign 5

# 3. 外部留出集（CrossWOZ，取用户侧话语；三列冻结题用清洗库的读音转）
#    见 data/eval/README.md 里的步骤与 sha256
```

每个产物都记 sha256（前 16 位）在报告里；报数时**必须带产物指纹**（`dict.qj` / `lm.qj`），
不然版本对不上（2026-10-08 就出过一次：同一张表里两行是不同产物）。
