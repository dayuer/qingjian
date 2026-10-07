#!/usr/bin/env bash
# 分语域各跑一遍 bigram：对话侧与书面侧各出一份 lm-unigram / lm-bigram，
# 之后合成三套 LM（对话 / 书面 / 混合，混合的二元按语域分配额截断）。
set -euo pipefail
cd "$(dirname "$0")/../.."
BIN=./target/release/qingjian-dict-convert

run() {
  local register="$1"
  local corpus="$2"
  local out="/tmp/lm-$register"
  rm -rf "$out"; mkdir -p "$out"
  echo "[$(date +%H:%M:%S)] bigram $register ($corpus)"
  ""$BIN"" --out-dir "$out" bigram --dict data/generated/dict.tsv --max-bigrams 5000000 \
    --phrases assets/lexicon/phrases.tsv --phrases assets/lexicon/domain_words.tsv \
    --brand assets/lexicon/brand.tsv --brand assets/lexicon/mixed_words.tsv "$corpus" > "$out/bigram.log" 2>&1
  echo "[$(date +%H:%M:%S)] $register done: $(wc -l < "$out/lm-bigram.tsv") bigrams, $(wc -l < "$out/lm-unigram.tsv") unigrams"
}

run dialog data/corpus/lccc.txt
run prose data/corpus/zhwiki.txt
echo "[$(date +%H:%M:%S)] both done"
