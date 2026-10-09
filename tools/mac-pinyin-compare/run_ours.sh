#!/bin/bash
# 对照测试的「我们这边」：随包旧库词图 + 通变融合，空学习、空上文，取首选。
#
# 词图与模型都用绝对路径显式给，不走 cwd（`--dict` / `--language-model` 见 apps/cli）。
# 逐条结果写 JSONL（含每句的 `top`，判定谁对谁错要用）。
#
# 用法：bash tools/mac-pinyin-compare/run_ours.sh [模型目录] [输出前缀]

set -eu
cd /Users/liyuqing/sproot/qingjian-neural

CLI=target/release/qingjian-cli
CONFIG=.lab/cli-config.toml
TASK=tools/mac-pinyin-compare/task-300.tsv
SHIPPED=/Users/liyuqing/sproot/qingjian/data/generated.shipped
MODEL="${1:-.lab/tongbian}"
PREFIX="${2:-.lab/mac-compare/ours}"

mkdir -p "$(dirname "$PREFIX")"
"$CLI" --config "$CONFIG" \
    --dict "$SHIPPED/dict.qj" --language-model "$SHIPPED/lm.qj" \
    --eval-text "$TASK" --neural "$MODEL" \
    --misses 0 --eval-details "$PREFIX.jsonl" > "$PREFIX.log" 2>&1
echo "写出 $PREFIX.jsonl"
tail -4 "$PREFIX.log"
