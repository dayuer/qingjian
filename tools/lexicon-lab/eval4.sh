#!/bin/bash
# 用法：eval4.sh <库目录> <标签>：四把尺子（外部 / sentences / 对话留出 / 书面留出）+ 回放 clean2，结果追加到 .lab/bm/results.tsv
set -uo pipefail
W=/Users/liyuqing/sproot/qingjian-dict-hunt; B=$W/target/release/qingjian-cli; E=/Users/liyuqing/sproot/qingjian/data/eval
lib=$(cd "$1" && pwd); tag=$2; R=$W/.lab/bm/run-$tag; O=$W/.lab/bm/out-$tag
rm -rf $R $O; mkdir -p $R/data $O
ln -s $lib $R/data/generated; ln -s $E $R/data/eval; ln -s $W/assets $R/assets; ln -s $W/apps $R/apps
cd $R
for s in external-frozen sentences dialog-holdout-frozen prose-holdout-frozen; do
  $B --config /tmp/eval-config.toml ${EXTRA:-} --eval-text data/eval/$s.tsv --misses 0 --eval-details $O/ed-$s.jsonl > $O/eval-$s.txt 2>/dev/null &
done
$B --config /tmp/eval-config.toml ${EXTRA:-} --replay data/eval/input-log-2026-10-04.jsonl --clean2 --misses 0 --replay-details $O/rd.jsonl > $O/replay.txt 2>/dev/null &
wait
t1() { grep -m1 "^句子" $O/eval-$1.txt | sed -E 's/.*首选 +([0-9.]+)%.*/\1/'; }
w=$(sed -n '/干净口径 2/,/整句 /p' $O/replay.txt | grep -m1 "^  词" | sed -E 's/.*全部 ([0-9]+)\/[0-9]+.*前半 ([0-9]+)\/[0-9]+.*后半 ([0-9]+)\/[0-9]+.*/\1|\2|\3/')
s=$(sed -n '/干净口径 2/,/整句 /p' $O/replay.txt | grep -m1 "^  整句" | sed -E 's/.*全部 ([0-9]+)\/[0-9]+.*前半 ([0-9]+)\/[0-9]+.*后半 ([0-9]+)\/[0-9]+.*/\1|\2|\3/')
fp() { [ -f $lib/$1 ] && echo "$(shasum -a 256 $lib/$1 | cut -c1-8)/$(( $(stat -f %z $lib/$1) / 1048576 ))M" || echo -; }
line="$tag	$(t1 external-frozen)	$(t1 sentences)	$(t1 dialog-holdout-frozen)	$(t1 prose-holdout-frozen)	$w	$s	dict=$(fp dict.qj)	lm=$(fp lm.qj)	$(cd $W && git rev-parse --short HEAD)"
echo "$line" | tee -a $W/.lab/bm/results.tsv
