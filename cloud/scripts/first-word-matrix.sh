#!/usr/bin/env bash
set -u
cd /Users/liyuqing/sproot/qingjian-context-prediction
BIN=target/release/qingjian-cli
A=(--config /tmp/eval-config.toml --neural data/models/hanzhang-tongbian --neural-async)
for mode in ctx start mix0.3 mix0.5; do
  export QJ_FIRST=$mode
  echo "##### mode=$mode"
  for i in 1 2 3; do echo "--- eval-context $i"; $BIN "${A[@]}" --eval-context cloud/data/eval/context-pairs.tsv --misses 0 2>&1 | grep -E "^对数|按键同步"; done
  for i in 1 2 3; do echo "--- replay $i"; $BIN "${A[@]}" --replay data/eval/input-log-2026-10-04.jsonl --misses 0 2>&1 | grep -E "^词|^整句|按键同步"; done
  echo "--- eval-text"; $BIN "${A[@]}" --eval-text data/eval/sentences.tsv --misses 0 2>&1 | grep -E "^句子"
done
echo MATRIX-DONE
