#!/usr/bin/env bash
# 扫知微词级重排的权重 λ_w：产品配置（通变 + 知微），每个 λ 跑 eval-context / replay / eval-text
set -u
cd "$(dirname "$0")/../.."
BIN=target/release/qingjian-cli
A=(--config /tmp/eval-config.toml --neural data/models/hanzhang-tongbian --neural-async --word-model data/models/hanzhang-zhiwei)
for w in "$@"; do
  echo "== word-weight=$w"
  $BIN "${A[@]}" --word-weight $w --eval-context cloud/data/eval/context-pairs.tsv --misses 0 2>&1 | grep -E "^对数|按键同步"
  $BIN "${A[@]}" --word-weight $w --replay data/eval/input-log-2026-10-04.jsonl --misses 0 2>&1 | grep -E "^词|^整句|按键同步"
  $BIN "${A[@]}" --word-weight $w --eval-text data/eval/sentences.tsv --misses 0 2>&1 | grep -E "^句子"
done
echo SWEEP-DONE
