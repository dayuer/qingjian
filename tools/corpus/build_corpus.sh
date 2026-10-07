#!/usr/bin/env bash
# 语料准备：Hugging Face 的 parquet → `data/corpus/*.txt`（`dict-convert bigram` 的输入）。
#
# 两件事，都交给 `tools/corpus/clean_corpus.py`（**Python + sqlite 计数**）：
#   1. 丢非正文行（维基章节标题、模板/HTML 残留、只有标点的行），长度与汉字门槛；
#   2. 整段对话去重 + 单句封顶。
# 同时写一份不再改动的 `<名>-raw.txt`（原始文本不覆盖、派生文件另起名字）。
#
# **不要再用 sort / uniq -c / awk 数中文** —— 2026-10-08 的事故：macOS 那三个工具在 CJK 上给出
# 「LCCC 前 20 句占 85.8%」这种假数字，把整条对话侧带偏了一整轮实验。数数一律用 Python。
#
# 用法：PYTHON=/tmp/corpus-venv/bin/python tools/corpus/build_corpus.sh
set -euo pipefail

cd "$(dirname "$0")/../.."
PYTHON="${PYTHON:-uv run}"

echo "[$(date +%H:%M:%S)] LCCC"
$PYTHON tools/corpus/clean_corpus.py \
  --parquet data/corpus/lccc-base-train-0000.parquet data/corpus/lccc-base-train-0001.parquet \
  --column dialog --register dialog --dedup-dialogs --max-repeat 10000 --min-chars 2 \
  --db /tmp/lccc-counts.sqlite --out data/corpus/lccc.txt --raw-out data/corpus/lccc-raw.txt \
  --report /tmp/lccc-clean-report.txt

echo "[$(date +%H:%M:%S)] 维基"
$PYTHON tools/corpus/clean_corpus.py \
  --parquet data/corpus/zhwiki-20231101-zh-0000.parquet data/corpus/zhwiki-20231101-zh-0001.parquet \
  data/corpus/zhwiki-20231101-zh-0002.parquet data/corpus/zhwiki-20231101-zh-0003.parquet \
  data/corpus/zhwiki-20231101-zh-0004.parquet data/corpus/zhwiki-20231101-zh-0005.parquet \
  --column text --register prose --max-repeat 1000 --min-chars 8 --min-han 5 \
  --db /tmp/wiki-counts.sqlite --out data/corpus/zhwiki.txt --raw-out data/corpus/zhwiki-raw.txt \
  --report /tmp/zhwiki-clean-report.txt

echo "[$(date +%H:%M:%S)] 完成"
wc -l data/corpus/*.txt
