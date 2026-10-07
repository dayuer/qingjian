#!/usr/bin/env bash
# 在指定底座上冻结题目：句子 → 三列冻结题（读音按底座词库）→ 加词级题 → 注入敲错与简拼。
# 用法：freeze_questions.sh <底座名> <输入句子文件（一行一句，或三列只取第一列）> <输出名> [条数上限]
set -euo pipefail
REPO=/Users/liyuqing/sproot/qingjian-lexicon
LAB="$REPO/.lab"
BASE_NAME="$1"
INPUT="$2"
TAG="$3"
LIMIT="${4:-0}"

work="$LAB/work-$TAG"
rm -rf "$work"; mkdir -p "$work/data/generated/dicts"
cp "$LAB/libs/$BASE_NAME/dict.qj" "$LAB/libs/$BASE_NAME/lm.qj" "$work/data/generated/"
cp "$REPO"/data/generated/glossary-en.qj "$REPO"/data/generated/english.qj "$work/data/generated/" 2>/dev/null || true
cp "$REPO"/data/generated/dicts/idioms.qj "$work/data/generated/dicts/" 2>/dev/null || true

sentences="$LAB/q/$TAG-sentences.txt"
python3 - "$INPUT" "$LIMIT" > "$sentences" <<'PY'
import pathlib, sys
path, limit = pathlib.Path(sys.argv[1]), int(sys.argv[2])
out = 0
for line in path.read_text(encoding="utf-8").splitlines():
    text = line.split("\t")[0].strip()
    if text:
        print(text)
        out += 1
        if limit and out >= limit:
            break
PY
echo "[$(date +%H:%M:%S)] 句子 $(wc -l < "$sentences") 条 → 冻结读音"
(cd "$work" && "$REPO/target/release/qingjian-cli" --config "$REPO/tools/eval/offline.toml" \
  --eval-text "$sentences" --eval-save "$LAB/q/$TAG.tsv" --extra-dict "$work/data/generated/dicts/idioms.qj" \
  > "$LAB/q/$TAG-save.log" 2>&1)
echo "[$(date +%H:%M:%S)] 冻结题 $(wc -l < "$LAB/q/$TAG.tsv") 条 → 加词级题"
python3 "$REPO/tools/lexicon-lab/word_questions.py" "$LAB/q/$TAG.tsv" "$LAB/q/$TAG-wq.tsv" \
  > "$LAB/q/$TAG-wq.log" 2>&1
echo "[$(date +%H:%M:%S)] 词级题 $(wc -l < "$LAB/q/$TAG-wq.tsv") 条 → 注入敲错与简拼"
python3 "$REPO/tools/lexicon-lab/perturb.py" "$LAB/q/$TAG-wq.tsv" "$LAB/q/$TAG-pert.tsv" 0.15 0.10 \
  > "$LAB/q/$TAG-pert.log" 2>&1
echo "[$(date +%H:%M:%S)] 扰动后 $(wc -l < "$LAB/q/$TAG-pert.tsv") 条 → $LAB/q/$TAG-pert.tsv"
shasum -a 256 "$LAB/q/$TAG-pert.tsv" | cut -c1-16
