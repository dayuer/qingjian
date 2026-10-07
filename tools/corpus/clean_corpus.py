#!/usr/bin/env python3
"""语料清洗与去重（**全部用 Python + sqlite**，不碰 sort / uniq / awk）。

2026-10-08 的教训：macOS 的 `sort` / `uniq -c` / `awk` 在 CJK 上会给出错的结果 ——
「LCCC 前 20 句占 85.8%」那组统计就是它们算出来的假数字。真值（Python 数）：原始 20,067,915 轮、
不同单句 14,959,069 种，最多的「哈哈」41,078 次、「谢谢」37,713 次。**凡是数数的事一律在 Python 里做**，
计数用 sqlite（两千万个不同的句子，放内存里要两个 G，放库里不占内存）。

流程：

1. 读 parquet，写一份**不再改动**的 `<名>-raw.txt`（原始文本不覆盖，派生文件另起名字）；
2. 清洗：非正文行（维基章节标题、模板 / HTML 残留、只有标点的行）、长度与汉字门槛；`--raw` 关掉全部清洗
   （只为复现旧口径）；
3. 整段对话去重（`--dedup-dialogs`）：同一段对话完整重复的只留第一份（对话哈希进 sqlite）；
4. 单句封顶 `--max-repeat`：只对**超过上限的那几句**留状态，正常句子直接写（不占内存）；
5. 报告：清洗前后轮次、不同单句数、最高频 20 句（与审计那边对拍）、被剔掉的句子。

用法：
  python3 tools/corpus/clean_corpus.py --parquet data/corpus/lccc-base-train-*.parquet --column dialog \
      --register dialog --dedup-dialogs --max-repeat 10000 \
      --out data/corpus/lccc.txt --raw-out data/corpus/lccc-raw.txt --report /tmp/lccc-clean-report.txt
"""

import argparse
import hashlib
import pathlib
import re
import sqlite3
import sys

import pyarrow.parquet as pq

HAN = re.compile(r"[一-鿿]")
SPACES = re.compile(r"[ \t　]+")
SECTION_HEADS = frozenset(
    """参见 参见条目 相关条目 另见 注释 注释与参考资料 注解 参考资料 参考来源 参考文献
    来源 脚注 延伸阅读 外部链接 外部连接 外部连结 外部連結 外部链接与参考 图集 画廊 图片 目录
    注释和参考 参考文献及注释 参考 书目 文献 引用 引用来源 备注 附注 注释与参考 阅读更多""".split()
)
ONLY_MARKUP = re.compile(r"^[\s\-–—|{}()\[\]<>#*:;=~.、。，,！？!?…·\"'“”‘’]+$")


def drop_reason(line: str) -> str | None:
    if line in SECTION_HEADS:
        return "章节标题"
    if ONLY_MARKUP.match(line):
        return "只有标点或表格符号"
    if "{{" in line or "{|" in line:
        return "模板或表格残留"
    if line.startswith("<"):
        return "HTML 残留"
    return None


def digest(text: str) -> int:
    return int.from_bytes(hashlib.blake2b(text.encode("utf-8"), digest_size=8).digest(), "big")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--parquet", nargs="+", required=True)
    parser.add_argument("--column", default="text")
    parser.add_argument("--register", required=True)
    parser.add_argument("--out", type=pathlib.Path, required=True)
    parser.add_argument("--raw-out", type=pathlib.Path, required=True, help="不再改动的原始文本")
    parser.add_argument("--report", type=pathlib.Path)
    parser.add_argument("--db", type=pathlib.Path, default=pathlib.Path("/tmp/corpus-counts.sqlite"))
    parser.add_argument("--min-chars", type=int, default=2)
    parser.add_argument("--min-han", type=int, default=1)
    parser.add_argument("--dedup-dialogs", action="store_true")
    parser.add_argument("--max-repeat", type=int, default=10_000, help="整行最多算几次")
    parser.add_argument("--raw", action="store_true", help="不做清洗（只用于复现旧口径）")
    parser.add_argument("--trim", nargs="*", default=[], help="要剔掉的句子文件（留出集 / 开发集）")
    args = parser.parse_args()

    trim: set[int] = set()
    for path in args.trim:
        for line in pathlib.Path(path).read_text(encoding="utf-8").splitlines():
            text = line.replace(" ", "").strip() if args.register == "dialog" else line.strip()
            if text:
                trim.add(digest(text))

    args.db.unlink(missing_ok=True)
    db = sqlite3.connect(args.db)
    db.execute("PRAGMA journal_mode = OFF")
    db.execute("PRAGMA synchronous = OFF")
    db.execute("CREATE TABLE counts (key INTEGER PRIMARY KEY, sample TEXT, count INTEGER)")
    db.execute("CREATE TABLE dialogs (key INTEGER PRIMARY KEY)")

    dropped = {"太短": 0, "太窄": 0}
    dropped_reason: dict[str, int] = {}
    total_turns = 0
    duplicate_turns = 0
    with args.raw_out.open("w", encoding="utf-8") as raw_out:
        for path in args.parquet:
            table = pq.ParquetFile(path)
            for batch in table.iter_batches(columns=[args.column], batch_size=512):
                pending: list[tuple[str, ...]] = []
                for value in batch.column(args.column).to_pylist():
                    turns = []
                    for turn in value if isinstance(value, list) else [value]:
                        for piece in str(turn or "").split("\n"):
                            line = SPACES.sub(" ", piece).strip()
                            if len(line) < args.min_chars:
                                dropped["太短"] += 1
                                continue
                            if len(HAN.findall(line)) < args.min_han:
                                dropped["太窄"] += 1
                                continue
                            if not args.raw:
                                reason = drop_reason(line)
                                if reason is not None:
                                    dropped_reason[reason] = dropped_reason.get(reason, 0) + 1
                                    continue
                            turns.append(line)
                    if not turns:
                        continue
                    if args.dedup_dialogs:
                        key = digest("\n".join(turns))
                        if db.execute("SELECT 1 FROM dialogs WHERE key = ?", (key,)).fetchone():
                            duplicate_turns += len(turns)
                            continue
                        db.execute("INSERT INTO dialogs (key) VALUES (?)", (key,))
                    pending.append(tuple(turns))
                    total_turns += len(turns)
                for turns in pending:
                    for turn in turns:
                        raw_out.write(turn + "\n")     # 一轮一行：轮次相邻关系要留住（造题原料的 prev 靠它）
                for turns in pending:
                    db.executemany(
                        "INSERT INTO counts (key, sample, count) VALUES (?, ?, 1) "
                        "ON CONFLICT(key) DO UPDATE SET count = count + 1",
                        [(digest(turn), turn) for turn in turns],
                    )
            db.commit()
            print(f"  读完 {path}（累计 {total_turns} 轮）", file=sys.stderr)

    distinct = db.execute("SELECT COUNT(*) FROM counts").fetchone()[0]
    written = 0
    over: dict[int, int] = {}          # 只给「超过上限」的句子留状态
    lines = [
        f"[{args.register}] 轮次 {total_turns}，不同句子 {distinct} 种 → {args.raw_out}（不再改动）",
    ]
    if duplicate_turns:
        lines.append(f"  整段对话去重丢弃 {duplicate_turns} 轮")
    if dropped["太短"] or dropped["太窄"] or dropped_reason:
        lines.append(
            f"  清洗丢掉：太短 {dropped['太短']}、汉字太少 {dropped['太窄']}、"
            + "、".join(f"{k} {v}" for k, v in dropped_reason.items())
        )
    lines.append("  最高频 20 句（与审计那边对拍）：")
    for sample, count in db.execute("SELECT sample, count FROM counts ORDER BY count DESC, key LIMIT 20"):
        lines.append(f"      {count} 次  {sample[:60]}")

    with args.raw_out.open(encoding="utf-8") as fh, args.out.open("w", encoding="utf-8") as out:
        for raw in fh:
            turn = raw.rstrip("\n")
            if not turn:
                continue
            key = digest(turn)
            if key in trim:
                continue
            count = db.execute("SELECT count FROM counts WHERE key = ?", (key,)).fetchone()
            if count is None:
                continue
            count = count[0]
            if count > args.max_repeat:
                left = over.get(key, args.max_repeat)
                if left <= 0:
                    continue
                over[key] = left - 1
            out.write(turn + "\n")
            written += 1

    lines.append(f"  写出 {written} 行 → {args.out}（上限 {args.max_repeat} 次）")
    text = "\n".join(lines)
    print(text, file=sys.stderr)
    if args.report:
        args.report.write_text(text + "\n", encoding="utf-8")
    db.close()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
