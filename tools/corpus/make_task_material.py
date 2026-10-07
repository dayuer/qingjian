#!/usr/bin/env python3
"""把清洗后的语料切成「留出评测 / 开发集 / 造题原料」，并把留出与开发两份从训练语料里剔掉。

- **留出评测**（1000 句）：只在验收时跑，冻结，不许拿来调参。plain text，一行一句，直接喂
  `qingjian-cli --eval-text`（没有制表符，评测会自己按读音表转拼音）。
- **开发集**（2000 句）：调参、挑权重用。同样一行一句。
- **造题原料**：一行一条 JSON `{"id","register","prev","text"}`（`register` 是 `dialog` / `prose`，
  `prev` 是上一句 —— 对话取上一轮、书面取上一行，没有就空串；拼音训练器自己转，这边不转）。
  先给 `--train-limit`（缺省 10 万）+ 全量两个文件，都流式写，不装进内存。

去重：与 `data/eval/sentences.tsv`、回放日志里上屏过的文字重叠的句子一律不收。
排除：`--exclude-words`（一行一个词）里的词出现在句子里就不收 —— 只放**仍然有分歧**的多音字词，
读音已经定下来的（回收的、判过改的）不放，否则含「银行」这类常用词的句子会被大批删掉。

用法：
  python3 tools/corpus/make_task_material.py --corpus data/corpus/lccc.txt --register dialog \\
      --exclude-words data/eval/polyphone-disputed.txt
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


def is_candidate(line: str, limits) -> bool:
    """够不够当评测 / 造题的一句。门槛按语域给：对话短、口语、允许个别字母数字；书面的长一些、要干净。"""
    if "\t" in line or not (limits.min_len <= len(line) <= limits.max_len):
        return False
    han = len(HAN.findall(line))
    if han < limits.min_han:
        return False
    non_space = len(line.replace(" ", ""))
    if non_space == 0 or han / non_space < limits.han_ratio:
        return False
    return len(NOT_HAN_OR_PUNCT.findall(line)) <= limits.max_foreign


def existing_sentences() -> set[str]:
    """现有评测集与回放日志里出现过的句子。"""
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
    parser.add_argument("--corpus", type=pathlib.Path, required=True, help="清洗后的语料（会被就地剔掉留出与开发）")
    parser.add_argument("--register", required=True, choices=["dialog", "prose"])
    parser.add_argument("--out-dir", type=pathlib.Path, default=pathlib.Path("data/eval"))
    parser.add_argument("--train-dir", type=pathlib.Path, default=pathlib.Path("data/train"))
    parser.add_argument("--holdout", type=int, default=1000)
    parser.add_argument("--dev", type=int, default=2000)
    parser.add_argument("--train-limit", type=int, default=100_000, help="小实验用的造题条数（全量另写一份）")
    parser.add_argument("--seed", type=int, default=20261007)
    parser.add_argument("--exclude-words", type=pathlib.Path, help="一行一个词，含这些词的句子不进三份")
    parser.add_argument("--min-len", type=int, default=10)
    parser.add_argument("--max-len", type=int, default=40)
    parser.add_argument("--min-han", type=int, default=6)
    parser.add_argument("--han-ratio", type=float, default=0.6, help="汉字占非空白字符的最低比例")
    parser.add_argument("--max-foreign", type=int, default=0, help="允许出现的非汉字非标点字符数（对话里会有个别字母数字）")
    args = parser.parse_args()

    excluded_words = []
    if args.exclude_words and args.exclude_words.exists():
        excluded_words = [
            w.strip()
            for w in args.exclude_words.read_text(encoding="utf-8").splitlines()
            if w.strip() and not w.startswith("#")
        ]
    existing = existing_sentences()

    args.out_dir.mkdir(parents=True, exist_ok=True)
    args.train_dir.mkdir(parents=True, exist_ok=True)
    holdout_path = args.out_dir / f"{args.register}-holdout.txt"
    dev_path = args.out_dir / f"{args.register}-dev.txt"
    full_path = args.train_dir / f"{args.register}-all.jsonl"
    small_path = args.train_dir / f"{args.register}-{args.train_limit // 1000}k.jsonl"

    rng = random.Random(args.seed)
    want = args.holdout + args.dev
    reservoir: list[str] = []
    candidates = 0
    skipped_words = 0
    previous = ""
    written = 0
    with args.corpus.open(encoding="utf-8") as fh:
        for raw in fh:
            line = raw.rstrip("\n")
            if args.register == "dialog":
                line = line.replace(" ", "")      # LCCC 一行是分词后的串，去掉空格才是句子
            if not is_candidate(line, args):
                continue
            if excluded_words and any(word in line for word in excluded_words):
                skipped_words += 1
                continue
            if line in existing:
                continue
            # 留出与开发：蓄水池抽样，先占满 want 个
            candidates += 1
            if len(reservoir) < want:
                reservoir.append(line)
                previous = line
                continue
            index = rng.randint(0, candidates - 1)
            if index < want:
                reservoir[index] = line
                previous = line
                continue

    # 抽样定下来之后再走第二遍：写造题原料 + 剔掉留出与开发（这里只留内存里那 want 句话）
    picked = set(reservoir)
    holdout = reservoir[: args.holdout]
    dev = reservoir[args.holdout :]
    holdout_path.write_text("\n".join(holdout) + "\n", encoding="utf-8")
    dev_path.write_text("\n".join(dev) + "\n", encoding="utf-8")

    previous = ""
    with args.corpus.open(encoding="utf-8") as fh, \
            args.corpus.with_suffix(".trimmed").open("w", encoding="utf-8") as trimmed, \
            full_path.open("w", encoding="utf-8") as full, \
            small_path.open("w", encoding="utf-8") as small:
        index = 0
        kept = 0
        for raw in fh:
            line = raw.rstrip("\n")
            normalized = line.replace(" ", "") if args.register == "dialog" else line
            if normalized in picked:
                continue
            trimmed.write(line)
            trimmed.write("\n")
            kept += 1
            if not is_candidate(normalized, args):
                continue
            if excluded_words and any(word in normalized for word in excluded_words):
                continue
            if normalized in existing or normalized == previous:
                continue      # 语料的重复行（封顶时留了 3 份）不重复进造题原料
            record = json.dumps(
                {"id": f"{args.register}-{index}", "register": args.register,
                 "prev": previous, "text": normalized},
                ensure_ascii=False,
            )
            full.write(record + "\n")
            if written < args.train_limit:
                small.write(record + "\n")
            previous = normalized
            index += 1
            written += 1
    args.corpus.with_suffix(".trimmed").replace(args.corpus)

    print(f"[{args.register}] 造题候选 {candidates} 句（被排除词剔掉 {skipped_words}）", file=sys.stderr)
    for path, count in ((holdout_path, len(holdout)), (dev_path, len(dev)),
                        (small_path, min(written, args.train_limit)), (full_path, written)):
        digest = hashlib.sha256(path.read_bytes()).hexdigest()
        print(f"  {path}  {count} 条  sha256={digest[:16]}", file=sys.stderr)
    print(f"  训练语料剔掉留出与开发后剩 {kept} 行", file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
