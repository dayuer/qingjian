#!/usr/bin/env python3
"""整行完全相同的段落封顶：一句话重复一万次不该顶过一整篇文章。

输入是 `sort <语料> | uniq -c` 的输出（`次数 行`）—— 去重靠外排，不把 2500 万行读进内存；
输出是封顶后的纯文本（`-o`），统计与样本打到 stderr / `--report`。

同一句话在语料里出现三次以上的部分丢掉：模板句（「本条目需要补充来源」）、对话语料里的短回复（「好的」「嗯」）
都靠它压住。上限用 `--max-repeat`（缺省 3）。

用法（`build_corpus.sh` 里就是这么接的）：
  sort 语料.raw -S 1G -T /tmp | uniq -c | python3 tools/corpus/cap_repeats.py -o 语料.txt --report 报告.txt
"""

import argparse
import pathlib
import re
import sys
from collections import Counter

COUNT_LINE = re.compile(r"^\s*(\d+) (.*)$")

SAMPLE_COUNT = 10


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("-i", "--input", help="`uniq -c` 的输出文件；不给就读 stdin")
    parser.add_argument("-o", "--output", required=True, help="输出的纯文本文件")
    parser.add_argument("--preserve-order", action="store_true",
                        help="保持输入原序（对话语料要保留轮次相邻关系，给造题原料用 prev）"
                             "；此时 -i 是**原始逐行文件**，计数从 --counts 读")
    parser.add_argument("--counts", help="--preserve-order 时的计数文件（`sort | uniq -c` 的输出）")
    parser.add_argument("--drop-above-percentile", type=float,
                        help="出现次数高过这个分位（如 99.99）的整行剔掉，单列进报告："
                             "「冷藏」78.8 万次这种模板刷屏靠它挡，正常口语的重复（「哈哈」几万次）留着")
    parser.add_argument("--max-repeat", type=int, default=3, help="整行最多算几次")
    parser.add_argument("--report", help="统计写到这个文件（缺省只打 stderr）")
    args = parser.parse_args()

    # 先扫一遍拿分位数（只留次数的分布，不留文本）
    threshold = None
    if args.drop_above_percentile is not None:
        counts: list[int] = []
        source = pathlib.Path(args.input) if args.input else None
        if source is None:
            raise SystemExit("--drop-above-percentile 需要 -i/--input（管道读不了两遍）")
        with source.open(encoding="utf-8") as fh:
            for raw in fh:
                match = COUNT_LINE.match(raw)
                if match:
                    counts.append(int(match.group(1)))
        counts.sort()
        index = min(len(counts) - 1, int(len(counts) * args.drop_above_percentile / 100))
        threshold = counts[index] if counts else None
        print(f"  {args.drop_above_percentile} 分位的出现次数是 {threshold}，高过它整行剔掉", file=sys.stderr)

    total = 0          # 输入行数（含重复）
    unique = 0         # 去重后的行数
    capped = 0         # 因为超过上限丢掉的次数
    dropped_spam = 0
    kept = 0
    repeated = Counter()
    spam: list[tuple[int, str]] = []
    samples: list[tuple[int, str]] = []
    if args.preserve_order:
        if not (args.input and args.counts):
            raise SystemExit("--preserve-order 需要 -i（原始逐行）与 --counts（uniq -c 输出）")
        quota: dict[str, int] = {}
        with pathlib.Path(args.counts).open(encoding="utf-8") as fh:
            for raw in fh:
                match = COUNT_LINE.match(raw)
                if not match:
                    continue
                count, line = int(match.group(1)), match.group(2)
                if threshold is not None and count > threshold:
                    quota[line] = 0
                    continue
                quota[line] = min(count, args.max_repeat)
        written = 0
        with pathlib.Path(args.input).open(encoding="utf-8") as fh, \
                open(args.output, "w", encoding="utf-8") as out:
            for raw in fh:
                line = raw.rstrip("\n")
                left = quota.get(line, 0)
                if left <= 0:
                    continue
                quota[line] = left - 1
                out.write(line)
                out.write("\n")
                written += 1
        total = sum(quota.values()) + written
        print(f"[保持原序] 保留 {written} 行，去重后 {len(quota)} 种", file=sys.stderr)
        return 0

    source = pathlib.Path(args.input).open(encoding="utf-8") if args.input else sys.stdin
    with open(args.output, "w", encoding="utf-8") as out, source:
        for raw in source:
            # uniq -c 的格式是「右对齐的次数 + 空格 + 整行」，次数前面有空白
            match = COUNT_LINE.match(raw)
            if match is None:       # 格式不对就当成出现一次
                count, line = 1, raw.rstrip("\n")
            else:
                count, line = int(match.group(1)), match.group(2)
            total += count
            unique += 1
            if threshold is not None and count > threshold:
                dropped_spam += 1
                if len(spam) < SAMPLE_COUNT:
                    spam.append((count, line))
                continue
            kept += min(count, args.max_repeat)
            capped += max(0, count - args.max_repeat)
            if count > args.max_repeat:
                repeated[line] += count
                if len(samples) < 40:
                    samples.append((count, line))
            for _ in range(min(count, args.max_repeat)):
                out.write(line)
                out.write("\n")

    most = repeated.most_common(SAMPLE_COUNT)
    lines = [
        f"重复行封顶（每行最多算 {args.max_repeat} 次）：",
        f"  输入 {total} 段（去重后 {unique} 种），保留 {kept} 段，因超上限丢掉 {capped} 段"
        + (f"，按分位剔掉模板刷屏 {dropped_spam} 种" if threshold is not None else ""),
    ]
    if spam:
        lines.append("  按分位整行剔掉的（出现次数高得不像自然语言）：")
        for count, line in spam:
            lines.append(f"      出现 {count} 次  {line[:80]}")
    lines.append(f"  被截住的重复行有 {len(repeated)} 种；重复最多的是：")
    for line, count in most:
        lines.append(f"      出现 {count} 次  {line[:80]}")
    text = "\n".join(lines)
    print(text, file=sys.stderr)
    if args.report:
        with open(args.report, "w", encoding="utf-8") as fh:
            fh.write(text + "\n")


if __name__ == "__main__":
    main()
