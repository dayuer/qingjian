#!/usr/bin/env bash
# 扫同输入串选择加分系数 β：对照配置，--replay 词首选必须不低于基线，--eval-context 取最好的
set -u
cd "$(dirname "$0")/../.."
BIN=target/release/qingjian-cli
A=(--config /tmp/eval-config.toml --neural data/models/hanzhang-tongbian --neural-async)
for b in "$@"; do
  echo "== choice=$b"
  $BIN "${A[@]}" --tune choice=$b --replay data/eval/input-log-2026-10-04.jsonl --misses 0 2>&1 | grep -E "^词|^整句"
  $BIN "${A[@]}" --tune choice=$b --eval-context cloud/data/eval/context-pairs.tsv --misses 0 2>&1 | grep -E "^对数"
  $BIN "${A[@]}" --tune choice=$b --eval-text data/eval/sentences.tsv --misses 0 2>&1 | grep -E "^句子"
done
echo SWEEP-DONE
