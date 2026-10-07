#!/usr/bin/env python3
"""把清洗后的语料切出三份互不重叠的集合，并把它们从训练语料里剔掉。

一份语料切三份（对话 = LCCC，书面 = 维基，各自切）：

| 集合 | 用途 | 缺省条数 |
|---|---|---|
| 留出评测 | 只在验收时跑，冻结、不许拿来调参 | 1000 |
| 开发集 | 调参、选权重用 | 1000 |
| 造题用 | 「解码器在环的代价训练」造题（见 docs/design/decoder-in-the-loop.md） | 5000 |

三份互不重叠，且都会从 `--corpus` 那个训练文件里删掉（剔完剩下的才是训练语料）。
另外做去重校验：与现有评测集 `data/eval/sentences.tsv`、回放日志里上屏过的句子撞了就不收。

音质门槛：一行至少 6 个汉字、汉字占非空白字符的六成以上，长度 10–40 字；
可选 `--exclude-words <文件>`（一行一个词）—— 含这些词的句子一律不进三份（多音字有分歧的词就靠它剔）。

用法：
  python3 tools/corpus/make_splits.py --corpus data/corpus/zhwiki.txt --register written \\
      --out-dir data/eval --exclude-words data/eval/polyphone-disputed.txt
"""

import argparse
import hashlib
import json
import pathlib
import random
import re
import sys

HAN = re.compile(r"[一-鿿]")
NOT_HAN_OR_PUNCT = re.compile(r"[^　-〿一-鿿＀-￯]")
SENTENCES = pathlib.Path("data/eval/sentences.tsv")
REPLAY = pathlib.Path("data/eval/input-log-2026-10-04.jsonl")


def is_candidate(line: str) -> bool:
    if "\t" in line or not (10 <= len(line) <= 40):
        return False
    han = len(HAN.findall(line))
    if han < 6:
        return False
    non_space = len(line.replace(" ", ""))
    return non_space > 0 and han / non_space >= 0.6 and not NOT_HAN_OR_PUNCT.search(line)


def existing_sentences() -> set[str]:
    """现有评测集与回放日志里出现过的句子：新切的三份不许与它们重复。"""
    seen: set[str] = set()
    if SENTENCES.exists():
        for line in SENTENCES.read_text(encoding="utf-8").splitlines():
            if line.strip() and not line.startswith("#"):
                seen.add(line.split("\t")[0].strip())
    if REPLAY.exists():
        for line in REPLAY.read_text(encoding="utf-8").splitlines():
            if not line.strip():
                continue
            try:
                entry = json.loads(line)
            except json.JSONDecodeError:
                continue
            text = entry.get("text")
            if isinstance(text, str) and text:
                seen.add(text)
    return seen


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--corpus", type=pathlib.Path, required=True, help="清洗后的语料（会被就地剔掉切出来的句子）")
    parser.add_argument("--register", required=True, help="dialogue（LCCC）或 written（维基）")
    parser.add_argument("--out-dir", type=pathlib.Path, default=pathlib.Path("data/eval"))
    parser.add_argument("--holdout", type=int, default=1000)
    parser.add_argument("--dev", type=int, default=1000)
    parser.add_argument("--train", type=int, default=5000)
    parser.add_argument("--seed", type=int, default=20261007)
    parser.add_argument("--exclude-words", type=pathlib.Path, help="一行一个词，含这些词的句子不收")
    args = parser.parse_args()

    excluded_words = []
    if args.exclude_words and args.exclude_words.exists():
        excluded_words = [
            w.strip()
            for w in args.exclude_words.read_text(encoding="utf-8").splitlines()
            if w.strip() and not w.startswith("#")
        ]

    rng = random.Random(args.seed)
    want = args.holdout + args.dev + args.train
    reservoir: list[str] = []
    candidates = 0
    skipped_existing = 0
    skipped_words = 0
    with args.corpus.open(encoding="utf-8") as fh:
        for raw in fh:
            line = raw.rstrip("\n")
            if args.register == "dialogue":
                line = line.replace(" ", "")      # LCCC 一行是分词后的串，去掉空格才是句子
            if not is_candidate(line):
                continue
            if excluded_words and any(word in line for word in excluded_words):
                skipped_words += 1
                continue
            candidates += 1
            if len(reservoir) < want:
                reservoir.append(line)
            else:
                index = rng.randint(0, candidates - 1)
                if index < want:
                    reservoir[index] = line

    # 与现有评测集 / 回放去重之后再分三份
    existing = existing_sentences()
    fresh = []
    for line in reservoir:
        if line in existing or line in fresh:
            skipped_existing += 1
            continue
        fresh.append(line)
    holdout = fresh[: args.holdout]
    dev = fresh[args.holdout : args.holdout + args.dev]
    train = fresh[args.holdout + args.dev : want]

    args.out_dir.mkdir(parents=True, exist_ok=True)
    picked = set(fresh)
    files = {}
    for name, rows in (("holdout", holdout), ("dev", dev), ("train", train)):
        target = args.out_dir / f"{args.register}-{name}.txt"
        target.write_text("\n".join(rows) + "\n", encoding="utf-8")
        files[name] = (target, rows)

    # 把切出来的句子从训练语料里剔掉
    kept = 0
    removed = 0
    with args.corpus.open(encoding="utf-8") as fh, args.corpus.with_suffix(".trimmed").open("w", encoding="utf-8") as out:
        for raw in fh:
            line = raw.rstrip("\n")
            normalized = line.replace(" ", "") if args.register == "dialogue" else line
            if normalized in picked:
                removed += 1
                continue
            out.write(line)
            out.write("\n")
            kept += 1
    args.corpus.with_suffix(".trimmed").replace(args.corpus)

    print(
        f"[{args.register}] 候选 {candidates} 句（含被排除词剔掉的 {skipped_words}）"
        f"→ 留出 {len(holdout)} / 开发 {len(dev)} / 造题 {len(train)}"
        f"（与现有评测集或彼此撞了 {skipped_existing}）；训练语料 {kept} 行（剔掉 {removed}）",
        file=sys.stderr,
    )
    for name, (target, rows) in files.items():
        digest = hashlib.sha256(target.read_bytes()).hexdigest()
        print(f"  {name}: {target}  {len(rows)} 句  sha256={digest[:16]}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
