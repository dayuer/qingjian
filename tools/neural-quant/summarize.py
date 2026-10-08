# 从明细自己数（不读 CLI 汇总行）：融合首选 = candidates[0]==text，生成首选 = generated[0]==text；题数必须等于尺子行数；
# 回放 clean2 词 / 整句 rank==1 由 CLI 汇总行与明细两边核对。最后把各模型对 fp16 的差列出来，判据 int8 每项掉分 ≤ 0.5。
import json, re, sys, os
E = '/Users/liyuqing/sproot/qingjian/data/eval'
SETS = ['sentences', 'dialog-holdout-frozen', 'prose-holdout-frozen', 'external-frozen']
SHORT = ['sent', 'dialog', 'prose', 'ext']
def acc(path, key):
    rows = [json.loads(l) for l in open(path, encoding='utf-8') if l.strip()]
    hit = sum(bool(r.get(key)) and r[key][0] == r['text'] for r in rows)
    return hit, len(rows)
def cli_top1(path):
    m = re.search(r'首选 +([\d.]+)%', open(path, encoding='utf-8').read()); return float(m.group(1)) if m else None
def p(vals, q):
    v = sorted(vals); return v[min(len(v) - 1, int(q * len(v)))]
res = {}
for tag in sys.argv[1:]:
    O = f'out-{tag}'
    if not os.path.exists(f'{O}/DONE'): print(tag, '未跑完'); continue
    row = {}
    for s, k in zip(SETS, SHORT):
        n = sum(1 for _ in open(f'{E}/{s}.tsv', encoding='utf-8'))
        for path, key in (('fuse', 'candidates'), ('gen', 'generated')):
            h, t = acc(f'{O}/{path}-{s}.jsonl', key)
            assert t == n, f'{tag} {path} {s} 题数 {t} != {n}'
            mine = round(100 * h / t, 1); cli = cli_top1(f'{O}/{path}-{s}.txt')
            assert cli is not None and abs(mine - cli) < 0.051, f'{tag} {path} {s} 验算 {mine} 汇总 {cli}'
            row[f'{path}-{k}'] = mine
    gms = [json.loads(l)['generate_ms'] for s in SETS for l in open(f'{O}/gen-{s}.jsonl', encoding='utf-8')]
    row['gen p50/p95'] = f'{p(gms, .5):.0f}/{p(gms, .95):.0f}'
    rep = open(f'{O}/replay.txt', encoding='utf-8').read().split('干净口径 2')[1]
    w = re.search(r'词 +全部 (\d+)/\d+.*?前半 (\d+)/\d+.*?后半 (\d+)/\d+', rep); s2 = re.search(r'整句 +全部 (\d+)/\d+.*?前半 (\d+)/\d+.*?后半 (\d+)/\d+', rep)
    row['clean2 词'] = '/'.join(w.groups()); row['clean2 整句'] = '/'.join(s2.groups())
    row['指纹'] = open(f'{O}/fingerprint.txt').read().strip()
    res[tag] = row
    print(tag, row)
if 'fp16' in res:
    base = res['fp16']
    for tag, row in res.items():
        if tag == 'fp16': continue
        d = {k: round(row[k] - base[k], 1) for k in base if k.startswith(('fuse', 'gen-'))}
        print(f'{tag} 对 fp16：', d, '最大掉分', min(d.values()))
