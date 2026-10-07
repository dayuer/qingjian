#!/usr/bin/env python3
"""按语域合成三套语言模型与词频：对话、书面、混合。

为什么要分：一套频率伺候不了两种语域 —— 对话加权，回放（真人打字）涨、整句评测（文档口吻）降，
反过来也一样。引擎侧按左侧上下文的语域信号在两套之间插值（见 docs/notes/eval-data.md）。

三套的构成：

| 套 | 一元 | 二元 |
|---|---|---|
| dialog | 对话语料的分词计数 | 对话的二元，按计数取前 `--dialog-bigrams` |
| prose | 书面语料的分词计数 | 书面的二元，按计数取前 `--prose-bigrams` |
| mixed | 对话 ×`--dialog-weight` + 书面 ×1 | 上面两套按语域分配额各取前 N 再合并（**不按全局计数截**，否则维基的二元会把「餐馆→吃」这类口语二元挤出表） |

用法：
  python3 tools/corpus/build_lm_sets.py --dialog /tmp/lm-dialog --prose /tmp/lm-prose \\
      --out-dir data/sets --dialog-weight 5 --dialog-bigrams 2000000 --prose-bigrams 3500000
"""

import argparse
import pathlib
import sys


def rows(path: pathlib.Path):
    with path.open(encoding="utf-8") as fh:
        for line in fh:
            if line.startswith("#") or not line.strip():
                continue
            yield line.rstrip("\n")


def combine_unigram(sources, out: pathlib.Path) -> int:
    """把几份一元表按权重相加（按词）。"""
    totals: dict[str, int] = {}
    for path, weight in sources:
        for line in rows(path):
            fields = line.split("\t")
            if len(fields) < 2:
                continue
            try:
                count = int(fields[1])
            except ValueError:
                continue
            totals[fields[0]] = totals.get(fields[0], 0) + count * weight
    with out.open("w", encoding="utf-8") as fh:
        fh.write("# 词\t计数；<s> 是句首标记\n")
        for word, count in sorted(totals.items(), key=lambda kv: (-kv[1], kv[0])):
            fh.write(f"{word}\t{count}\n")
    return len(totals)


def copy_unigram(source: pathlib.Path, out: pathlib.Path) -> int:
    count = 0
    with source.open(encoding="utf-8") as src, out.open("w", encoding="utf-8") as fh:
        fh.write("# 词\t计数；<s> 是句首标记\n")
        for line in src:
            if line.startswith("#") or not line.strip():
                continue
            fh.write(line)
            count += 1
    return count


def top_bigrams(source: pathlib.Path, out: pathlib.Path, limit: int) -> int:
    """按计数取前 N（源文件已经是按计数降序写的，直接截）。"""
    count = 0
    with source.open(encoding="utf-8") as src, out.open("w", encoding="utf-8") as fh:
        fh.write("# 前词\t后词\t计数\n")
        for line in src:
            if line.startswith("#") or not line.strip():
                continue
            if count >= limit:
                break
            fh.write(line)
            count += 1
    return count


def merge_bigrams(dialog: pathlib.Path, prose: pathlib.Path, dialog_limit: int, prose_limit: int, out: pathlib.Path) -> int:
    """两套各取前 N 再合并（同一个二元两边都有时取较大的计数）。"""
    merged: dict[tuple[str, str], int] = {}
    for path, limit in ((dialog, dialog_limit), (prose, prose_limit)):
        taken = 0
        for line in rows(path):
            fields = line.split("\t")
            if len(fields) < 3 or taken >= limit:
                break
            try:
                count = int(fields[2])
            except ValueError:
                continue
            key = (fields[0], fields[1])
            if count > merged.get(key, 0):
                merged[key] = count
            taken += 1
    with out.open("w", encoding="utf-8") as fh:
        fh.write("# 前词\t后词\t计数\n")
        for (first, second), count in sorted(merged.items(), key=lambda kv: (-kv[1], kv[0])):
            fh.write(f"{first}\t{second}\t{count}\n")
    return len(merged)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--dialog", type=pathlib.Path, required=True, help="对话侧的 bigram 输出目录")
    parser.add_argument("--prose", type=pathlib.Path, required=True, help="书面侧的 bigram 输出目录")
    parser.add_argument("--out-dir", type=pathlib.Path, default=pathlib.Path("data/sets"))
    parser.add_argument("--dialog-weight", type=int, default=5, help="混合套里对话一元的权重")
    parser.add_argument("--dialog-bigrams", type=int, default=2_000_000)
    parser.add_argument("--prose-bigrams", type=int, default=3_500_000)
    args = parser.parse_args()

    dialog_unigram = args.dialog / "lm-unigram.tsv"
    prose_unigram = args.prose / "lm-unigram.tsv"
    dialog_bigram = args.dialog / "lm-bigram.tsv"
    prose_bigram = args.prose / "lm-bigram.tsv"
    for path in (dialog_unigram, prose_unigram, dialog_bigram, prose_bigram):
        if not path.exists():
            raise SystemExit(f"缺 {path}：先跑 tools/corpus/register_lms.sh")

    for name in ("dialog", "prose", "mixed"):
        (args.out_dir / name).mkdir(parents=True, exist_ok=True)

    n = copy_unigram(dialog_unigram, args.out_dir / "dialog" / "lm-unigram.tsv")
    print(f"dialog 一元 {n} 个词", file=sys.stderr)
    n = copy_unigram(prose_unigram, args.out_dir / "prose" / "lm-unigram.tsv")
    print(f"prose 一元 {n} 个词", file=sys.stderr)
    n = combine_unigram(
        [(dialog_unigram, args.dialog_weight), (prose_unigram, 1)],
        args.out_dir / "mixed" / "lm-unigram.tsv",
    )
    print(f"mixed 一元 {n} 个词（对话 ×{args.dialog_weight} + 书面 ×1）", file=sys.stderr)

    n = top_bigrams(dialog_bigram, args.out_dir / "dialog" / "lm-bigram.tsv", args.dialog_bigrams)
    print(f"dialog 二元 {n} 条", file=sys.stderr)
    n = top_bigrams(prose_bigram, args.out_dir / "prose" / "lm-bigram.tsv", args.prose_bigrams)
    print(f"prose 二元 {n} 条", file=sys.stderr)
    n = merge_bigrams(
        dialog_bigram, prose_bigram, args.dialog_bigrams, args.prose_bigrams,
        args.out_dir / "mixed" / "lm-bigram.tsv",
    )
    print(f"mixed 二元 {n} 条（对话前 {args.dialog_bigrams} + 书面前 {args.prose_bigrams} 合并）", file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
