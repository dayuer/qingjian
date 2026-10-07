# /// script
# requires-python = ">=3.11"
# dependencies = ["pyarrow>=17", "opencc-python-reimplemented>=0.1.7"]
# ///
"""把 Hugging Face 上的中文语料 parquet 转成纯文本（每行一段，繁体转简体），供 `dict-convert bigram` 统计。

用法：
  uv run tools/corpus/parquet_to_text.py data/corpus/zhwiki-*.parquet -o data/corpus/zhwiki.txt
  uv run tools/corpus/parquet_to_text.py data/corpus/lccc-*.parquet --column dialog -o data/corpus/lccc.txt

已验证的数据源（都只用于开发测试，见 docs/design/landscape.md）：
  wikimedia/wikipedia 20231101.zh   列 text，CC BY-SA 4.0
  thu-coai/lccc（refs/convert/parquet，base/train）列 dialog（每行一个列表，一轮一句），MIT

转换时做两件清洗，都打统计与样本到 stderr（`--report` 可另外写一份进文件），防止误删正文：

1. **丢非正文行**：维基正文里混着大量章节标题与模板残留 —— 2026-10-07 量到 91 万行是光秃秃的标题
   （参考文献 47 万、参见 17 万、外部链接 14 万），`参见` 一个词凭空多出 17 万次，把百科与对话的配比整个带偏。
重复行封顶（同一模板句、对话里反复出现的短回复）在下一步做：`tools/corpus/cap_repeats.py`（外排去重，
不占内存），两步一起见 `tools/corpus/build_corpus.sh`。
"""

import argparse
import re
import sys
from collections import Counter

import pyarrow.parquet as pq
from opencc import OpenCC

# 多余空白
SPACES = re.compile(r"[ \t　]+")

# 维基的章节标题：整行就是这几个字，不是句子
SECTION_HEADS = frozenset(
    """参见 参见条目 相关条目 另见 注释 注释与参考资料 注解 参考资料 参考来源 参考文献
    来源 脚注 延伸阅读 外部链接 外部连接 外部连结 外部連結 外部链接与参考 图集 画廊 图片 目录
    注释和参考 参考文献及注释 参考 书目 文献 引用 引用来源 备注 附注 注释与参考 阅读更多""".split()
)
# 整行只有标点 / 表格与标题符号（`|-`、`|}`、`===`、`*` 这类）的，不是句子
ONLY_MARKUP = re.compile(r"^[\s\-–—|{}()\[\]<>#*:;=~.、。，,！？!?…·\"'“”‘’]+$")

SAMPLE_COUNT = 10


def drop_reason(line: str) -> str | None:
    """这一行该按哪条规则丢掉；正文返回 None。"""
    if line in SECTION_HEADS:
        return "章节标题"
    if ONLY_MARKUP.match(line):
        return "只有标点或表格符号"
    # 模板与表格、HTML 残留：`{{cite web|…}}`、`{|`、`<div id=…>`；正文里的行内链接（`[[北京]]是…`）不动
    if "{{" in line or "{|" in line:
        return "模板或表格残留"
    if line.startswith("<"):
        return "HTML 残留"
    return None


def flatten(value):
    """列可能是字符串，也可能是字符串列表（对话的每一轮）。"""
    if value is None:
        return
    if isinstance(value, str):
        yield from value.split("\n")
        return
    for item in value:
        yield from flatten(item)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("inputs", nargs="+", help="parquet 文件")
    parser.add_argument("-o", "--output", required=True, help="输出的纯文本文件")
    parser.add_argument("--column", default="text", help="取哪一列（默认 text）")
    parser.add_argument("--min-chars", type=int, default=2, help="短于此的段落丢掉")
    parser.add_argument("--report", help="清洗统计写到这个文件（缺省只打 stderr）")
    args = parser.parse_args()

    converter = OpenCC("t2s")
    dropped: Counter[str] = Counter()
    samples: dict[str, list[str]] = {}
    kept = 0
    with open(args.output, "w", encoding="utf-8") as out:
        for path in args.inputs:
            table = pq.ParquetFile(path)
            for batch in table.iter_batches(columns=[args.column], batch_size=2048):
                for value in batch.column(args.column).to_pylist():
                    for line in flatten(value):
                        line = SPACES.sub(" ", line).strip()
                        if len(line) < args.min_chars:
                            dropped["太短"] += 1
                            continue
                        line = converter.convert(line)
                        reason = drop_reason(line)
                        if reason is not None:
                            dropped[reason] += 1
                            bucket = samples.setdefault(reason, [])
                            if len(bucket) < SAMPLE_COUNT:
                                bucket.append(line[:80])
                            continue
                        out.write(line)
                        out.write("\n")
                        kept += 1
            print(f"{path}: 累计保留 {kept} 段", file=sys.stderr)

    total_dropped = sum(dropped.values())
    lines = [f"清洗统计：保留 {kept} 段，丢掉 {total_dropped} 段", ""]
    for reason, count in dropped.most_common():
        lines.append(f"  {reason}：{count} 段")
        for sample in samples.get(reason, []):
            lines.append(f"      样本 {sample}")
    text = "\n".join(lines)
    print(text, file=sys.stderr)
    if args.report:
        with open(args.report, "w", encoding="utf-8") as fh:
            fh.write(text + "\n")


if __name__ == "__main__":
    main()
