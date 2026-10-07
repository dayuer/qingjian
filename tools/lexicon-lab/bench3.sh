#!/bin/bash
# 用法：EXTRA="--neural …" EVAL_ONLY=1 bench3.sh <库名> <标签> [二进制]；结果写 .lab/bench-<标签>.txt
W=/Users/liyuqing/sproot/qingjian-dict-hunt; L=$W/.lab; v=$1; tag=$2; B=${3:-$W/target/release/qingjian-cli}
R=$L/run-$v; mkdir -p $R/data
for pair in "data/generated:$L/libs/$v" "data/eval:/Users/liyuqing/sproot/qingjian/data/eval" "data/models:/Users/liyuqing/sproot/qingjian/data/models" "assets:$W/assets" "apps:$W/apps"; do
  [ -e "$R/${pair%%:*}" ] || ln -sfn "${pair#*:}" "$R/${pair%%:*}" 2>/dev/null
done
cd $R; rm -f $L/rd-$tag.jsonl $L/ed-$tag.jsonl
if [ -z "$EVAL_ONLY" ]; then
  $B --config /tmp/eval-config.toml $EXTRA --replay data/eval/input-log-2026-10-04.jsonl --clean2 --misses 0 --replay-details $L/rd-$tag.jsonl > $L/replay-$tag.txt 2>/dev/null &
fi
$B --config /tmp/eval-config.toml $EXTRA $EVAL_EXTRA --eval-text data/eval/sentences.tsv --misses 0 --eval-details $L/ed-$tag.jsonl > $L/eval-$tag.txt 2>/dev/null &
wait
{ echo "== $tag"; grep -E "^句子|生成|P2C" $L/eval-$tag.txt | head -4; [ -f $L/replay-$tag.txt ] && sed -n '/干净口径 2/,/整句 /p' $L/replay-$tag.txt | tail -2; } > $L/bench-$tag.txt
