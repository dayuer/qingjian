#!/usr/bin/env bash
# 词库/语言模型的评测门槛：跑两条尺子（整句评测 + 回放），都在容差内就写 data/generated/GATE_PASSED。
#
# 装机脚本（apps/macos/scripts/bundle.sh、cloud/ios/scripts/build-bridge.sh）看到 GATE_PASSED 才用
# data/generated 那份数据，否则回退到上次发版的 data/generated.shipped —— 没过门槛的库装不上去。
# 基线数字与容差见 docs/plan/dictionary-layering.md 第 5 节（改动基线要在同一个提交里改这里与那一页）。
#
# 用法：tools/release/gate-pass.sh        （仓库根目录跑；需要 data/eval/ 的两份评测数据）
set -euo pipefail

cd "$(dirname "$0")/../.."

SENTENCES=data/eval/sentences.tsv
LOG=data/eval/input-log-2026-10-04.jsonl
CONFIG=tools/eval/offline.toml
EXTRA=data/generated/dicts/idioms.qj

for f in "$SENTENCES" "$LOG"; do
  [[ -f "$f" ]] || { echo "缺评测数据 $f（本机 data/eval/ 下，不进 git）" >&2; exit 1; }
done

echo "=== 整句评测 ==="
cargo run --quiet --release -p qingjian-cli -- --config "$CONFIG" \
  --eval-text "$SENTENCES" --extra-dict "$EXTRA" 2>/dev/null | tee /tmp/gate-eval.txt | grep -E "^句子"
echo "=== 回放 ==="
cargo run --quiet --release -p qingjian-cli -- --config "$CONFIG" \
  --replay "$LOG" --extra-dict "$EXTRA" --misses 0 2>/dev/null | tee /tmp/gate-replay.txt | grep -E "^词|^整句|^英文"

python3 - "$@" <<'PY'
"""比对基线：首选 / 前三（回放的整句是前五）各允许 −0.3 个点；字准确率也按 −0.3。"""
import pathlib
import re
import subprocess
import sys
from datetime import datetime

TOLERANCE = 0.3
BASELINE = {
    "评测首选": 34.2, "评测前三": 39.9, "评测字准": 77.5,
    "回放词首选": 88.6, "回放词前五": 97.6, "回放整句首选": 62.4,
}

def percent(text, pattern):
    m = re.search(pattern, text, re.M)
    if not m:
        raise SystemExit(f"读不出指标，模式 {pattern!r} 没匹配上")
    return float(m.group(1))

eval_text = pathlib.Path("/tmp/gate-eval.txt").read_text(encoding="utf-8")
replay_text = pathlib.Path("/tmp/gate-replay.txt").read_text(encoding="utf-8")
current = {
    "评测首选": percent(eval_text, r"首选\s+([\d.]+)%"),
    "评测前三": percent(eval_text, r"前三\s+([\d.]+)%"),
    "评测字准": percent(eval_text, r"字准确率\s+([\d.]+)%"),
    "回放词首选": percent(replay_text, r"^词\s+\d+ 条\s+首选\s+([\d.]+)%"),
    "回放词前五": percent(replay_text, r"^词\s+\d+ 条\s+首选\s+[\d.]+%\s+前五\s+([\d.]+)%"),
    "回放整句首选": percent(replay_text, r"^整句\s+\d+ 条\s+首选\s+([\d.]+)%"),
}

failed = []
print("\n指标        基线    现在    差")
for name, baseline in BASELINE.items():
    now = current[name]
    delta = round(now - baseline, 1)
    flag = "" if delta >= -TOLERANCE else "  ← 超容差"
    if delta < -TOLERANCE:
        failed.append(name)
    print(f"{name:<12}{baseline:>6.1f}%{now:>8.1f}%{delta:>+7.1f}{flag}")

if failed:
    print(f"\n没过门槛：{ '、'.join(failed) }（容差 −{TOLERANCE} 个点）—— 不写 GATE_PASSED，装机脚本会回退用 data/generated.shipped", file=sys.stderr)
    raise SystemExit(1)

marker = pathlib.Path("data/generated/GATE_PASSED")
marker.write_text(
    f"# 词库/语言模型过了评测门槛（tools/release/gate-pass.sh）\n"
    f"# {datetime.now().isoformat(timespec='seconds')}\n"
    + "".join(f"# {name} {BASELINE[name]}% → {current[name]}%（{current[name] - BASELINE[name]:+.1f}）\n" for name in BASELINE),
    encoding="utf-8",
)
print(f"\n门槛通过，已写 {marker}")
PY
