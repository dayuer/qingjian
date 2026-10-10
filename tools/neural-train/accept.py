#!/usr/bin/env python3
"""神经 P2C 小模型的验收跑：四把尺子（融合 / 生成两条路径）、回放 clean2 两半、体积与指纹。

融合与回放都靠词图，所以每个词图各跑一遍三方同尺；生成不经词图，只跑一遍。
每一节开头打印**实际加载**的 dict 与 lm（从 CLI 日志里读回来，再自己算 sha256 与体积）——
不打印出来的一律不算数：2026-10-08 就因为 CLI 相对路径读到了被短语合成撑大的 202MB `lm.qj`，
「旧库」整行作废过一次。

用法：python3 tools/neural-train/accept.py --panel [--generate]
      python3 tools/neural-train/accept.py <模型目录> [标签] [--generate]
"""

import hashlib
import json
import os
import pathlib
import re
import subprocess
import sys

# 2026-10-10：三个 worktree 的 .lab 合并迁到主检出 data/archive/，脚本不再指 worktree 路径。
# 默认按**脚本所在仓库**（主检出）解析；在 worktree 里跑时用 QJ_REPO 指主检出。
REPO = pathlib.Path(os.environ.get("QJ_REPO") or pathlib.Path(__file__).resolve().parents[2])
ARCHIVE = pathlib.Path(os.environ.get("QJ_ARCHIVE") or REPO / "data/archive")
LAB = ARCHIVE / "neural-lab"            # 训练产物、模型、venv、验收日志
LEXICON_LAB = ARCHIVE / "lexicon-lab"   # 词库线的中间产物（prose-holdout-clean.tsv、base-cap100…）
CLI = pathlib.Path(os.environ.get("QJ_CLI") or REPO / "target/release/qingjian-cli")
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

# 三方同尺：新小模型用 `best/`（dev 首选创新高时另存的那份）
PANEL = [("新小模型", LAB / "full/best"), ("旧库", None), ("通变", LAB / "tongbian")]

# 两套词图（绝对路径，不靠 cwd）：旧库 = 随包发出去的那份；产品底座 = 500 万二元上限重建的
GRAPHS = [
    ("旧库词图", REPO / "data/generated.shipped/dict.qj", REPO / "data/generated.shipped/lm.qj"),
    ("产品底座", LEXICON_LAB / "base-cap100/dict.qj", LEXICON_LAB / "base-cap100/lm.qj"),
]

ANSI = re.compile(r"\x1b\[[0-9;]*m")
SENTENCE_ROW = re.compile(r"句子\s+(\d+)\s+条\s+首选\s+([\d.]+)%\s+前三\s+([\d.]+)%\s+前五\s+([\d.]+)%\s+"
                          r"整句候选\s+([\d.]+)%\s+字准确率\s+([\d.]+)%")
GEN_ROW = re.compile(r"句子\s+(\d+)\s+条\s+首选\s+([\d.]+)%\s+前五\s+([\d.]+)%\s+字准确率\s+([\d.]+)%\s+生成失败\s+(\d+)")
TALLY_ROW = re.compile(r"^(词|整句)\s+(\d+)\s+条\s+首选\s+([\d.]+)%")
CLEAN_ROW = re.compile(r"^\s+(词|整句)\s+全部\s+(\d+)/(\d+) = ([\d.]+)%")


def digest_of(path: pathlib.Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def describe(name: str, path: pathlib.Path) -> str:
    if not path.exists():
        return f"{name}=**缺失** {path}"
    return f"{name}={path.name} {path.stat().st_size / 1048576:.2f} MiB sha256 {digest_of(path)[:8]}"


def run(args: list[str], graph: tuple[pathlib.Path, pathlib.Path] | None = None) -> tuple[str, str]:
    """跑一条 CLI 命令，返回 (stdout, stderr)。词图用绝对路径显式指定。"""
    full = [str(CLI), "--config", str(CONFIG.resolve())]
    if graph:
        full += ["--dict", str(graph[0].resolve()), "--language-model", str(graph[1].resolve())]
    result = subprocess.run(full + args, cwd=REPO, capture_output=True, text=True, timeout=7200)
    if result.returncode != 0:
        raise SystemExit(f"命令失败 {args}：{result.stderr[-400:]}")
    return result.stdout, result.stderr


def loaded(stderr: str) -> str:
    """从 CLI 日志里读回它**实际**加载的 dict 与 lm，自己再算指纹。"""
    plain = ANSI.sub("", stderr)
    dict_path = next((m.group(1) for line in plain.splitlines()
                      if (m := re.search(r"\bdict=(\S+)", line))), "")
    lm_path = next((m.group(1) for line in plain.splitlines()
                    if "语言模型已加载" in line and (m := re.search(r"path=(\S+)", line))), "")
    parts = []
    for name, text in (("dict", dict_path), ("lm", lm_path)):
        path = pathlib.Path(text)
        parts.append(describe(name, path) if path.exists() else f"{name}=**没打印或不存在**（{text or '无'}）")
    return "   " + "  ".join(parts)


def percentiles(values: list[float]) -> tuple[float, float]:
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
    return (f"== {label} ==  {model}\n   {weight.stat().st_size / 1048576:.2f} MiB  "
            f"sha256 {digest_of(weight)}")


def neural_arg(model: pathlib.Path | None) -> list[str]:
    return ["--neural", str(model.resolve())] if model else []


def fusion_rows(model: pathlib.Path | None, graph: tuple[pathlib.Path, pathlib.Path],
                details: pathlib.Path) -> None:
    for name, path in RULERS:
        details.unlink(missing_ok=True)
        out, err = run(["--eval-text", str(path), *neural_arg(model), "--misses", "0",
                        "--eval-details", str(details)], graph)
        row = SENTENCE_ROW.search(out)
        p50, p95 = percentiles(latencies(details, "query_ms"))
        body = " / ".join(row.groups()) if row else f"没解析到：{out[-200:]}"
        print(f"   {name:9s} {body}  | 查询 p50 {p50:.1f} / p95 {p95:.1f} ms", flush=True)
        print(loaded(err), flush=True)


def replay_rows(model: pathlib.Path | None, graph: tuple[pathlib.Path, pathlib.Path]) -> None:
    out, err = run(["--replay", str(REPLAY), *neural_arg(model), "--clean2", "--misses", "0"], graph)
    for line in out.splitlines():
        if TALLY_ROW.match(line) or CLEAN_ROW.match(line) or "干净口径" in line:
            print("   " + line.rstrip(), flush=True)
    print(loaded(err), flush=True)


def generate_rows(model: pathlib.Path, details: pathlib.Path) -> None:
    """生成不经词图，词图参数传任意一套只为让引擎起得来。"""
    for name, path in RULERS:
        details.unlink(missing_ok=True)
        out, _ = run(["--eval-text", str(path), "--eval-generate", str(model.resolve()), "--misses", "0",
                      "--eval-details", str(details)], GRAPHS[0][1:])
        row = GEN_ROW.search(out)
        p50, p95 = percentiles(latencies(details, "generate_ms"))
        body = " / ".join(row.groups()) if row else f"没解析到：{out[-200:]}"
        print(f"   {name:9s} {body}  | 生成 p50 {p50:.0f} / p95 {p95:.0f} ms", flush=True)


def main() -> int:
    details = pathlib.Path("/tmp/accept-details.jsonl")
    if "--panel" in sys.argv:
        for label, model in PANEL:
            print("\n" + fingerprint(label, model), flush=True)
        for graph_label, dict_path, lm_path in GRAPHS:
            print(f"\n########## 词图：{graph_label} ##########", flush=True)
            print("   " + describe("dict", dict_path) + "  " + describe("lm", lm_path), flush=True)
            for label, model in PANEL:
                print(f"\n-- {label} · 融合（词图 + P2C 打分） --", flush=True)
                fusion_rows(model, (dict_path, lm_path), details)
                print(f"-- {label} · 回放 clean2 --", flush=True)
                replay_rows(model, (dict_path, lm_path))
        if "--generate" in sys.argv:
            print("\n########## 生成（P2C 直接生成，不经词图） ##########", flush=True)
            for label, model in PANEL:
                if model:
                    print(f"\n-- {label} --", flush=True)
                    generate_rows(model, details)
        return 0

    model = pathlib.Path(sys.argv[1]).resolve()
    print(fingerprint(sys.argv[2] if len(sys.argv) > 2 else model.name, model), flush=True)
    for graph_label, dict_path, lm_path in GRAPHS:
        print(f"\n-- 融合 · {graph_label} --", flush=True)
        fusion_rows(model, (dict_path, lm_path), details)
    if "--generate" in sys.argv:
        generate_rows(model, details)
    return 0


if __name__ == "__main__":
    sys.exit(main())
