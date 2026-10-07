#!/usr/bin/env bash
# 全量冻结题目（对话 132k + 书面 90k），并做内部开发集与评测/回放的去重检查
set -euo pipefail
REPO=/Users/liyuqing/sproot/qingjian-lexicon
LAB="$REPO/.lab"
cd "$REPO"

bash "$LAB/freeze_questions.sh" base-single-w10 \
  /Users/liyuqing/sproot/qingjian-dict-hunt/.lab/q/dialog100k-new.tsv dialog 0
bash "$LAB/freeze_questions.sh" base-single-w10 \
  /Users/liyuqing/sproot/qingjian-dict-hunt/.lab/q/prose50k-m.tsv prose 0

echo "[$(date +%H:%M:%S)] 去重检查：造题句 vs 评测集 / 留出 / 外部 / 回放"
python3 - <<'PY'
import json, pathlib
lab = pathlib.Path("/Users/liyuqing/sproot/qingjian-lexicon/.lab")
eval_dir = pathlib.Path("/Users/liyuqing/sproot/qingjian/data/eval")

seen = set()
for name in ("sentences.tsv", "dialog-holdout-frozen.tsv", "prose-holdout-frozen.tsv", "external-frozen.tsv"):
    path = eval_dir / name
    if path.exists():
        for line in path.read_text(encoding="utf-8").splitlines():
            if line.strip() and not line.startswith("#"):
                seen.add(line.split("\t")[0].strip())
log = eval_dir / "input-log-2026-10-04.jsonl"
if log.exists():
    for line in log.read_text(encoding="utf-8").splitlines():
        if line.strip():
            try:
                entry = json.loads(line)
            except json.JSONDecodeError:
                continue
            text = entry.get("text")
            if isinstance(text, str) and text:
                seen.add(text)
print(f"评测 / 留出 / 外部 / 回放里的句子共 {len(seen)} 条")

for tag in ("dialog", "prose"):
    path = lab / "q" / f"{tag}-pert.tsv"
    total = overlap = 0
    kept = []
    for line in path.read_text(encoding="utf-8").splitlines():
        if not line.strip():
            continue
        total += 1
        text = line.split("\t")[0].strip()
        if text in seen:
            overlap += 1
            continue
        kept.append(line)
    out = lab / "q" / f"{tag}-clean.tsv"
    out.write_text("\n".join(kept) + "\n", encoding="utf-8")
    print(f"{tag}: {total} 条，与评测撞了 {overlap} 条 → {out.name} {len(kept)} 条")
PY
echo "[$(date +%H:%M:%S)] 全量冻结完成"
