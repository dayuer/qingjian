#!/usr/bin/env python3
"""把多音字待审清单（`assets/lexicon/00_meta/polyphone-review.tsv`）分成「自动改」和「人工看」。

只出**结论文件**，不改词库：自动改的那批写成 `pinyin-corrections.jsonl`（与 `gloss-gen pinyin` 同格式，
交给 `lexicon --pinyin`），lexicon 会把建议读音当主读音、原读音降权保留（`DISPUTED_READING_DIVISOR`）；
拿不定主意的写进 `polyphone-manual.tsv`，带理由。

三类**不自动采纳**（审计定的）：
1. CC-CEDICT 标的是台湾读音或旧读 —— 对照 `taiwan-readings.tsv` 排除；
2. 地名的当地读音（乐亭 lao ting、六安 lu an 这类）—— 以大陆现行地名读音为准，单列；
3. 方向相反：词库读音是 Unihan 里的常用音、CC-CEDICT 给的是罕用音 —— 人工看。

用法：python3 tools/lexicon/polyphone-fix.py [--cedict /tmp/cedict.txt]
"""
import argparse
import collections
import json
import re
import unicodedata
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


def strip_tone(syllable: str) -> str:
    syllable = re.sub(r"\d", "", syllable)
    for a, b in [("ü", "v"), ("ǖ", "v"), ("ǘ", "v"), ("ǚ", "v"), ("ǜ", "v"), ("u:", "v")]:
        syllable = syllable.replace(a, b)
    decomposed = unicodedata.normalize("NFD", syllable)
    return "".join(c for c in decomposed if not unicodedata.combining(c)).lower()


def load_unihan(path: Path):
    """字 → {读音: 词频}（只有 kHanyuPinlu 带频次，其余按 0）。"""
    readings: dict[str, dict[str, int]] = collections.defaultdict(dict)
    for line in path.read_text(encoding="utf-8").splitlines():
        if line.startswith("#") or not line.strip():
            continue
        fields = line.split("\t")
        if len(fields) < 3 or fields[1] not in ("kHanyuPinlu", "kXHC1983", "kMandarin"):
            continue
        ch = chr(int(fields[0][2:], 16))
        for syllable, freq in re.findall(r"([A-Za-züÀ-ɏ]+)(?:\((\d+)\))?", fields[2]):
            if not syllable:
                continue
            key = strip_tone(syllable)
            readings[ch][key] = max(readings[ch].get(key, 0), int(freq) if freq else 0)
    return readings


def load_cedict(path: Path) -> dict[str, list[str]]:
    out: dict[str, list[str]] = collections.defaultdict(list)
    if not path or not path.exists():
        return out
    for line in path.read_text(encoding="utf-8").splitlines():
        if line.startswith("#") or not line.strip():
            continue
        m = re.match(r"^(\S+)\s+(\S+)\s+\[([^\]]*)\]\s+/.*/\s*$", line)
        if m:
            reading = " ".join(strip_tone(s) for s in m.group(3).split())
            if reading not in out[m.group(2)]:
                out[m.group(2)].append(reading)
    return out


def load_taiwan(path: Path) -> dict[str, set[str]]:
    """字或词 → 台湾读音集合。"""
    out: dict[str, set[str]] = collections.defaultdict(set)
    for line in path.read_text(encoding="utf-8").splitlines():
        if line.startswith("#") or not line.strip():
            continue
        fields = line.split("\t")
        if len(fields) >= 3:
            out[fields[0]].add(strip_tone(fields[2]))
    return out


def load_places(path: Path) -> set[str]:
    words = set()
    if not path.exists():
        return words
    for line in path.read_text(encoding="utf-8-sig").splitlines():
        if line.startswith("#") or not line.strip():
            continue
        word = line.split("\t")[0].strip()
        if word:
            words.add(word)
    return words


def unihan_accepts(unihan, word: str, syllables: list[str]) -> bool:
    return len(word) == len(syllables) and all(
        s in unihan.get(ch, {}) for ch, s in zip(word, syllables)
    )


def suggests_the_rare_reading(unihan, word: str, syllables: list[str]) -> bool:
    """建议读音里，每个多音字选的是不是 Unihan 里的罕用音（词库那个是常用音 → 方向相反）。"""
    for ch, s in zip(word, syllables):
        readings = unihan.get(ch, {})
        if len(readings) < 2:
            continue
        best = max(readings.values())
        if best == 0:
            continue
        if readings.get(s, 0) < best and readings.get(s, 0) <= best / 4:
            return True
    return False


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--review", type=Path, default=ROOT / "assets/lexicon/00_meta/polyphone-review.tsv")
    parser.add_argument("--unihan", type=Path, default=ROOT / "data/unihan/Unihan_Readings.txt")
    parser.add_argument("--cedict", type=Path, default=Path("/tmp/cedict.txt"))
    parser.add_argument("--taiwan", type=Path, default=ROOT / "assets/lexicon/00_meta/taiwan-readings.tsv")
    parser.add_argument("--places", type=Path, default=ROOT / "assets/lexicon/03_domains/places.tsv")
    parser.add_argument("--corrections", type=Path, default=ROOT / "assets/lexicon/00_meta/pinyin-corrections.jsonl")
    parser.add_argument("--manual", type=Path, default=ROOT / "assets/lexicon/00_meta/polyphone-manual.tsv")
    parser.add_argument("--samples", type=int, default=20)
    args = parser.parse_args()

    unihan = load_unihan(args.unihan)
    cedict = load_cedict(args.cedict)
    taiwan = load_taiwan(args.taiwan)
    places = load_places(args.places)

    auto, manual = [], []
    for line in args.review.read_text(encoding="utf-8").splitlines():
        if line.startswith("#") or not line.strip():
            continue
        fields = line.split("\t")
        if len(fields) < 3:
            continue
        word, current = fields[0].strip(), fields[1].strip()
        m = re.search(r"CEDICT：([^）]*)", fields[2])
        if not m:
            continue
        suggestions = [s.strip() for s in m.group(1).split(" / ") if s.strip()]
        suggestions = [s for s in suggestions if unihan_accepts(unihan, word, s.split())]
        if not suggestions:
            manual.append((word, current, "CC-CEDICT 的建议读音 Unihan 也不认（例如地名当地读音）", fields[2]))
            continue
        suggestion = suggestions[0]
        syllables = suggestion.split()
        # 1. 台湾读音 / 旧读
        taiwan_hit = [
            ch for ch, s in zip(word, syllables)
            if s in taiwan.get(ch, set()) and s not in (current.split() or [""])
        ] or [w for w in (word,) if suggestion in taiwan.get(w, set())]
        if taiwan_hit:
            manual.append((word, current, f"CC-CEDICT 给的是台湾读音（{'、'.join(taiwan_hit)}）", f"{current} → {suggestion}"))
            continue
        # 2. 地名的当地读音
        if word in places:
            manual.append((word, current, "地名：以大陆现行地名读音为准，人工定", f"{current} → {suggestion}"))
            continue
        # 3. 方向相反：建议的是罕用音
        if suggests_the_rare_reading(unihan, word, syllables):
            manual.append((word, current, "方向相反：CC-CEDICT 给的是罕用音，词库那个是常用音", f"{current} → {suggestion}"))
            continue
        auto.append((word, current, suggestion))

    with args.corrections.open("w", encoding="utf-8") as fh:
        # 与 gloss-gen pinyin 同格式：lexicon 会把建议当主读音、原读音降权保留
        for word, _current, suggestion in sorted(auto):
            fh.write(json.dumps({"word": word, "pinyin": suggestion.split()}, ensure_ascii=False) + "\n")
    with args.manual.open("w", encoding="utf-8") as fh:
        fh.write("# 多音字：人工待定（不自动改）。理由见第三列。\n")
        fh.write("# 词\t词库读音\t为什么人工看\tCC-CEDICT 的建议\n")
        for word, current, reason, suggestion in sorted(manual):
            fh.write(f"{word}\t{current}\t{reason}\t{suggestion}\n")

    print(f"待审 {len(auto) + len(manual)} 条 → 自动改 {len(auto)} 条、人工 {len(manual)} 条")
    print(f"  自动的写进 {args.corrections}（lexicon --pinyin 用）")
    print(f"  人工的写进 {args.manual}")
    by_reason = collections.Counter(reason.split("（")[0] for _, _, reason, _ in manual)
    for reason, count in by_reason.most_common():
        print(f"  人工 · {reason}：{count} 条")
    print(f"\n自动改的前 {args.samples} 条：")
    for word, current, suggestion in sorted(auto)[: args.samples]:
        print(f"  {word}\t{current} → {suggestion}")
    print(f"\n人工的前 {args.samples} 条：")
    for word, current, reason, _ in sorted(manual)[: args.samples]:
        print(f"  {word}\t{current}\t{reason}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
