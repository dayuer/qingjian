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
import re
import sys
from collections import Counter

COUNT_LINE = re.compile(r"^\s*(\d+) (.*)$")

SAMPLE_COUNT = 10


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("-o", "--output", required=True, help="输出的纯文本文件")
    parser.add_argument("--max-repeat", type=int, default=3, help="整行最多算几次")
    parser.add_argument("--report", help="统计写到这个文件（缺省只打 stderr）")
    args = parser.parse_args()

    total = 0          # 输入行数（含重复）
    unique = 0         # 去重后的行数
    capped = 0         # 因为超过上限丢掉的次数
    kept = 0
    repeated = Counter()
    samples: list[tuple[int, str]] = []
    sys.stdin.reconfigure(encoding="utf-8")
    with open(args.output, "w", encoding="utf-8") as out:
        for raw in sys.stdin:
            # uniq -c 的格式是「右对齐的次数 + 空格 + 整行」，次数前面有空白
            match = COUNT_LINE.match(raw)
            if match is None:       # 格式不对就当成出现一次
                count, line = 1, raw.rstrip("\n")
            else:
                count, line = int(match.group(1)), match.group(2)
            total += count
            unique += 1
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
        f"  输入 {total} 段（去重后 {unique} 种），保留 {kept} 段，因超上限丢掉 {capped} 段",
        f"  被截住的重复行有 {len(repeated)} 种；重复最多的是：",
    ]
    for line, count in most:
        lines.append(f"      出现 {count} 次  {line[:80]}")
    text = "\n".join(lines)
    print(text, file=sys.stderr)
    if args.report:
        with open(args.report, "w", encoding="utf-8") as fh:
            fh.write(text + "\n")


if __name__ == "__main__":
    main()
