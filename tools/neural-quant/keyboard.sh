#!/bin/bash
# 用法：keyboard.sh [句数]：键盘 Data 目录 + 8 位通变上跑 keyboard_probe，三个阶段各量一次 footprint，写 kbd-mem.txt
W=/Users/liyuqing/sproot/qingjian-dict-hunt; P=$W/cloud/target/release/examples/keyboard_probe
D=$W/cloud/ios/Keyboard/Data; Q=/Users/liyuqing/sproot/qingjian/data/eval/sentences.tsv; out=$W/.lab/quant/kbd-mem.txt
fifo=$(mktemp -u /tmp/kbd.XXXX); mkfifo $fifo; log=$(mktemp /tmp/kbd-log.XXXX)
$P $D $Q ${1:-50} < $fifo > $log 2> $out.err & pid=$!
exec 3> $fifo; : > $out
for stage in before loaded typed; do
  until grep -qx $stage $log; do sleep 0.3; kill -0 $pid 2>/dev/null || { echo "probe 退出了"; cat $out.err; exit 1; }; done
  sleep 1; echo "== $stage" >> $out; footprint -p $pid -f bytes 2>/dev/null >> $out; echo >&3
done
exec 3>&-; wait $pid; rm -f $fifo $log
