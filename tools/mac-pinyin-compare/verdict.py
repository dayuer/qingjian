#!/usr/bin/env python3
"""对照测试的判定：把我们首选、Mac 首选与原句三方对齐，出总表与「只有 Mac 对」的清单。

只有每句的判定与归类入库；Mac 的原始输出留在 `.lab/mac-compare/`，不入库、不用于训练。

用法：python3 tools/mac-pinyin-compare/verdict.py
"""

import json
import pathlib

HERE = pathlib.Path(__file__).resolve().parent
LAB = pathlib.Path("/Users/liyuqing/sproot/qingjian-neural/.lab/mac-compare")
SOURCES = ["对话留出", "书面留出", "外部集"]


def load_ours(path: pathlib.Path) -> dict[str, str]:
    """我们的首选：明细 JSONL 里每句的 `top`。"""
    out = {}
    for line in path.read_text(encoding="utf-8").splitlines():
        if line.strip():
            row = json.loads(line)
            if row.get("top"):
                out[row["text"]] = row["top"]
    return out


def main() -> int:
    tasks = [line.split("\t") for line in (HERE / "task-300.tsv").read_text(encoding="utf-8").splitlines() if line.strip()]
    meta = {text: label for text, label in
            (line.split("\t") for line in (HERE / "task-300.meta.tsv").read_text(encoding="utf-8").splitlines() if line.strip())}
    ours = load_ours(LAB / "ours.jsonl")
    mac = {expected: got for expected, _pinyin, got, *_ in
           (line.split("\t") for line in (LAB / "mac-raw.tsv").read_text(encoding="utf-8").splitlines() if line.strip())}

    counts = {source: {"n": 0, "ours": 0, "mac": 0, "mac_only": 0, "ours_only": 0, "both_wrong": 0} for source in SOURCES}
    mac_only_rows = []
    for text, pinyin, *_rest in tasks:
        source = meta.get(text, "?")
        got_ours, got_mac = ours.get(text, ""), mac.get(text, "")
        counts[source]["n"] += 1
        counts[source]["ours"] += got_ours == text
        counts[source]["mac"] += got_mac == text
        if got_mac == text and got_ours != text:
            counts[source]["mac_only"] += 1
            mac_only_rows.append((source, text, got_ours, got_mac, pinyin))
        elif got_ours == text and got_mac != text:
            counts[source]["ours_only"] += 1
        elif got_ours != text and got_mac != text:
            counts[source]["both_wrong"] += 1

    print(f"{'集合':8s} {'句数':>4} {'我们首选':>8} {'Mac 首选':>8} {'只有 Mac 对':>11} {'只有我们对':>10} {'都错':>5}")
    total = {key: 0 for key in ("n", "ours", "mac", "mac_only", "ours_only", "both_wrong")}
    for source in SOURCES:
        row = counts[source]
        for key in total:
            total[key] += row[key]
        print(f"{source:8s} {row['n']:>4} {row['ours']:>8} {row['mac']:>8} {row['mac_only']:>11} "
              f"{row['ours_only']:>10} {row['both_wrong']:>5}")
    print(f"{'合计':8s} {total['n']:>4} {total['ours']:>8} {total['mac']:>8} {total['mac_only']:>11} "
          f"{total['ours_only']:>10} {total['both_wrong']:>5}")

    out = LAB / "mac-only.tsv"
    out.write_text("".join(f"{s}\t{t}\t{o}\t{m}\t{p}\n" for s, t, o, m, p in mac_only_rows), encoding="utf-8")
    print(f"\n只有 Mac 对的 {len(mac_only_rows)} 句写到 {out}（人工归类用：集合 / 正确 / 我们的错 / Mac / 拼音）")
    for source, text, got_ours, _got_mac, _pinyin in mac_only_rows:
        print(f"  [{source}] 对：{text} ｜ 我们：{got_ours}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
