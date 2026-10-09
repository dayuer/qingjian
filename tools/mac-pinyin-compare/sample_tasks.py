#!/usr/bin/env python3
"""抽对照测试的题目：从三份冻结留出集各取 100 句，句长 4–20 字，固定种子。

产物（入库）：
- `task-300.tsv`：`句子\t拼音\t前文`（前文留空，评测要空上文），可直接喂 `--eval-text`；
  顺序固定为 对话 100 / 书面 100 / 外部 100。
- `task-300.meta.tsv`：`句子\t来源`，写报告归类时用。

用法：python3 tools/mac-pinyin-compare/sample_tasks.py
"""

import pathlib
import random

SEED = 20261009
PER_SET = 100
MIN_LEN, MAX_LEN = 4, 20
EVAL = pathlib.Path("/Users/liyuqing/sproot/qingjian/data/eval")
OUT = pathlib.Path(__file__).resolve().parent
SOURCES = [
    ("对话留出", EVAL / "dialog-holdout-frozen.tsv"),
    ("书面留出", EVAL / "prose-holdout-frozen.tsv"),
    ("外部集", EVAL / "external-frozen.tsv"),
]


def main() -> int:
    rng = random.Random(SEED)
    picked, meta = [], []
    for label, path in SOURCES:
        rows = []
        for line in path.read_text(encoding="utf-8").splitlines():
            if not line.strip() or line.startswith("#"):
                continue
            fields = line.split("\t")
            if len(fields) >= 2 and MIN_LEN <= len(fields[0]) <= MAX_LEN:
                rows.append((fields[0], fields[1]))
        chosen = rng.sample(rows, PER_SET)
        picked.extend(chosen)
        meta.extend((text, label) for text, _ in chosen)
        print(f"{label}: 候选 {len(rows)}，抽 {len(chosen)}", flush=True)

    (OUT / "task-300.tsv").write_text(
        "".join(f"{text}\t{pinyin}\t\n" for text, pinyin in picked), encoding="utf-8"
    )
    (OUT / "task-300.meta.tsv").write_text(
        "".join(f"{text}\t{label}\n" for text, label in meta), encoding="utf-8"
    )
    print(f"写出 {OUT / 'task-300.tsv'}（种子 {SEED}）")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
