#!/bin/bash
# 用法：memory.sh <模型>：三个阶段各量一次 footprint（-f bytes），输出写 mem-<文件名>.txt（macOS 自带 bash 3.2 没有 coproc，用 FIFO）
P=/Users/liyuqing/sproot/qingjian-dict-hunt/target/release/examples/load_probe; m=$1; out=mem-$(basename $m).txt
fifo=$(mktemp -u /tmp/probe.XXXX); mkfifo $fifo; log=$(mktemp /tmp/probe-log.XXXX)
$P $m < $fifo > $log 2>/dev/null & pid=$!
exec 3> $fifo
: > $out
for stage in before loaded generated; do
  until grep -qx $stage $log; do sleep 0.2; done
  sleep 1
  echo "== $stage" >> $out
  footprint -p $pid -f bytes 2>/dev/null >> $out
  echo >&3
done
exec 3>&-; wait $pid; rm -f $fifo $log
