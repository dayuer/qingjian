#!/bin/bash
# 全量训练的外层：防系统睡眠 + 非 0 退出自动 --resume 重开（最多 3 次）。
# 日志一律**追加**，绝不覆盖；训练自己写了 `done` 才算完成。
#
# 用法：bash tools/neural-train/run_full.sh

set -u
R=${QJ_REPO:-$(cd "$(dirname "$0")/../.." && pwd)}   # 脚本所在仓库（主检出）
A=${QJ_ARCHIVE:-$R/data/archive}                  # 2026-10-10 三个 worktree 的 .lab 合并迁到这里
LAB=$A/neural-lab
LOG=$LAB/full.log
EVAL=$R/data/eval

for attempt in 1 2 3 4; do
    echo "=== 第 $attempt 次启动 $(date '+%m-%d %H:%M') ===" >> "$LOG"
    caffeinate -i -s "$LAB/venv/bin/python" tools/neural-train/train_p2c.py \
        --epochs 2 --resume --out "$LAB/full" \
        --dev "$EVAL/dialog-dev.tsv" "$EVAL/prose-dev.tsv" >> "$LOG" 2>&1
    code=$?
    if [ -f "$LAB/full/done" ]; then
        echo "=== 训练完成 $(date '+%m-%d %H:%M') ===" >> "$LOG"
        exit 0
    fi
    echo "=== 第 $attempt 次退出码 $code，60 秒后 --resume 重开 $(date '+%m-%d %H:%M') ===" >> "$LOG"
    sleep 60
done
echo "=== 三次重启都没跑完，停手等人工看 ===" >> "$LOG"
exit 1
