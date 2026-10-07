#!/usr/bin/env bash
# 语料准备：Hugging Face 的 parquet → `data/corpus/*.txt`（`dict-convert bigram` 的输入）。
#
# 两步，各出一份报告（`/tmp/*-clean.txt` 与 `/tmp/*-dedup.txt`，复核有没有误删正文）：
#   1. parquet_to_text.py  丢非正文行（维基章节标题、模板/HTML 残留、只有标点的行）
#   2. sort | uniq -c | cap_repeats.py  整行完全相同的封顶到 3 次（模板句、对话里的短回复）
# 去重走外排（sort 用 `-S 1G -T /tmp`），不把 2500 万行读进内存。
#
# 用法：tools/corpus/build_corpus.sh          （缺省用 `uv run`；本机也常用
#       PYTHON=/tmp/corpus-venv/bin/python tools/corpus/build_corpus.sh 免得 uv 每次解析依赖）
set -euo pipefail

cd "$(dirname "$0")/../.."
PYTHON="${PYTHON:-uv run}"

echo "[$(date +%H:%M:%S)] 维基：parquet → 文本（丢非正文行）"
$PYTHON tools/corpus/parquet_to_text.py data/corpus/zhwiki-20231101-zh-*.parquet \
  -o /tmp/zhwiki.raw --report /tmp/zhwiki-clean.txt
echo "[$(date +%H:%M:%S)] 维基：重复行封顶"
sort /tmp/zhwiki.raw -S 1G -T /tmp | uniq -c \
  | python3 tools/corpus/cap_repeats.py --max-repeat 3 -o data/corpus/zhwiki.txt --report /tmp/zhwiki-dedup.txt
rm -f /tmp/zhwiki.raw

echo "[$(date +%H:%M:%S)] LCCC：parquet → 文本"
$PYTHON tools/corpus/parquet_to_text.py data/corpus/lccc-base-train-*.parquet --column dialog \
  -o /tmp/lccc.raw --report /tmp/lccc-clean.txt
echo "[$(date +%H:%M:%S)] LCCC：重复行封顶"
sort /tmp/lccc.raw -S 1G -T /tmp | uniq -c \
  | python3 tools/corpus/cap_repeats.py --max-repeat 3 -o data/corpus/lccc.txt --report /tmp/lccc-dedup.txt
rm -f /tmp/lccc.raw

echo "[$(date +%H:%M:%S)] 完成"
wc -l data/corpus/zhwiki.txt data/corpus/lccc.txt
