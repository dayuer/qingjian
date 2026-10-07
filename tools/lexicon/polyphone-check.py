#!/usr/bin/env python3
"""多音字注音校验（对标报告 P0-3）：拿 Unihan 与 CC-CEDICT 交叉核基础词库里含多音字的词条。

三方：
- **词库**（`assets/lexicon/dict.tsv`，`词<Tab>音节<Tab>词频`）的读音；
- **Unihan**（`data/unihan/Unihan_Readings.txt`，取 kHanyuPinlu + kXHC1983 + kMandarin 并集）——
  判断「这个词的读音里，每个字是否都有这个读音」；
- **CC-CEDICT**（`--cedict` 给的 cedict_ts_out.txt）—— 同一个词条它标的读音。

只**报告**，不改任何读音：对不上的列进待审清单，人工过。用法：

  python3 tools/lexicon/polyphone-check.py --cedict /tmp/cedict.txt
"""
import argparse
import collections
import re
import unicodedata
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


def strip_tone(syllable: str) -> str:
    # CC-CEDICT 用数字标调（yi1），Unihan 用带调字母（yī）：两种都归成去调小写
    syllable = re.sub(r"\d", "", syllable)
    for a, b in [("ü", "v"), ("ǖ", "v"), ("ǘ", "v"), ("ǚ", "v"), ("ǜ", "v"), ("u:", "v")]:
        syllable = syllable.replace(a, b)
    decomposed = unicodedata.normalize("NFD", syllable)
    return "".join(c for c in decomposed if not unicodedata.combining(c)).lower()


def load_unihan(path: Path) -> dict[str, set[str]]:
    readings: dict[str, set[str]] = collections.defaultdict(set)
    for line in path.read_text(encoding="utf-8").splitlines():
        if line.startswith("#") or not line.strip():
            continue
        fields = line.split("\t")
        if len(fields) < 3 or fields[1] not in ("kHanyuPinlu", "kXHC1983", "kMandarin"):
            continue
        ch = chr(int(fields[0][2:], 16))
        for syllable in re.findall(r"[A-Za-züÀ-ɏ]+", fields[2]):
            readings[ch].add(strip_tone(syllable))
    return readings


def load_cedict(path: Path) -> dict[str, set[str]]:
    """词 → 读音串（空格分隔，去声调）；同一个词多条时都收下。"""
    out: dict[str, set[str]] = collections.defaultdict(set)
    if not path or not path.exists():
        return out
    for line in path.read_text(encoding="utf-8").splitlines():
        if line.startswith("#") or not line.strip():
            continue
        m = re.match(r"^(\S+)\s+(\S+)\s+\[([^\]]*)\]\s+/.*/\s*$", line)
        if not m:
            continue
        out[m.group(2)].add(" ".join(strip_tone(s) for s in m.group(3).split()))
    return out


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--dict", type=Path, default=ROOT / "assets/lexicon/dict.tsv")
    parser.add_argument("--unihan", type=Path, default=ROOT / "data/unihan/Unihan_Readings.txt")
    parser.add_argument("--cedict", type=Path, default=Path("/tmp/cedict.txt"))
    parser.add_argument("--out", type=Path, default=ROOT / "assets/lexicon/00_meta/polyphone-review.tsv")
    parser.add_argument("--samples", type=int, default=20)
    args = parser.parse_args()

    unihan = load_unihan(args.unihan)
    cedict = load_cedict(args.cedict)

    def unihan_accepts(word: str, syllables: list[str]) -> bool:
        if len(word) != len(syllables):
            return False
        return all(s in unihan.get(ch, ()) for ch, s in zip(word, syllables))

    total = 0            # 含多音字的词条
    with_cedict = 0      # 其中 CC-CEDICT 也有的
    unihan_bad = []      # 词库读音 Unihan 不认
    cedict_differs = []  # 与 CEDICT 不一致
    cedict_bad = []      # CEDICT 读音 Unihan 不认
    erhua = []           # 只是儿化音写法不同
    rows = []
    for line in args.dict.read_text(encoding="utf-8-sig").splitlines():
        if line.startswith("#") or not line.strip():
            continue
        fields = line.split("\t")
        if len(fields) < 2 or not fields[0].strip() or not fields[1].strip():
            continue
        word, pinyin = fields[0].strip(), fields[1].strip()
        syllables = pinyin.split()
        if len(word) != len(syllables):
            continue
        polyphonic = [ch for ch in word if len(unihan.get(ch, ())) > 1]
        if not polyphonic:
            continue
        total += 1
        issues = []
        if not unihan_accepts(word, syllables):
            issues.append("词库读音 Unihan 不认")
            unihan_bad.append((word, pinyin))
        cedict_readings = cedict.get(word, set())
        if cedict_readings:
            with_cedict += 1
            # 儿化音只是写法不同（词库把 儿 写成一个音节 er，CC-CEDICT 写作 r），不算读音分歧
            without_erhua = re.sub(r"\ber\b", "r", pinyin)
            if pinyin not in cedict_readings and without_erhua in cedict_readings:
                erhua.append((word, pinyin))
                issues.append("儿化音写法不同（CEDICT 把 er 写成 r），不是读音分歧")
            elif pinyin not in cedict_readings:
                issues.append("与 CC-CEDICT 不一致（CEDICT：" + " / ".join(sorted(cedict_readings)) + "）")
                cedict_differs.append((word, pinyin, sorted(cedict_readings)))
            if not any(unihan_accepts(word, r.split()) for r in cedict_readings):
                issues.append("CEDICT 读音 Unihan 也不认")
                cedict_bad.append((word, sorted(cedict_readings)))
        if issues:
            rows.append((word, pinyin, "；".join(issues), "".join(polyphonic)))

    args.out.parent.mkdir(parents=True, exist_ok=True)
    with args.out.open("w", encoding="utf-8") as fh:
        fh.write("# 多音字待审清单（对标报告 P0-3）：只报告，不改读音。\n")
        fh.write("# 三方 = 词库读音 / Unihan（kHanyuPinlu + kXHC1983 + kMandarin）/ CC-CEDICT。\n")
        fh.write("# 词\t词库读音\t问题\t多音字\n")
        for row in sorted(rows):
            fh.write("\t".join(row) + "\n")

    print(f"基础词库含多音字的词条：{total} 条（其中 CC-CEDICT 也收了的 {with_cedict} 条）")
    print(f"  词库读音 Unihan 不认：{len(unihan_bad)} 条")
    print(f"  与 CC-CEDICT 不一致：{len(cedict_differs)} 条（另有 {len(erhua)} 条只是儿化音写法不同，不算分歧）")
    print(f"  CEDICT 读音 Unihan 也不认：{len(cedict_bad)} 条")
    real = len(rows) - len(erhua)
    print(f"待审清单（{len(rows)} 条，其中真分歧 {real} 条、儿化写法 {len(erhua)} 条）→ {args.out}")
    for label, items in [("词库读音 Unihan 不认", unihan_bad), ("与 CC-CEDICT 不一致", cedict_differs), ("CEDICT 读音 Unihan 也不认", cedict_bad)]:
        if not items:
            continue
        print(f"\n{label}：前 {args.samples} 条")
        for item in items[: args.samples]:
            print("  " + "   ".join(str(x) for x in item))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
