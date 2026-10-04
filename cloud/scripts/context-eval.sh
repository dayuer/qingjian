#!/usr/bin/env bash
# 用法：cloud/scripts/context-eval.sh <标签>  —— 对照配置跑 eval-context ×3、replay ×3、eval-text ×1，只打关键行
set -u
cd /Users/liyuqing/sproot/qingjian-context-prediction
B=(cargo run --release -q -p qingjian-cli -- --config /tmp/eval-config.toml --neural data/models/hanzhang-tongbian --neural-async)
echo "##### $1"
for i in 1 2 3; do echo "--- eval-context run $i"; "${B[@]}" --eval-context cloud/data/eval/context-pairs.tsv --misses 0 2>&1 | grep -E "^对数|按键同步"; done
for i in 1 2 3; do echo "--- replay run $i"; "${B[@]}" --replay data/eval/input-log-2026-10-04.jsonl --misses 0 2>&1 | grep -E "^词|^整句|按键同步"; done
echo "--- eval-text"; "${B[@]}" --eval-text data/eval/sentences.tsv --misses 0 2>&1 | grep -E "^句子"
