# 相对熵剪枝（Stolcke 1998）到 N 条，按引擎的固定插值 P = λ·c(v,w)/c(v) + (1-λ)·p(w) 算：
# 去掉 (v,w) 后 P' = (1-λ)·p(w)，其它词在 v 后的概率不变（引擎不重归一），所以该条对相对熵的贡献就是
# P(v,w)·ln(P/P')，P(v,w) ≈ c(v,w)/N。用法：stolcke.py <源目录> <输出目录> [N] [λ]
import sys, math, os, shutil
src, dst = sys.argv[1], sys.argv[2]
N = int(sys.argv[3]) if len(sys.argv) > 3 else 5_000_000
lam = float(sys.argv[4]) if len(sys.argv) > 4 else 0.8
uni = {}
for line in open(f'{src}/lm-unigram.tsv', encoding='utf-8'):
    if line.startswith('#'): continue
    w, c = line.rstrip('\n').split('\t')[:2]; uni[w] = int(c)
total = sum(uni.values())
rows = []
head = None
for line in open(f'{src}/lm-bigram.tsv', encoding='utf-8'):
    if line.startswith('#'): head = line; continue
    v, w, c = line.rstrip('\n').split('\t'); c = int(c)
    cv, cw = uni.get(v, 0), uni.get(w, 0)
    if cv == 0 or cw == 0: continue
    pw = cw / total
    score = c * math.log((lam * c / cv + (1 - lam) * pw) / ((1 - lam) * pw))
    rows.append((score, line))
rows.sort(key=lambda r: -r[0])
os.makedirs(dst, exist_ok=True)
shutil.copy(f'{src}/lm-unigram.tsv', f'{dst}/lm-unigram.tsv')
with open(f'{dst}/lm-bigram.tsv', 'w', encoding='utf-8') as out:
    out.write(head)
    for _, line in rows[:N]: out.write(line)
kept = rows[:N]
print(f'候选 {len(rows)} 条，留 {len(kept)} 条，最低分 {kept[-1][0]:.4g}')
