# 句级混合模拟：每句的候选 = 各权重下的首选；每条按两套 LM 的路径概率 log(π·Pd + (1−π)·Pp) 挑最好的。π 只在 dev 上定。
import json, subprocess, sys, math, collections
sys.path.insert(0, '/Users/liyuqing/sproot/qingjian-dict-hunt/.lab/tools')
import trainer
WS = ('0.0', '0.2', '0.4', '0.6', '0.8', '1.0')
SETS = ('ddev', 'pdev', 'ext', 'doc', 'dho', 'pho')
L = {(n, w): [json.loads(l) for l in open(f'mix2/{n}-{w}.jsonl')] for n in SETS for w in WS}
header, rows, readings, freq = trainer.load_dict('libs/set-mixed/../set-mixed/dict.tsv') if False else trainer.load_dict('/Users/liyuqing/sproot/qingjian/data/sets/mixed/dict.tsv')
seg = trainer.Segmenter(readings, freq)
cands = {}
queries = set()
for n in SETS:
    for i in range(len(L[(n, '0.0')])):
        r0 = L[(n, '0.0')][i]
        texts = list(dict.fromkeys(L[(n, w)][i]['top'] for w in WS if L[(n, w)][i]['top']))
        paths = []
        for t in texts:
            ws = seg.segment(t, r0['pinyin'])
            if ws:
                prs = [('', ws[0])] + list(zip(ws, ws[1:]))
                paths.append((t, prs))
                queries.update(prs)
        cands[(n, i)] = (r0['text'], paths)
qs = sorted(queries)
def probe(lm):
    out = subprocess.run(['../target/release/examples/lm_probe', f'libs/set-{lm}/lm.qj'],
                         input='\n'.join(f'{a}\t{b}' for a, b in qs) + '\n', capture_output=True, text=True).stdout.splitlines()[1:]
    d = {}
    for line in out:
        a, b, v = line.split('\t')
        d[(a, b)] = float(v) if v != '-' else -30.0
    return d
PD, PP = probe('dialog'), probe('prose')
def acc(n, pi):
    hit = 0; total = len(L[(n, '0.0')])
    for i in range(total):
        gold, paths = cands[(n, i)]
        if not paths:
            continue
        def score(prs):
            d = sum(PD[p] for p in prs); p = sum(PP[q] for q in prs)
            m = max(d, p)
            return m + math.log(pi * math.exp(d - m) + (1 - pi) * math.exp(p - m))
        best = max(paths, key=lambda tp: score(tp[1]))[0]
        hit += best == gold
    return hit / total * 100
grid = [x / 20 for x in range(1, 20)]
best = max(grid, key=lambda pi: (acc('ddev', pi) * len(L[('ddev', '0.0')]) + acc('pdev', pi) * len(L[('pdev', '0.0')])))
print('dev 选定 π =', best)
for n in SETS:
    oracle = sum(any(t == cands[(n, i)][0] for t, _ in cands[(n, i)][1]) for i in range(len(L[(n, '0.0')]))) / len(L[(n, '0.0')]) * 100
    print(f'{n:5s} 句级混合 {acc(n, best):5.1f}   （候选里有正确答案的上限 {oracle:5.1f}）')
