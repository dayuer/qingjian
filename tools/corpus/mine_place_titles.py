#!/usr/bin/env python3
"""从维基条目标题里挖地标与机构名，补进 `assets/lexicon/03_domains/places.tsv`。

为什么要这一步：`places.tsv`（THUOCL）里没有「经济型」「会议室」「锦江之星」「毛主席纪念堂」这类
实体，外部对话集（订酒店、逛景点、找餐馆）里却天天出现。**不能照着评测集逐条加** —— 那是拿评测
调词库。系统性的做法是拿维基条目标题当清单（它天然就是地标与机构名），按后缀与在清洗后语料里的
出现次数筛，再与现有源表去重后追加，来源列写 `wikipedia-titles`，许可与署名见 `docs/notes/eval-data.md`。

用法：
  PYTHON=/tmp/corpus-venv/bin/python python3 tools/corpus/mine_place_titles.py \\
      --min-mentions 50 --limit 20000 --dry-run
"""

import argparse
import collections
import pathlib
import re
import sys

import pyarrow.parquet as pq
from opencc import OpenCC

HAN = re.compile(r"^[一-鿿]+$")
# 地标与机构的后缀（挑对输入法有用的：出行、订酒店、逛景点、去机构办事）
# 单字后缀（要求标题 ≥3 字，避开「斯塔」这类两字碎片）：店 / 路 / 街 是 2026-10-09 补的，
# 外部对话集里「四季民福烤鸭店」「北京亚运村店」这类「专名 + 行业后缀」缺的就是它们
SINGLE_CHAR_TAILS = ("寺", "庙", "塔", "桥", "湖", "山", "岛", "湾", "店", "路", "街")
TAILS = (
    "公园", "广场", "纪念馆", "纪念堂", "博物馆", "展览馆", "美术馆", "图书馆", "科技馆",
    "大酒店", "酒店", "饭店", "宾馆", "大厦", "大楼", "中心", "商场", "广场",
    "机场", "火车站", "高铁站", "地铁站", "码头", "客运站",
    "南站", "北站", "东站", "西站",
    "大学", "学院", "中学", "小学", "医院", "体育馆", "体育场", "游泳馆", "剧院", "影院",
    "古镇", "风景区", "度假区", "开发区",
) + SINGLE_CHAR_TAILS
ROOT = pathlib.Path(__file__).resolve().parents[2]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--parquet", default="data/corpus/zhwiki-20231101-zh-*.parquet")
    parser.add_argument("--corpus", type=pathlib.Path, default=ROOT / "data/corpus/zhwiki.txt")
    parser.add_argument("--places", type=pathlib.Path, default=ROOT / "assets/lexicon/03_domains/places.tsv")
    parser.add_argument("--common", type=pathlib.Path, default=ROOT / "assets/lexicon/02_common/modern_chinese_common_words.tsv",
                        help="已经是常用词的不要（大学 / 中心 / 半岛 这类通用名词不是机构名）")
    parser.add_argument("--min-mentions", type=int, default=50, help="在清洗后语料里至少出现多少次")
    parser.add_argument("--max-chars", type=int, default=8)
    parser.add_argument("--limit", type=int, default=20000)
    parser.add_argument("--dry-run", action="store_true")
    parser.add_argument("--report", type=pathlib.Path, help="把「收了多少、按哪条规则、样例」写到这里")
    parser.add_argument("--dump", type=pathlib.Path,
                        help="把**所有**候选与语料次数写到这里（`词\t次数`）；给了它就不按 --min-mentions 筛，"
                             "门槛留到 dev 上定")
    args = parser.parse_args()

    # 条目标题里繁简混杂（聯合颱風警報中心 / 孫中山），先统一成简体再筛
    converter = OpenCC("t2s")
    titles = set()
    for path in sorted(pathlib.Path(ROOT).glob(args.parquet)):
        table = pq.ParquetFile(path)
        for batch in table.iter_batches(columns=["title"], batch_size=4096):
            for row in batch.column("title").to_pylist():
                title = (row or "").strip()
                # 带括号消歧的（「日坛公园（北京）」这类）：去掉括号部分
                title = converter.convert(title.split("（")[0].split("(")[0].strip())
                if not title or not HAN.match(title) or not (2 <= len(title) <= args.max_chars):
                    continue
                tail = next((t for t in TAILS if title.endswith(t)), None)
                if tail is None:
                    continue
                if tail in SINGLE_CHAR_TAILS and len(title) < 3:
                    continue      # 「斯塔」这种两字译名碎片
                titles.add(title)
        print(f"  {path.name}：候选累计 {len(titles)}", file=sys.stderr)

    existing = set()
    for line in args.places.read_text(encoding="utf-8").splitlines():
        if line.strip() and not line.startswith("#"):
            existing.add(line.split("\t")[0])
    common = set()
    if args.common.exists():
        for line in args.common.read_text(encoding="utf-8-sig").splitlines():
            if line.strip() and not line.startswith("#"):
                common.add(line.split("\t")[0].strip())
    others = set()
    for path in (ROOT / "assets/lexicon/03_domains").glob("*.tsv"):
        if path.name == "places.tsv":
            continue
        for line in path.read_text(encoding="utf-8-sig").splitlines():
            if line.strip() and not line.startswith("#"):
                others.add(line.split("\t")[0].strip())
    generic = {"大酒店", "大饭店", "大酒店", "市中心", "文化中心", "购物中心", "人民医院", "省人民医院"}
    fresh = titles - existing - common - others - generic
    print(f"其中已是常用词的 {len(titles & common)} 条不收", file=sys.stderr)

    # 在清洗后的语料里数出现次数（当作文档频次）。只在「后缀」附近回溯查表，别对每行做子串枚举
    mentions: collections.Counter = collections.Counter()
    with args.corpus.open(encoding="utf-8") as fh:
        for line in fh:
            for tail in TAILS:
                start = 0
                while (hit := line.find(tail, start)) != -1:
                    for begin in range(max(0, hit - 8), hit + 1):
                        candidate = line[begin : hit + len(tail)]
                        if candidate in fresh:
                            mentions[candidate] += 1
                    start = hit + 1

    if args.dump:
        args.dump.write_text("".join(f"{w}\t{c}\n" for w, c in
                                    sorted(mentions.items(), key=lambda kv: (-kv[1], kv[0]))),
                             encoding="utf-8")
        print(f"候选 {len(mentions)} 条写到 {args.dump}", file=sys.stderr)
    # 前缀必须是现有的专名/地名/机构表里的条目（「青山」+「公路」可以，「一带」+「一路」不行）：
    # 路 / 街 / 店 这类后缀噪声最大，不卡前缀就会收进一堆恰好以它结尾的普通词（2026-10-09 审计要求）
    prefix = set(existing) | set(others)
    for extra in (ROOT / "assets/lexicon/brand.tsv", ROOT / "assets/lexicon/domain_words.tsv"):
        if extra.exists():
            for line in extra.read_text(encoding="utf-8-sig").splitlines():
                if line.strip() and not line.startswith("#"):
                    prefix.add(line.split("\t")[0].strip())
    def has_known_prefix(title: str) -> str | None:
        tail = next((t for t in TAILS if title.endswith(t)), None)
        if tail is None or len(title) <= len(tail):
            return None
        return title[: -len(tail)] if title[: -len(tail)] in prefix else None

    rejected = [t for t in mentions if has_known_prefix(t) is None]
    mentions = {t: c for t, c in mentions.items() if has_known_prefix(t) is not None}
    print(f"前缀不在专名表里的 {len(rejected)} 条不收（如 " + "、".join(rejected[:6]) + "）", file=sys.stderr)
    picked = [(title, count) for title, count in mentions.items() if count >= args.min_mentions]
    picked.sort(key=lambda kv: (-kv[1], kv[0]))
    picked = picked[: args.limit]
    print(
        f"标题候选 {len(titles)}、现有源表已有 {len(existing)}、新词 {len(fresh)}；"
        f"语料里 ≥ {args.min_mentions} 次的 {len(picked)} 条",
        file=sys.stderr,
    )
    print("样本：" + "、".join(f"{w}({c})" for w, c in picked[:12]), file=sys.stderr)

    lines = [
        "# 维基条目标题补地标与机构（tools/corpus/mine_place_titles.py）",
        "# 规则：标题 2–8 个汉字、去括号消歧、统一成简体、按后缀筛地标/建筑/机构/酒店/景区，",
        "#       排除已是常用词的、已在别处源表里的、通用名词；只收在清洗后语料里出现 ≥ "
        f"{args.min_mentions} 次的（df ≥ {args.min_mentions} 在 dev 上定）。",
        f"# 结果：候选 {len(titles)}、已收 {len(picked)}",
        "# 样例：",
    ]
    lines += [f"#   {w}	{c}" for w, c in picked[:20]]
    text = "\n".join(lines)
    if args.report:
        args.report.write_text(text + "\n", encoding="utf-8")
    if args.dry_run:
        print(text, file=sys.stderr)
        return 0
    with args.places.open("a", encoding="utf-8") as fh:
        fh.write("# 以下由 tools/corpus/mine_place_titles.py 从维基条目标题挖出（CC BY-SA 4.0），\n")
        fh.write("# 文档频次 = 在清洗后的维基语料里出现的行数。列：词条 拼音 排序号 文档频次 字表级别 来源\n")
        for title, count in picked:
            fh.write(f"{title}\t\t\t{count}\t\twikipedia-titles\n")
    print(f"已追加 {len(picked)} 条到 {args.places}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
