#!/usr/bin/env python3
"""把「自动改」与人工判定合成 lexicon 用的拼音修正文件 `pinyin-corrections.jsonl`。

来源三份，后者覆盖前者：
- `polyphone-auto.tsv`：polyphone-fix.py 自动采纳的（词 / 词库读音 / 建议读音）；
- `polyphone-verdicts.tsv`：逐条人工判定（词 / 词库读音 / 建议读音 / 判定 / 理由），
  只有判定为「改」的进结果，同名覆盖自动条目；
- `polyphone-marks.tsv`：降权标记（词 / 标记 / 理由），只改「旧读音怎么处理」，不改主读音。

标记 —— 这是 R5（见 docs/plan/dictionary-layering.md）：
- 空：旧读音是错的，除以 lexicon 的 `DISPUTED_READING_DIVISOR`（8）；
- `keep_both`：两个读音都是规范读音、又都常用（谁 shui/shei、重装 chong/zhong），同权保留；
- 数字 N：旧读音不是错的、只是常有人打（露 lóu 家词），降权放轻，除以 N。

**单字条目一律不进结果**：单字走规范字表那条路（按 Unihan 读音频次分摊，如 谁 shui 29 / shei 1），
不查标注 —— 2026-10-07 复跑实测，修正文件里的单字条目全是空转（195/195），所以在这里挡掉，
不让它们冒充「已生效的修正」。

用法：python3 tools/lexicon/polyphone-apply.py
"""
import argparse
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]

HEADER = (
    "# 多音字拼音修正：由 tools/lexicon/polyphone-apply.py 合成，不要手改。\n"
    "# 来源：polyphone-auto.tsv（自动采纳）+ polyphone-verdicts.tsv（人工判定）。\n"
    "# 每行 {\"word\":…,\"pinyin\":[…]}\n"
)


def read_tsv(path: Path) -> list[list[str]]:
    rows = []
    for line in path.read_text(encoding="utf-8-sig").splitlines():
        if line.startswith("#") or not line.strip():
            continue
        rows.append(line.split("\t"))
    return rows


def entry(word: str, syllables: str, keep_both: bool = False, divisor: str = "") -> dict:
    row = {"word": word, "pinyin": syllables.split()}
    if keep_both:
        row["keep_both"] = True
    elif divisor:
        row["divisor"] = int(divisor)
    return row


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--auto", type=Path, default=ROOT / "assets/lexicon/00_meta/polyphone-auto.tsv")
    parser.add_argument("--verdicts", type=Path, default=ROOT / "assets/lexicon/00_meta/polyphone-verdicts.tsv")
    parser.add_argument("--marks", type=Path, default=ROOT / "assets/lexicon/00_meta/polyphone-marks.tsv")
    parser.add_argument("--out", type=Path, default=ROOT / "assets/lexicon/00_meta/pinyin-corrections.jsonl")
    args = parser.parse_args()

    corrections: dict[str, dict] = {}
    skipped_single = 0
    for fields in read_tsv(args.auto):
        if len(fields) < 3:
            continue
        word = fields[0].strip()
        if len(word) == 1:
            skipped_single += 1
            continue
        corrections[word] = entry(word, fields[2].strip())

    changed = 0
    for fields in read_tsv(args.verdicts):
        if len(fields) < 4 or fields[3].strip() != "改":
            continue
        word, syllables = fields[0].strip(), fields[2].strip()
        if not syllables:
            raise SystemExit(f"判定为「改」的「{word}」没写建议读音：补上，或者把判定改成「不确定」")
        if len(word) == 1:
            skipped_single += 1
            continue
        corrections[word] = entry(word, syllables)
        changed += 1

    marked = {"keep_both": 0, "divisor": 0}
    for fields in read_tsv(args.marks):
        if len(fields) < 2:
            continue
        word, mark = fields[0].strip(), fields[1].strip()
        if word not in corrections:
            raise SystemExit(f"标记里的「{word}」不在修正清单里：标记只能落在已有的修正条目上")
        if mark == "keep_both":
            corrections[word]["keep_both"] = True
            marked["keep_both"] += 1
        elif mark.isdigit() and int(mark) > 1:
            corrections[word]["divisor"] = int(mark)
            marked["divisor"] += 1
        else:
            raise SystemExit(f"标记「{word}」的值看不懂：{mark}（只认 keep_both 或大于 1 的整数）")

    with args.out.open("w", encoding="utf-8") as fh:
        fh.write(HEADER)
        for word in sorted(corrections):
            fh.write(json.dumps(corrections[word], ensure_ascii=False) + "\n")

    print(
        f"修正 {len(corrections)} 条 → {args.out}（人工判定 {changed} 条；"
        f"其中 keep_both {marked['keep_both']} 条、放轻降权 {marked['divisor']} 条；"
        f"单字条目挡掉 {skipped_single} 条 —— 字表那条路不查标注）"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
