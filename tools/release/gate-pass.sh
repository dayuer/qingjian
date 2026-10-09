#!/usr/bin/env bash
# 词库/语言模型的评测门槛：整句评测与 clean2 回放，不带模型、融合（含章·通变）两种配置各跑一遍，都在容差内就写 data/generated/GATE_PASSED。
# 回放用 clean2 口径（剔掉用户事后撤销、删掉重打的上屏）：原始回放里那些行的「答案」本身就是错的，不该当门槛。
#
# 装机脚本（apps/macos/scripts/bundle.sh、cloud/ios/scripts/build-bridge.sh）看到 GATE_PASSED 才用
# data/generated 那份数据，否则回退到上次发版的 data/generated.shipped —— 没过门槛的库装不上去。
# 基线数字与容差见 docs/plan/dictionary-layering.md 第 5 节（改动基线要在同一个提交里改这里与那一页）。
#
# 用法：tools/release/gate-pass.sh        （仓库根目录跑；需要 data/eval/ 的两份评测数据与通变模型）
set -euo pipefail

cd "$(dirname "$0")/../.."

SENTENCES=data/eval/sentences.tsv
LOG=data/eval/input-log-2026-10-04.jsonl
CONFIG=tools/eval/offline.toml
EXTRA=data/generated/dicts/idioms.qj
# 与 bundle.sh 同一个位置找通变
MODEL="${QINGJIAN_P2C_MODEL_DIR:-data/models/hanzhang-tongbian}/hanzhang-tongbian-small.qjm"

for f in "$SENTENCES" "$LOG"; do
  [[ -f "$f" ]] || { echo "缺评测数据 $f（本机 data/eval/ 下，不进 git）" >&2; exit 1; }
done
[[ -f "$MODEL" ]] || { echo "缺通变模型 $MODEL（融合那一遍要用；可用 QINGJIAN_P2C_MODEL_DIR 指目录）" >&2; exit 1; }

cargo build --quiet --release -p qingjian-cli
CLI=target/release/qingjian-cli
for setup in none fusion; do
  # macOS 的 bash 3.2 在 set -u 下展开空数组会报错，所以下面用 ${neural[@]+…}
  neural=()
  [[ $setup == fusion ]] && neural=(--neural "$MODEL")
  echo "=== 整句评测（$setup）==="
  $CLI --config "$CONFIG" ${neural[@]+"${neural[@]}"} --eval-text "$SENTENCES" --extra-dict "$EXTRA" 2>/dev/null \
    | tee "/tmp/gate-eval-$setup.txt" | grep -E "^句子"
  echo "=== clean2 回放（$setup）==="
  $CLI --config "$CONFIG" ${neural[@]+"${neural[@]}"} --replay "$LOG" --clean2 --extra-dict "$EXTRA" --misses 0 2>/dev/null \
    | sed -n '/干净口径 2/,/整句 /p' | tee "/tmp/gate-replay-$setup.txt" | grep -E "词|整句"
done

python3 - "$@" <<'PY'
"""比对基线：每个指标各允许 −0.3 个点。clean2 整句一共 138 条、一条 0.72 个点，等于整句一条都不许掉。"""
import pathlib
import re
import subprocess
import sys
from datetime import datetime

TOLERANCE = 0.3
# sujian 361214b + data-v3（dict 7fafe1b8、lm f9fb7b44）+ 通变 8 位（2c326abf）量的，见 docs/plan/dictionary-layering.md 5.1
BASELINE = {
    "none": {"评测首选": 34.2, "评测前三": 39.9, "评测字准": 77.5, "clean2词首选": 89.9, "clean2整句首选": 69.6},
    "fusion": {"评测首选": 38.8, "评测前三": 57.7, "评测字准": 79.9, "clean2词首选": 89.8, "clean2整句首选": 74.6},
}

def percent(text, pattern):
    m = re.search(pattern, text, re.M)
    if not m:
        raise SystemExit(f"读不出指标，模式 {pattern!r} 没匹配上")
    return float(m.group(1))

def measure(setup):
    eval_text = pathlib.Path(f"/tmp/gate-eval-{setup}.txt").read_text(encoding="utf-8")
    replay_text = pathlib.Path(f"/tmp/gate-replay-{setup}.txt").read_text(encoding="utf-8")
    return {
        "评测首选": percent(eval_text, r"首选\s+([\d.]+)%"),
        "评测前三": percent(eval_text, r"前三\s+([\d.]+)%"),
        "评测字准": percent(eval_text, r"字准确率\s+([\d.]+)%"),
        "clean2词首选": percent(replay_text, r"^\s*词\s+全部 \d+/\d+ = ([\d.]+)%"),
        "clean2整句首选": percent(replay_text, r"^\s*整句\s+全部 \d+/\d+ = ([\d.]+)%"),
    }

current = {setup: measure(setup) for setup in BASELINE}
failed = []
print("\n配置    指标            基线    现在    差")
for setup, metrics in BASELINE.items():
    for name, baseline in metrics.items():
        now = current[setup][name]
        delta = round(now - baseline, 1)
        flag = "" if delta >= -TOLERANCE else "  ← 超容差"
        if delta < -TOLERANCE:
            failed.append(f"{setup} {name}")
        print(f"{setup:<8}{name:<14}{baseline:>6.1f}%{now:>8.1f}%{delta:>+7.1f}{flag}")

if failed:
    print(f"\n没过门槛：{ '、'.join(failed) }（容差 −{TOLERANCE} 个点）—— 不写 GATE_PASSED，装机脚本会回退用 data/generated.shipped", file=sys.stderr)
    raise SystemExit(1)

marker = pathlib.Path("data/generated/GATE_PASSED")
marker.write_text(
    f"# 词库/语言模型过了评测门槛（tools/release/gate-pass.sh）\n"
    f"# {datetime.now().isoformat(timespec='seconds')}\n"
    + "".join(
        f"# {setup} {name} {baseline}% → {current[setup][name]}%（{current[setup][name] - baseline:+.1f}）\n"
        for setup, metrics in BASELINE.items()
        for name, baseline in metrics.items()
    ),
    encoding="utf-8",
)
print(f"\n门槛通过，已写 {marker}")
PY
