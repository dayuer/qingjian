#!/bin/bash
# 模拟器键盘内存浸泡：跑 KeyboardMemorySoak（连打 50 句），同时每 0.5 秒量键盘扩展进程的 phys_footprint，
# 输出 sim-soak.tsv（时刻、pid、footprint 字节）与 xcodebuild 日志。用法：sim-soak.sh <模拟器 UDID> <输出目录>
set -u
udid=$1; mkdir -p "$2"; out=$(cd "$2" && pwd); : > $out/sim-soak.tsv
cd "$(dirname "$0")/../../cloud/ios"
xcodebuild test -project QingjianCloud.xcodeproj -scheme QingjianCloud -destination "id=$udid" \
  -derivedDataPath /tmp/qj-ios-dd -only-testing:QingjianCloudUITests/KeyboardMemorySoak > $out/xcodebuild.log 2>&1 &
test_pid=$!
while kill -0 $test_pid 2>/dev/null; do
  pid=$(pgrep -f "$udid.*/Keyboard.appex/Keyboard" | head -1)
  if [ -n "$pid" ]; then
    fp=$(footprint -p $pid -f bytes 2>/dev/null | awk '/phys_footprint:/{print $2}')
    [ -n "$fp" ] && echo -e "$(date +%s.%N)\t$pid\t$fp" >> $out/sim-soak.tsv
  fi
  sleep 0.5
done
wait $test_pid; echo "test exit $?" >> $out/xcodebuild.log
