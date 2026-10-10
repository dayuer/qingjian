#!/bin/bash
# 用法：accept_q.sh <模型目录> <标签>：产品底座（dict d25f200a + lm bc19ea57）上跑四把尺子的融合与生成、回放 clean2，明细写 out-<标签>/
set -uo pipefail
R=${QJ_REPO:-$(cd "$(dirname "$0")/../.." && pwd)}   # 脚本所在仓库（主检出）
A=${QJ_ARCHIVE:-$R/data/archive}                  # 2026-10-10 三个 worktree 的 .lab 合并迁到这里
B=$R/target/release/qingjian-cli; E=$R/data/eval; L=$A/dict-hunt-lab
LIB=$L/bm/lib-1cbase; M=$(/tmp/q-venv/bin/python -c "import os,sys;print(os.path.realpath(sys.argv[1]))" "$1"); MF=$M; [ -d $M ] && MF=$M/model.safetensors; tag=$2; RUN=$L/quant/run-$tag; O=$L/quant/out-$tag
rm -rf $RUN $O; mkdir -p $RUN/data $O
ln -s $LIB $RUN/data/generated; ln -s $E $RUN/data/eval; ln -s $R/assets $RUN/assets; ln -s $R/apps $RUN/apps
{ echo "dict $(shasum -a 256 $LIB/dict.qj | cut -c1-8)  lm $(shasum -a 256 $LIB/lm.qj | cut -c1-8)  model $(shasum -a 256 $MF | cut -c1-8)  cli $(cd $R && git rev-parse --short HEAD)"; } > $O/fingerprint.txt
cd $RUN
for s in sentences dialog-holdout-frozen prose-holdout-frozen external-frozen; do
  $B --config /tmp/eval-config.toml --eval-text data/eval/$s.tsv --neural $M --misses 0 --eval-details $O/fuse-$s.jsonl > $O/fuse-$s.txt 2>&1 &
  $B --config /tmp/eval-config.toml --eval-text data/eval/$s.tsv --eval-generate $M --misses 0 --eval-details $O/gen-$s.jsonl > $O/gen-$s.txt 2>&1 &
done
$B --config /tmp/eval-config.toml --replay data/eval/input-log-2026-10-04.jsonl --neural $M --clean2 --misses 0 --replay-details $O/rd.jsonl > $O/replay.txt 2>&1 &
wait
echo done > $O/DONE
