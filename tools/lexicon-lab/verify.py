# 独立验算：不信 CLI 汇总行、不信别人报的数。
# 1. 评测集 sha256 与交接文档记录的指纹对上；
# 2. 每个 out-<标签>/ 的明细 JSONL 自己数首选命中（候选第一条 == text，与 CLI 同一口径），与 results.tsv 的数比；
# 3. 回放明细自己数 rank==1 的词 / 整句；
# 4. 产物 lm.qj / dict.qj 的 sha256 现算。
import hashlib, json, glob, os, sys
E = '/Users/liyuqing/sproot/qingjian/data/eval'
EXPECT = {'sentences.tsv': '09c6941ee07f8839', 'external-frozen.tsv': 'c87aca25b6bc50a7',
          'dialog-holdout-frozen.tsv': '361d688b73829866', 'prose-holdout-frozen.tsv': '17c3c2dc4c8330ad',
          'input-log-2026-10-04.jsonl': 'bcaedd99df84f8b3'}
def sha(p, n=16):
    h = hashlib.sha256()
    with open(p, 'rb') as f:
        for b in iter(lambda: f.read(1 << 20), b''): h.update(b)
    return h.hexdigest()[:n]
bad = 0
for f, want in EXPECT.items():
    got = sha(f'{E}/{f}'); ok = got == want; bad += not ok
    print(f'{"OK " if ok else "BAD"} {f} {got}')
SETS = ['external-frozen', 'sentences', 'dialog-holdout-frozen', 'prose-holdout-frozen']
reported = {}
for line in open('results.tsv', encoding='utf-8'):
    p = line.rstrip('\n').split('\t'); reported[p[0]] = p
for tag, p in reported.items():
    d = f'out-{tag}'
    if not os.path.isdir(d): continue
    mine = []
    for i, s in enumerate(SETS):
        rows = [json.loads(l) for l in open(f'{d}/ed-{s}.jsonl', encoding='utf-8')]
        nset = sum(1 for _ in open(f'{E}/{s}.tsv', encoding='utf-8'))
        hit = sum(bool(r['candidates']) and r['candidates'][0] == r['text'] for r in rows)
        acc = round(100 * hit / len(rows), 1)
        mine.append(acc)
        if abs(acc - float(p[1 + i])) > 0.05 or len(rows) != nset:
            print(f'BAD {tag} {s}: 明细 {hit}/{len(rows)}={acc} 题数 {nset} 汇总 {p[1+i]}'); bad += 1
    rd = [json.loads(l) for l in open(f'{d}/rd.jsonl', encoding='utf-8')]
    lib = os.path.realpath(f'run-{tag}/data/generated')
    print(f'{tag}: 验算 {mine}  汇总 {p[1:5]}  回放明细 {len(rd)} 条 rank1 {sum(r.get("rank")==1 for r in rd)}  '
          f'dict {sha(lib+"/dict.qj",8)} lm {sha(lib+"/lm.qj",8)}  记录 {p[7]} {p[8]}')
print('全部对上' if not bad else f'{bad} 处不符')
