#!/usr/bin/env python3
"""输入画像：读青简的输入日志（input-log.jsonl），算打字速度、准确率、常用词，挖词库与排序的问题，给出优化建议。

  cloud/scripts/input-profile.py                                    # 读本机 ~/Library/Application Support/Qingjian/input-log.jsonl
  cloud/scripts/input-profile.py a.jsonl b.jsonl -o report.md       # 多份日志（比如服务器上 export-log 导出的各设备日志）
  cloud/scripts/input-profile.py --since 2026-10-01 --json          # 只看某天以后；--json 输出机器可读的指标

只用标准库。报告里有你打过的字，别随手发出去。字段含义见 apps/macos 的输入日志：
commit 的 ms 是从第一个键到上屏的毫秒数、index 是选了第几个候选、pages 是翻了几页；
retract 是上屏后删掉重选（text 错的、chosen 对的）；retype 是敲错重敲（before → after）。
"""

from __future__ import annotations

import argparse
import json
import math
import re
import statistics
import sys
from collections import Counter, defaultdict
from datetime import datetime
from pathlib import Path

DEFAULT_LOG = Path.home() / "Library/Application Support/Qingjian/input-log.jsonl"

# 两次上屏相隔超过这么久，就当中间停下来了，不算进「连续打字」的时间
ACTIVE_GAP_SECONDS = 60
# 选词位置达到这个下标（第 3 个及以后）才算「排序偏后」
DEEP_INDEX = 2
CJK = re.compile(r"[㐀-鿿]")
PINYIN_LIKE = re.compile(r"^[a-z']{4,}$")
# 键盘相邻键：重敲前后只差一个字母、且是相邻键，多半是手滑
QWERTY_ROWS = ["qwertyuiop", "asdfghjkl", "zxcvbnm"]


def neighbors() -> dict[str, set[str]]:
    pos = {c: (r, i) for r, row in enumerate(QWERTY_ROWS) for i, c in enumerate(row)}
    near: dict[str, set[str]] = defaultdict(set)
    for a, (ra, ia) in pos.items():
        for b, (rb, ib) in pos.items():
            if a != b and abs(ra - rb) <= 1 and abs(ia - ib) <= 1:
                near[a].add(b)
    return near


NEAR = neighbors()


def load(paths: list[Path], since: str | None) -> list[dict]:
    rows = []
    for path in paths:
        for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
            line = line.strip()
            if not line:
                continue
            try:
                row = json.loads(line)
            except json.JSONDecodeError:
                continue
            # export-log 的输出可能包了一层 {"seq","device","line"}
            if "line" in row and isinstance(row["line"], str):
                device = row.get("device")
                try:
                    row = json.loads(row["line"])
                except json.JSONDecodeError:
                    continue
                row["device"] = device
            if since and row.get("t", "") < since:
                continue
            rows.append(row)
    rows.sort(key=lambda r: r.get("t", ""))
    return rows


def ts(row: dict) -> datetime | None:
    try:
        return datetime.fromisoformat(row["t"])
    except (KeyError, ValueError):
        return None


def cjk_len(text: str) -> int:
    return len(CJK.findall(text))


def pct(part: float, whole: float) -> str:
    return "—" if not whole else f"{100 * part / whole:.1f}%"


def quantile(values: list[float], q: float) -> float:
    if not values:
        return 0.0
    values = sorted(values)
    k = (len(values) - 1) * q
    lo, hi = math.floor(k), math.ceil(k)
    return values[lo] + (values[hi] - values[lo]) * (k - lo)


def syllables(row: dict) -> int:
    pinyin = row.get("pinyin") or ""
    return len([s for s in pinyin.split("'") if s]) or 1


def one_edit(a: str, b: str) -> tuple[str, str] | None:
    """a、b 只差一处（替换 / 多一个 / 少一个）时返回差的那对字母，供看手滑。"""
    if a == b:
        return None
    if len(a) == len(b):
        diff = [(x, y) for x, y in zip(a, b) if x != y]
        return diff[0] if len(diff) == 1 else None
    if abs(len(a) - len(b)) == 1:
        short, long_ = (a, b) if len(a) < len(b) else (b, a)
        for i in range(len(long_)):
            if long_[:i] + long_[i + 1 :] == short:
                return ("∅", long_[i]) if len(a) < len(b) else (long_[i], "∅")
    return None


def profile(rows: list[dict]) -> dict:
    commits = [r for r in rows if r.get("event") == "commit"]
    retracts = [r for r in rows if r.get("event") == "retract"]
    retypes = [r for r in rows if r.get("event") == "retype"]
    predictions = [r for r in rows if r.get("event") == "prediction"]
    chinese = [r for r in commits if cjk_len(r.get("text", "")) > 0]
    picked = [r for r in chinese if isinstance(r.get("index"), int)]

    # 速度：组字时（从第一个键到上屏）的键速与出字速度，以及连续打字时段的整体吞吐
    compose = [r for r in chinese if r.get("ms") and r.get("keys")]
    kpm = [60000 * len(r["keys"]) / r["ms"] for r in compose if r["ms"] > 0]
    cpm = [60000 * cjk_len(r["text"]) / r["ms"] for r in compose if r["ms"] > 0]
    keys_per_char = [len(r["keys"]) / cjk_len(r["text"]) for r in compose]
    keys_per_syllable = [len(r["keys"]) / syllables(r) for r in compose if r.get("pinyin")]
    active_seconds, chars_in_active = 0.0, 0
    previous = None
    for r in commits:
        t = ts(r)
        if t is None:
            continue
        if previous is not None:
            gap = (t - previous).total_seconds()
            if 0 <= gap <= ACTIVE_GAP_SECONDS:
                active_seconds += gap
                chars_in_active += cjk_len(r.get("text", "")) or len(r.get("text", "")) // 5
        previous = t

    # 准确率
    index_hist = Counter(r["index"] for r in picked)
    first_hit = index_hist.get(0, 0)
    paged = sum(1 for r in commits if (r.get("pages") or 0) > 0)
    corrected = sum(1 for r in chinese if r.get("corrected"))
    sources = Counter(r.get("source") for r in commits)

    # 常用：词、英文、应用、时段
    words = Counter(r["text"] for r in chinese)
    english = Counter(
        r["text"].lower()
        for r in commits
        if r.get("source") in ("english", "raw") and re.fullmatch(r"[A-Za-z][A-Za-z0-9._-]+", r.get("text", ""))
    )
    apps = Counter(r.get("app") or "?" for r in commits)
    hours = Counter(t.hour for t in (ts(r) for r in commits) if t)

    # 排序问题：同一个词总要往后翻才选到
    deep = defaultdict(list)
    for r in picked:
        if r["index"] >= DEEP_INDEX or (r.get("pages") or 0) > 0:
            deep[(r.get("pinyin") or r.get("keys"), r["text"])].append(r["index"] + 9 * (r.get("pages") or 0))
    ranking = sorted(
        ((k, len(v), statistics.mean(v)) for k, v in deep.items()),
        key=lambda x: (-x[1], -x[2]),
    )

    # 同音换字：上屏错了删掉重选
    confusions = Counter((r.get("text"), r.get("chosen")) for r in retracts if r.get("text") and r.get("chosen"))

    # 手滑：重敲前后只差一个字母
    slips = Counter()
    typo_pairs = Counter()
    for r in retypes:
        before, after = r.get("before", ""), r.get("after", "")
        typo_pairs[(before, after)] += 1
        edit = one_edit(before, after)
        if edit:
            slips[edit] += 1
    adjacent = sum(n for (a, b), n in slips.items() if b in NEAR.get(a, ()))

    # 缺词：拼音样的字母串原样上屏（打不出想要的词，只好上屏字母）
    raw_pinyin = Counter(
        r["text"]
        for r in commits
        if r.get("source") == "raw" and PINYIN_LIKE.fullmatch(r.get("text", "")) and r.get("top")
    )
    # 云端整句被采用、本地候选里没有：本地词库 / 整句模型没覆盖到的说法
    cloud_wins = Counter(
        r["text"] for r in commits if r.get("source", "").startswith("cloud") and r["text"] not in (r.get("top") or [])
    )

    # 自定义短语候选：反复整段打出来的长串
    long_phrases = Counter(r["text"] for r in chinese if cjk_len(r["text"]) >= 4)
    phrase_candidates = [(t, n) for t, n in long_phrases.most_common() if n >= 2][:15]

    # 双拼能省多少键：双拼每个音节固定 2 键
    mean_kps = statistics.mean(keys_per_syllable) if keys_per_syllable else 0
    shuangpin_saving = 1 - 2 / mean_kps if mean_kps > 2 else 0

    first, last = (ts(rows[0]), ts(rows[-1])) if rows else (None, None)
    return {
        "range": [rows[0]["t"], rows[-1]["t"]] if rows else [],
        "span_hours": round((last - first).total_seconds() / 3600, 2) if first and last else 0,
        "devices": sorted({r.get("device") for r in rows if r.get("device")}),
        "commits": len(commits),
        "chinese_commits": len(chinese),
        "chinese_chars": sum(cjk_len(r["text"]) for r in chinese),
        "speed": {
            "kpm_median": round(statistics.median(kpm), 1) if kpm else 0,
            "kpm_p90": round(quantile(kpm, 0.9), 1),
            "cpm_median": round(statistics.median(cpm), 1) if cpm else 0,
            "cpm_p90": round(quantile(cpm, 0.9), 1),
            "throughput_cpm": round(60 * chars_in_active / active_seconds, 1) if active_seconds else 0,
            "active_minutes": round(active_seconds / 60, 1),
            "keys_per_char": round(statistics.mean(keys_per_char), 2) if keys_per_char else 0,
            "keys_per_syllable": round(mean_kps, 2),
            "compose_ms_median": round(statistics.median([r["ms"] for r in compose])) if compose else 0,
        },
        "accuracy": {
            "first_hit": first_hit,
            "picked": len(picked),
            "index_hist": dict(sorted(index_hist.items())),
            "paged": paged,
            "corrected": corrected,
            "retracts": len(retracts),
            "retypes": len(retypes),
            "adjacent_slips": adjacent,
            "sources": dict(sources),
            "predictions": len(predictions),
        },
        "top_words": words.most_common(30),
        "top_english": english.most_common(15),
        "apps": apps.most_common(8),
        "hours": dict(sorted(hours.items())),
        "ranking_issues": [
            {"pinyin": k[0], "text": k[1], "times": n, "avg_position": round(p + 1, 1)} for k, n, p in ranking[:15]
        ],
        "confusions": [{"wrong": a, "right": b, "times": n} for (a, b), n in confusions.most_common(15)],
        "slips": [{"typed": a, "meant": b, "times": n} for (a, b), n in slips.most_common(10)],
        "typo_pairs": [{"before": a, "after": b, "times": n} for (a, b), n in typo_pairs.most_common(15)],
        "missing_words": [{"keys": k, "times": n} for k, n in raw_pinyin.most_common(15)],
        "cloud_only": [{"text": k, "times": n} for k, n in cloud_wins.most_common(15)],
        "phrase_candidates": [{"text": t, "times": n} for t, n in phrase_candidates],
        "shuangpin_saving": round(shuangpin_saving, 3),
    }


def advice(p: dict) -> list[str]:
    a = p["accuracy"]
    s = p["speed"]
    tips = []
    hit = a["first_hit"] / a["picked"] if a["picked"] else 1
    if a["picked"] and hit < 0.85:
        tips.append(
            f"首选命中率 {hit:.0%}，偏低：先看「排序问题」与「同音换字」两节，这些词青简会随你选越学越靠前；"
            "反复出现的可以加进自定义短语直接固定。"
        )
    if a["retracts"]:
        tips.append(
            f"上屏后删掉重选 {a['retracts']} 次：主要是同音词（见「同音换字」）。多打几个字再上屏（整句）通常比单词更准，"
            "因为上下文能帮着排除同音词。"
        )
    if s["keys_per_syllable"] > 2.6 and p["shuangpin_saving"] > 0.15:
        tips.append(
            f"每个音节平均敲 {s['keys_per_syllable']} 键；换双拼（每音节固定 2 键）理论上能少敲约 {p['shuangpin_saving']:.0%}。"
            "上手要一两周，适合打字量大、愿意练的时候再换。"
        )
    if a["adjacent_slips"] >= 2:
        tips.append(f"重敲里有 {a['adjacent_slips']} 次是相邻键手滑（见「手滑」）：纠错已在帮你兜，多数时候不用重敲，直接看候选。")
    if p["missing_words"]:
        tips.append("有拼音被原样上屏（见「疑似缺词」）：多半是词库里没有你要的词，可以加进用户词或自定义短语。")
    if p["cloud_only"]:
        tips.append("有些说法只有云联想给得出（见「云端补上的」）：这些是本地词库与整句模型的盲区，适合补进词库。")
    if p["phrase_candidates"]:
        tips.append("有长串反复整段打（见「自定义短语候选」）：设成短语后几个字母就出整句。")
    if not tips:
        tips.append("数据还少或各项都不错；攒几天再跑一次更准。")
    return tips


def render(p: dict) -> str:
    s, a = p["speed"], p["accuracy"]
    hit = a["first_hit"] / a["picked"] if a["picked"] else 0
    out = ["# 输入画像", ""]
    out.append(
        f"数据：{p['range'][0][:16]} → {p['range'][1][:16]}（跨 {p['span_hours']} 小时，连续打字约 {s['active_minutes']} 分钟）"
        if p["range"]
        else "数据：无"
    )
    if p["devices"]:
        out.append(f"设备：{'、'.join(p['devices'])}")
    out.append(f"上屏 {p['commits']} 次，其中中文 {p['chinese_commits']} 次、{p['chinese_chars']} 个汉字。")
    out += ["", "## 速度", ""]
    out += [
        "| 指标 | 数值 | 说明 |",
        "|---|---|---|",
        f"| 组字时键速 | 中位 {s['kpm_median']} 键/分，快的时候 {s['kpm_p90']} | 从按第一个键到上屏这段 |",
        f"| 组字时出字 | 中位 {s['cpm_median']} 字/分，快的时候 {s['cpm_p90']} | 同上，按汉字算 |",
        f"| 连续打字吞吐 | {s['throughput_cpm']} 字/分 | 含想、选词、标点等停顿（两次上屏间隔不超过 {ACTIVE_GAP_SECONDS} 秒才算连续） |",
        f"| 每个汉字敲键 | {s['keys_per_char']} 键 | 全拼一般 3～4 |",
        f"| 每个音节敲键 | {s['keys_per_syllable']} 键 | 双拼固定 2 |",
        f"| 一次组字耗时 | 中位 {s['compose_ms_median']} ms | |",
    ]
    out += ["", "## 准确率", ""]
    index_text = "、".join(f"第 {k + 1} 个 {v} 次" for k, v in a["index_hist"].items())
    out += [
        "| 指标 | 数值 |",
        "|---|---|",
        f"| 首选命中（选的就是第 1 个） | {pct(a['first_hit'], a['picked'])}（{a['first_hit']}/{a['picked']}） |",
        f"| 选词位置 | {index_text or '—'} |",
        f"| 翻页才找到 | {a['paged']} 次 |",
        f"| 上屏后删掉重选 | {a['retracts']} 次（{pct(a['retracts'], p['chinese_commits'])}） |",
        f"| 敲错重敲 | {a['retypes']} 次，其中相邻键手滑 {a['adjacent_slips']} 次 |",
        f"| 自动纠错帮上忙 | {a['corrected']} 次 |",
        f"| 上屏来源 | " + "、".join(f"{k} {v}" for k, v in sorted(a["sources"].items(), key=lambda x: -x[1])) + " |",
        f"| 云联想给出结果 | {a['predictions']} 次 |",
    ]
    out += ["", "来源说明：word 词、sentence 本地整句、raw 字母原样上屏、english 英文词、cloud_sentence 采用云端整句。"]
    out += ["", "## 常用", ""]
    out.append("**中文**：" + "、".join(f"{w}×{n}" for w, n in p["top_words"]) if p["top_words"] else "**中文**：—")
    out.append("")
    out.append("**英文**：" + "、".join(f"{w}×{n}" for w, n in p["top_english"]) if p["top_english"] else "**英文**：—")
    out.append("")
    out.append("**应用**：" + "、".join(f"{w}×{n}" for w, n in p["apps"]))
    out.append("")
    out.append("**时段**：" + "、".join(f"{h} 点 {n}" for h, n in p["hours"].items()))

    def table(title: str, note: str, rows: list[dict], cols: list[tuple[str, str]]):
        out.extend(["", f"## {title}", "", note, ""])
        if not rows:
            out.append("（暂无）")
            return
        out.append("| " + " | ".join(c[1] for c in cols) + " |")
        out.append("|" + "---|" * len(cols))
        for r in rows:
            out.append("| " + " | ".join(str(r[c[0]]) for c in cols) + " |")

    table("排序问题", f"同一个词总在第 {DEEP_INDEX + 1} 个以后或要翻页才选到。青简会随选择学习；反复出现的可设自定义短语。",
          p["ranking_issues"], [("pinyin", "拼音"), ("text", "想要的词"), ("times", "次数"), ("avg_position", "平均位置")])
    table("同音换字", "上屏后发现不对、删掉重选。", p["confusions"], [("wrong", "先上屏的"), ("right", "改成"), ("times", "次数")])
    table("手滑", "重敲前后只差一个字母；∅ 表示多敲或漏敲。", p["slips"], [("typed", "敲成"), ("meant", "本想"), ("times", "次数")])
    table("重敲", "敲错后删掉重敲的前后对照。", p["typo_pairs"], [("before", "先敲的"), ("after", "重敲的"), ("times", "次数")])
    table("疑似缺词", "像拼音的字母串被原样上屏：多半是词库没有你要的词。", p["missing_words"], [("keys", "字母"), ("times", "次数")])
    table("云端补上的", "采用了云端整句、而本地候选里没有：本地词库与整句模型的盲区。", p["cloud_only"], [("text", "内容"), ("times", "次数")])
    table("自定义短语候选", "4 个字以上、反复整段打出来的。", p["phrase_candidates"], [("text", "内容"), ("times", "次数")])
    out += ["", "## 建议", ""]
    out += [f"{i}. {t}" for i, t in enumerate(advice(p), 1)]
    out += ["", f"_样本 {p['commits']} 次上屏；少于几千次时各项比例波动大，攒几天再跑更准。_", ""]
    return "\n".join(out)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("logs", nargs="*", type=Path, help="输入日志；缺省读本机的")
    parser.add_argument("--since", help="只看这个时间以后，如 2026-10-01")
    parser.add_argument("--json", action="store_true", help="输出指标 JSON 而不是报告")
    parser.add_argument("-o", "--output", type=Path, help="写到文件；缺省打印")
    args = parser.parse_args()
    paths = args.logs or [DEFAULT_LOG]
    missing = [p for p in paths if not p.is_file()]
    if missing:
        sys.exit(f"找不到日志：{', '.join(map(str, missing))}")
    result = profile(load(paths, args.since))
    text = json.dumps(result, ensure_ascii=False, indent=2) if args.json else render(result)
    if args.output:
        args.output.write_text(text + "\n", encoding="utf-8")
        print(f"已写到 {args.output}")
    else:
        print(text)


if __name__ == "__main__":
    main()
