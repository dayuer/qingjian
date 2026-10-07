#!/bin/bash
# 用法：shard_eval.sh <题目tsv> <标签> <分片数> <额外参数…> —— 在 run-clean1 上分片跑 eval-text，明细按原顺序合并到 work/fuse/<标签>.jsonl
L=/Users/liyuqing/sproot/qingjian-dict-hunt/.lab; Q=$1; tag=$2; n=$3; shift 3
d=$L/work/fuse/$tag; rm -rf $d; mkdir -p $d
python3 -c "
import sys; lines=open('$Q').readlines(); n=$n; k=(len(lines)+n-1)//n
for i in range(n): open('$d/q%02d.tsv'%i,'w').writelines(lines[i*k:(i+1)*k])"
cd $L/run-clean1
for f in $d/q*.tsv; do
  $L/qj-gate2 --config /tmp/eval-config.toml "$@" --eval-text $f --eval-details $f.jsonl --misses 0 > $f.out 2>/dev/null &
done
wait
cat $d/q*.tsv.jsonl > $L/work/fuse/$tag.jsonl; wc -l < $L/work/fuse/$tag.jsonl
