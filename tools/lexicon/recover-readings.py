#!/usr/bin/env python3
"""从旧产物里回收读音：旧 `dict.tsv` 里有、新重建后变了的词，旧读音满足条件就当作修正带过来。

为什么要有这一步：多音字里那一批正确读音是 LLM 标注（`pinyin-llm.jsonl`）挣来的，那份中间产物不在 git 里、
本机也丢了；但**旧 dict.tsv 本身就是标注之后的结果**，读音还留在里面。所以重建之后拿旧库对一遍，
把站得住的读音捞回来，别让重建把读音面拉低。

采纳条件（审计定的，两条任一）：
1. 与 CC-CEDICT 给这个词的读音一致；
2. Unihan 认这个读音（逐字）且不在台湾读音表里。

剩下的写进人工清单（`--manual`）等 LLM 那一步或人工判。结果入 git
（`assets/lexicon/00_meta/pinyin-recovered.jsonl`），是「读音三方校验」的一个来源。

用法：
  python3 tools/lexicon/recover-readings.py --new <新 dict.tsv> [--old assets/lexicon/dict.tsv]
"""

import argparse
import collections
import json
import pathlib
import re
import unicodedata

ROOT = pathlib.Path(__file__).resolve().parents[2]


def strip_tone(syllable: str) -> str:
    syllable = re.sub(r"\d", "", syllable)
    for a, b in [("ü", "v"), ("ǖ", "v"), ("ǘ", "v"), ("ǚ", "v"), ("ǜ", "v"), ("u:", "v")]:
        syllable = syllable.replace(a, b)
    decomposed = unicodedata.normalize("NFD", syllable)
    return "".join(c for c in decomposed if not unicodedata.combining(c)).lower()


def load_dict(path: pathlib.Path) -> dict[str, dict[str, int]]:
    """词 → {读音: 词频}。"""
    out: dict[str, dict[str, int]] = collections.defaultdict(dict)
    for line in path.read_text(encoding="utf-8").splitlines():
        if line.startswith("#") or not line.strip():
            continue
        fields = line.split("\t")
        if len(fields) >= 3:
            out[fields[0]][fields[1]] = int(fields[2])
    return out


def primary(readings: dict[str, int]) -> str:
    return max(readings.items(), key=lambda kv: (kv[1], kv[0]))[0]


def load_unihan(path: pathlib.Path) -> dict[str, set[str]]:
    readings: dict[str, set[str]] = collections.defaultdict(set)
    for line in path.read_text(encoding="utf-8").splitlines():
        if line.startswith("#") or not line.strip():
            continue
        fields = line.split("\t")
        if len(fields) < 3 or fields[1] not in ("kHanyuPinlu", "kXHC1983", "kMandarin"):
            continue
        ch = chr(int(fields[0][2:], 16))
        for syllable in re.findall(r"([A-Za-züÀ-ɏ]+)", fields[2]):
            readings[ch].add(strip_tone(syllable))
    return readings


def load_cedict(path: pathlib.Path) -> dict[str, set[str]]:
    out: dict[str, set[str]] = collections.defaultdict(set)
    if not path or not path.exists():
        return out
    for line in path.read_text(encoding="utf-8").splitlines():
        if line.startswith("#") or not line.strip():
            continue
        m = re.match(r"^(\S+)\s+(\S+)\s+\[([^\]]*)\]\s+/.*/\s*$", line)
        if m:
            out[m.group(2)].add(" ".join(strip_tone(s) for s in m.group(3).split()))
    return out


def load_taiwan(path: pathlib.Path) -> dict[str, set[str]]:
    """字或词 → 台湾读音集合。"""
    out: dict[str, set[str]] = collections.defaultdict(set)
    if not path.exists():
        return out
    for line in path.read_text(encoding="utf-8").splitlines():
        if line.startswith("#") or not line.strip():
            continue
        fields = line.split("\t")
        if len(fields) >= 3:
            out[fields[0]].add(strip_tone(fields[2]))
    return out


def unihan_accepts(unihan, word: str, syllables: list[str]) -> bool:
    return len(word) == len(syllables) and all(
        s in unihan.get(ch, set()) for ch, s in zip(word, syllables)
    )


def is_taiwan(taiwan, word: str, reading: str) -> bool:
    if reading in taiwan.get(word, set()):
        return True
    return any(s in taiwan.get(ch, set()) for ch, s in zip(word, reading.split()))


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--old", type=pathlib.Path, default=ROOT / "assets/lexicon/dict.tsv")
    parser.add_argument("--new", type=pathlib.Path, required=True)
    parser.add_argument("--unihan", type=pathlib.Path, default=ROOT / "data/unihan/Unihan_Readings.txt")
    parser.add_argument("--cedict", type=pathlib.Path, default=pathlib.Path("/tmp/cedict.txt"))
    parser.add_argument("--taiwan", type=pathlib.Path, default=ROOT / "assets/lexicon/00_meta/taiwan-readings.tsv")
    parser.add_argument("--out", type=pathlib.Path, default=ROOT / "assets/lexicon/00_meta/pinyin-recovered.jsonl")
    parser.add_argument("--manual", type=pathlib.Path, default=ROOT / "assets/lexicon/00_meta/pinyin-recovery-manual.tsv")
    parser.add_argument("--samples", type=int, default=15)
    args = parser.parse_args()

    old = load_dict(args.old)
    new = load_dict(args.new)
    unihan = load_unihan(args.unihan)
    cedict = load_cedict(args.cedict)
    taiwan = load_taiwan(args.taiwan)

    recovered, manual, same = [], [], 0
    for word in sorted(set(old) & set(new)):
        before, after = primary(old[word]), primary(new[word])
        if before == after:
            same += 1
            continue
        if "@" in before or not all(part.isalpha() for part in before.split()):
            continue
        if before in cedict.get(word, set()):
            recovered.append((word, before, after, "与 CC-CEDICT 一致"))
        elif unihan_accepts(unihan, word, before.split()) and not is_taiwan(taiwan, word, before):
            recovered.append((word, before, after, "Unihan 认、不在台湾读音表"))
        else:
            manual.append((word, before, after, "CC-CEDICT 与 Unihan 都不认（或台湾读音）"))

    with args.out.open("w", encoding="utf-8") as fh:
        fh.write("# 从旧 dict.tsv 回收的多音字读音（条件见 tools/lexicon/recover-readings.py）：\n")
        fh.write("# 与 CC-CEDICT 一致，或 Unihan 认且不在台湾读音表。lexicon --pinyin 直接用；\n")
        fh.write("# 与人工修正（pinyin-corrections.jsonl）冲突时以后来的为准。\n")
        for word, before, _after, _why in sorted(recovered):
            fh.write(json.dumps({"word": word, "pinyin": before.split()}, ensure_ascii=False) + "\n")
    with args.manual.open("w", encoding="utf-8") as fh:
        fh.write("# 回收不了的多音字读音（旧库与新库不一致，两条条件都不满足）\n")
        fh.write("# 词\t旧读音\t新读音\t为什么不收\n")
        for word, before, after, why in sorted(manual):
            fh.write(f"{word}\t{before}\t{after}\t{why}\n")

    print(f"两库共有的词 {len(set(old) & set(new))}：读音一致 {same}，不一致 {len(recovered) + len(manual)}")
    print(f"  回收 {len(recovered)} 条 → {args.out}")
    print(f"  回收不了 {len(manual)} 条 → {args.manual}")
    print(f"\n回收样本（前 {args.samples}）：")
    for word, before, after, why in sorted(recovered)[: args.samples]:
        print(f"  {word}\t{after} → {before}\t{why}")
    print(f"\n回收不了的样本（前 {args.samples}）：")
    for word, before, after, why in sorted(manual)[: args.samples]:
        print(f"  {word}\t{after} → {before}\t{why}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
