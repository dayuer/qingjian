#!/bin/bash
# 串行量延迟（不并发）：sentences.tsv 上融合 query_ms 与生成 generate_ms 的 p50 / p95，fp16 与 8 位同机同底座
R=${QJ_REPO:-$(cd "$(dirname "$0")/../.." && pwd)}   # 脚本所在仓库（主检出）
A=${QJ_ARCHIVE:-$R/data/archive}                  # 2026-10-10 三个 worktree 的 .lab 合并迁到这里
B=$R/target/release/qingjian-cli; Q=$A/dict-hunt-lab/quant; RUN=$Q/run-fp16
cd $RUN
for m in tongbian-f16.qjm tongbian-q8.qjm; do
  rm -f $Q/lat-fuse-$m.jsonl $Q/lat-gen-$m.jsonl
  $B --config /tmp/eval-config.toml --eval-text data/eval/sentences.tsv --neural $Q/$m --misses 0 --eval-details $Q/lat-fuse-$m.jsonl > /dev/null 2>&1
  $B --config /tmp/eval-config.toml --eval-text data/eval/sentences.tsv --eval-generate $Q/$m --misses 0 --eval-details $Q/lat-gen-$m.jsonl > /dev/null 2>&1
done
export Q
/tmp/q-venv/bin/python - <<'PY'
import json, os
Q=os.environ['Q']
def p(v,q): v=sorted(v); return v[min(len(v)-1,int(q*len(v)))]
for m in ('tongbian-f16.qjm','tongbian-q8.qjm'):
    f=[json.loads(l)['query_ms'] for l in open(f'{Q}/lat-fuse-{m}.jsonl')]
    g=[json.loads(l)['generate_ms'] for l in open(f'{Q}/lat-gen-{m}.jsonl')]
    print(m, f'融合 n={len(f)} p50 {p(f,.5):.1f} p95 {p(f,.95):.1f} ms', f'生成 n={len(g)} p50 {p(g,.5):.0f} p95 {p(g,.95):.0f} ms')
PY
