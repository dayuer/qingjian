#!/bin/bash
# 用法：accept_q.sh <模型目录> <标签>：产品底座（dict d25f200a + lm bc19ea57）上跑四把尺子的融合与生成、回放 clean2，明细写 out-<标签>/
set -uo pipefail
W=/Users/liyuqing/sproot/qingjian-dict-hunt; B=$W/target/release/qingjian-cli; E=/Users/liyuqing/sproot/qingjian/data/eval
LIB=$W/.lab/bm/lib-1cbase; M=$(cd "$1" && pwd); tag=$2; R=$W/.lab/quant/run-$tag; O=$W/.lab/quant/out-$tag
rm -rf $R $O; mkdir -p $R/data $O
ln -s $LIB $R/data/generated; ln -s $E $R/data/eval; ln -s $W/assets $R/assets; ln -s $W/apps $R/apps
{ echo "dict $(shasum -a 256 $LIB/dict.qj | cut -c1-8)  lm $(shasum -a 256 $LIB/lm.qj | cut -c1-8)  model $(shasum -a 256 $M/model.safetensors | cut -c1-8)  cli $(cd $W && git rev-parse --short HEAD)"; } > $O/fingerprint.txt
cd $R
for s in sentences dialog-holdout-frozen prose-holdout-frozen external-frozen; do
  $B --config /tmp/eval-config.toml --eval-text data/eval/$s.tsv --neural $M --misses 0 --eval-details $O/fuse-$s.jsonl > $O/fuse-$s.txt 2>&1 &
  $B --config /tmp/eval-config.toml --eval-text data/eval/$s.tsv --eval-generate $M --misses 0 --eval-details $O/gen-$s.jsonl > $O/gen-$s.txt 2>&1 &
done
$B --config /tmp/eval-config.toml --replay data/eval/input-log-2026-10-04.jsonl --neural $M --clean2 --misses 0 --replay-details $O/rd.jsonl > $O/replay.txt 2>&1 &
wait
echo done > $O/DONE
