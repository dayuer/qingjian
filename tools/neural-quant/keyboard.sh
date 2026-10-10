#!/bin/bash
# 用法：keyboard.sh [句数]：键盘 Data 目录 + 8 位通变上跑 keyboard_probe，三个阶段各量一次 footprint，写 kbd-mem.txt
R=${QJ_REPO:-$(cd "$(dirname "$0")/../.." && pwd)}   # 脚本所在仓库（主检出）
A=${QJ_ARCHIVE:-$R/data/archive}                  # 2026-10-10 三个 worktree 的 .lab 合并迁到这里
P=$R/cloud/target/release/examples/keyboard_probe
D=$R/cloud/ios/Keyboard/Data; Q=$R/data/eval/sentences.tsv; out=$A/dict-hunt-lab/quant/kbd-mem.txt
fifo=$(mktemp -u /tmp/kbd.XXXX); mkfifo $fifo; log=$(mktemp /tmp/kbd-log.XXXX)
$P $D $Q ${1:-50} < $fifo > $log 2> $out.err & pid=$!
exec 3> $fifo; : > $out
for stage in before loaded typed; do
  until grep -qx $stage $log; do sleep 0.3; kill -0 $pid 2>/dev/null || { echo "probe 退出了"; cat $out.err; exit 1; }; done
  sleep 1; echo "== $stage" >> $out; footprint -p $pid -f bytes 2>/dev/null >> $out; echo >&3
done
exec 3>&-; wait $pid; rm -f $fifo $log
