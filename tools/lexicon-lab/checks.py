#!/usr/bin/env python3
"""词库线的四道自动检查（用户定的规矩：出错改机制，不只改这一次）。构建/训练前跑，不过就退出。

1. `corpus`：语料统计自检 —— 拿一条已知答案比对（LCCC 去重后最高频是「哈哈」41,078 次）。
   2026-10-08 那次「前 20 句占 85.8%」的假数字就是 shell 工具在 CJK 上算错造成的，这条拦住同类。
2. `data`：训练集 / 开发集 / 留出集 / LM 语料两两无交集。那次「造题句在 LM 语料里」让 dev 涨、
   留出跌，这条在训练前就把这种数据泄漏顶回去。
3. `build`：建库产物门槛 —— 二元条数 ≤ 上限、lm.qj ≤ 50 MB（iOS 键盘内存预算）。
   那次 1568 万条 / 123 MB 就是漏了这道门。
4. `fingerprint`：给一组产物算 sha256 前 8 位与体积，报告里必须原样带上（没有指纹的数字不报）。

用法：
  python3 tools/lexicon-lab/checks.py corpus --db /tmp/lccc-counts.sqlite
  python3 tools/lexicon-lab/checks.py data --train a.tsv --dev b.tsv --holdout c.txt --lm-corpus d.txt
  python3 tools/lexicon-lab/checks.py build --lm-qj x/lm.qj --lm-bigram x/lm-bigram.tsv --limit 5000000
  python3 tools/lexicon-lab/checks.py fingerprint x/lm.qj x/dict.qj
"""

import argparse
import hashlib
import pathlib
import sqlite3
import sys

LM_QJ_MAX_MB = 50
KNOWN_LCCC_TOP = ("哈哈", 41078)


def check_corpus(args) -> int:
    db = sqlite3.connect(f"file:{args.db}?mode=ro", uri=True)
    row = db.execute("SELECT sample, count FROM counts ORDER BY count DESC, key LIMIT 1").fetchone()
    if row is None:
        print("语料自检失败：计数库里没有数据", file=sys.stderr)
        return 1
    sample, count = row
    # 允许 ±0.5% 的抖动（语料的切分与去重会随代码小改而微动；shell 那次是差 100 倍）
    tolerance = max(50, KNOWN_LCCC_TOP[1] // 200)
    if sample != KNOWN_LCCC_TOP[0] or abs(count - KNOWN_LCCC_TOP[1]) > tolerance:
        print(
            f"语料自检失败：LCCC 去重后最高频应为 {KNOWN_LCCC_TOP[0]} {KNOWN_LCCC_TOP[1]} 次，"
            f"实际 {sample} {count} 次 —— 统计管线动过了，先查清楚再往下走",
            file=sys.stderr,
        )
        return 1
    print(f"语料自检通过：最高频 {sample} {count} 次 ✓")
    return 0


def load_texts(path: pathlib.Path, strip_space: bool) -> set[str]:
    out = set()
    for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
        if not line.strip() or line.startswith("#"):
            continue
        text = line.split("\t")[0].strip()
        if strip_space:
            text = text.replace(" ", "")
        out.add(text)
    return out


def check_data(args) -> int:
    sets = {}
    for name, path, strip in (
        ("训练集", args.train, True),
        ("开发集", args.dev, True),
        ("留出集", args.holdout, True),
        ("LM 语料", args.lm_corpus, False),
    ):
        if path is None:
            continue
        path = pathlib.Path(path)
        sets[name] = load_texts(path, strip)
        print(f"{name}: {len(sets[name])} 条（{path}）")
    names = list(sets)
    bad = False
    for i in range(len(names)):
        for j in range(i + 1, len(names)):
            overlap = sets[names[i]] & sets[names[j]]
            if overlap:
                bad = True
                samples = list(overlap)[:5]
                print(
                    f"数据集交集断言失败：{names[i]} ∩ {names[j]} = {len(overlap)} 条，"
                    f"例如 {samples}",
                    file=sys.stderr,
                )
    if bad:
        return 1
    print("数据集两两无交集 ✓")
    return 0


def check_build(args) -> int:
    ok = True
    bigram = pathlib.Path(args.lm_bigram)
    if bigram.exists():
        rows = sum(1 for line in bigram.open(encoding="utf-8") if line.strip() and not line.startswith("#"))
        if rows > args.limit:
            print(f"建库门槛失败：二元 {rows} 条 > 上限 {args.limit}", file=sys.stderr)
            ok = False
        else:
            print(f"二元 {rows} 条 ≤ 上限 {args.limit} ✓")
    lm_qj = pathlib.Path(args.lm_qj)
    if lm_qj.exists():
        size_mb = lm_qj.stat().st_size / 1048576
        if size_mb > LM_QJ_MAX_MB:
            print(f"建库门槛失败：lm.qj {size_mb:.0f} MB > {LM_QJ_MAX_MB} MB", file=sys.stderr)
            ok = False
        else:
            print(f"lm.qj {size_mb:.0f} MB ≤ {LM_QJ_MAX_MB} MB ✓")
    return 0 if ok else 1


def check_fingerprint(args) -> int:
    for path in args.paths:
        path = pathlib.Path(path)
        if not path.exists():
            print(f"{path} 不存在", file=sys.stderr)
            continue
        digest = hashlib.sha256(path.read_bytes()).hexdigest()[:8]
        size_mb = path.stat().st_size / 1048576
        print(f"{digest}  {size_mb:6.1f} MB  {path}")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    sub = parser.add_subparsers(dest="command", required=True)

    corpus = sub.add_parser("corpus", help="语料统计自检")
    corpus.add_argument("--db", required=True)
    corpus.set_defaults(func=check_corpus)

    data = sub.add_parser("data", help="数据集交集断言")
    data.add_argument("--train")
    data.add_argument("--dev")
    data.add_argument("--holdout")
    data.add_argument("--lm-corpus")
    data.set_defaults(func=check_data)

    build = sub.add_parser("build", help="建库产物门槛")
    build.add_argument("--lm-qj", required=True)
    build.add_argument("--lm-bigram", required=True)
    build.add_argument("--limit", type=int, default=5_000_000)
    build.set_defaults(func=check_build)

    fingerprint = sub.add_parser("fingerprint", help="产物指纹")
    fingerprint.add_argument("paths", nargs="+")
    fingerprint.set_defaults(func=check_fingerprint)

    args = parser.parse_args()
    return args.func(args)


if __name__ == "__main__":
    raise SystemExit(main())
