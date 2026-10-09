#!/usr/bin/env python3
"""神经 P2C 小模型的验收跑：四把尺子（融合 / 生成两条路径）、回放 clean2 两半、体积与指纹。

`--panel` 一次跑三方同尺对比：旧库（不带神经模型）、含章·通变、新小模型。
参数一律用列表传给 `qingjian-cli`，不经过 shell —— `$KEYS` 那类词分割事故不再有机会发生。

用法：python3 tools/neural-train/accept.py --panel
      python3 tools/neural-train/accept.py <模型目录> [标签] [--generate]
"""

import hashlib
import json
import pathlib
import re
import subprocess
import sys

REPO = pathlib.Path("/Users/liyuqing/sproot/qingjian")
LAB = pathlib.Path("/Users/liyuqing/sproot/qingjian-neural/.lab")
CLI = REPO / "target/release/qingjian-cli"
CONFIG = LAB / "cli-config.toml"
EVAL = REPO / "data/eval"

# 四把尺子：主语料 / 对话留出 / 书面留出 / 外部（三边没见过）
RULERS = [
    ("sentences", EVAL / "sentences.tsv"),
    ("dialog", EVAL / "dialog-holdout-frozen.tsv"),
    ("prose", EVAL / "prose-holdout-frozen.tsv"),
    ("external", EVAL / "external-frozen.tsv"),
]
REPLAY = EVAL / "input-log-2026-10-04.jsonl"

# 三方同尺：旧库 = 不带神经模型（纯词图 + 统计 LM）
PANEL = [("旧库", None), ("通变", LAB / "tongbian"), ("新小模型", LAB / "full")]

SENTENCE_ROW = re.compile(r"句子\s+(\d+)\s+条\s+首选\s+([\d.]+)%\s+前三\s+([\d.]+)%\s+前五\s+([\d.]+)%\s+"
                          r"整句候选\s+([\d.]+)%\s+字准确率\s+([\d.]+)%")
GEN_ROW = re.compile(r"句子\s+(\d+)\s+条\s+首选\s+([\d.]+)%\s+前五\s+([\d.]+)%\s+字准确率\s+([\d.]+)%\s+生成失败\s+(\d+)")
TALLY_ROW = re.compile(r"^(词|整句)\s+(\d+)\s+条\s+首选\s+([\d.]+)%")
CLEAN_ROW = re.compile(r"^\s+(词|整句)\s+全部\s+(\d+)/(\d+) = ([\d.]+)%")


def run(args: list[str]) -> str:
    result = subprocess.run([str(CLI), "--config", str(CONFIG), *args],
                            cwd=REPO, capture_output=True, text=True)
    if result.returncode != 0:
        raise SystemExit(f"命令失败 {args}：{result.stderr[-400:]}")
    return result.stdout


def percentiles(values: list[float]) -> tuple[float, float]:
    """p50 / p95（最近秩法）。"""
    if not values:
        return 0.0, 0.0
    ordered = sorted(values)

    def pick(q: float) -> float:
        return ordered[min(len(ordered) - 1, int(q * len(ordered)))]

    return pick(0.5), pick(0.95)


def latencies(details: pathlib.Path, field: str) -> list[float]:
    if not details.exists():
        return []
    out = []
    for line in details.read_text(encoding="utf-8").splitlines():
        if line.strip():
            value = json.loads(line).get(field)
            if isinstance(value, (int, float)):
                out.append(float(value))
    return out


def fingerprint(label: str, model: pathlib.Path | None) -> str:
    if model is None:
        return f"== {label} ==（不带神经模型）"
    weight = model / "model.safetensors"
    if not weight.exists():
        raise SystemExit(f"{label} 没有 model.safetensors：{weight}")
    digest = hashlib.sha256(weight.read_bytes()).hexdigest()
    return f"== {label} ==  {model}\n   {weight.stat().st_size / 1048576:.2f} MiB  sha256 {digest}"


def neural_arg(model: pathlib.Path | None) -> list[str]:
    return ["--neural", str(model.resolve())] if model else []


def run_panel(model: pathlib.Path | None, generate: bool, details: pathlib.Path) -> None:
    print("\n-- 融合（词图 + P2C 打分，产品路径） --", flush=True)
    for name, path in RULERS:
        details.unlink(missing_ok=True)
        out = run(["--eval-text", str(path), *neural_arg(model), "--misses", "0",
                   "--eval-details", str(details)])
        row = SENTENCE_ROW.search(out)
        p50, p95 = percentiles(latencies(details, "query_ms"))
        body = " / ".join(row.groups()) if row else f"没解析到：{out[-200:]}"
        print(f"{name:9s} {body}  | 查询 p50 {p50:.1f} / p95 {p95:.1f} ms", flush=True)
    if generate and model:
        print("\n-- 生成（P2C 直接生成，不经词图） --", flush=True)
        for name, path in RULERS:
            details.unlink(missing_ok=True)
            out = run(["--eval-text", str(path), "--eval-generate", str(model.resolve()), "--misses", "0",
                       "--eval-details", str(details)])
            row = GEN_ROW.search(out)
            p50, p95 = percentiles(latencies(details, "generate_ms"))
            body = " / ".join(row.groups()) if row else f"没解析到：{out[-200:]}"
            print(f"{name:9s} {body}  | 生成 p50 {p50:.0f} / p95 {p95:.0f} ms", flush=True)

    print("\n-- 回放（clean2 两半） --", flush=True)
    out = run(["--replay", str(REPLAY), *neural_arg(model), "--clean2", "--misses", "0"])
    for line in out.splitlines():
        if TALLY_ROW.match(line) or CLEAN_ROW.match(line) or "干净口径" in line:
            print("   " + line.rstrip(), flush=True)


def main() -> int:
    if "--panel" in sys.argv:
        details = pathlib.Path("/tmp/accept-details.jsonl")
        for label, model in PANEL:
            print("\n" + fingerprint(label, model), flush=True)
            run_panel(model, "--generate" in sys.argv, details)
        return 0

    model = pathlib.Path(sys.argv[1]).resolve()
    print(fingerprint(sys.argv[2] if len(sys.argv) > 2 else model.name, model), flush=True)
    run_panel(model, "--generate" in sys.argv, pathlib.Path("/tmp/accept-details.jsonl"))
    return 0


if __name__ == "__main__":
    sys.exit(main())
