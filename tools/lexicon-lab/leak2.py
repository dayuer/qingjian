# 修正版：先去掉语料行里的空格（LCCC 按词空格分开），再按子串找尺子里的正确句（≥8 字），用 8 字前缀窗口加速。
import re, sys
E = '/Users/liyuqing/sproot/qingjian/data/eval'; C = '/Users/liyuqing/sproot/qingjian-lexicon/data/corpus'
sets = {}
for name in ['external-frozen', 'sentences', 'dialog-holdout-frozen', 'prose-holdout-frozen']:
    sets[name] = {re.sub(r'\s+', '', l.split('\t')[0]) for l in open(f'{E}/{name}.tsv', encoding='utf-8')}
    sets[name] = {t for t in sets[name] if len(t) >= 8}
byprefix = {}
for k in set().union(*sets.values()): byprefix.setdefault(k[:8], []).append(k)
for corpus in sys.argv[1:]:
    found = set()
    for line in open(f'{C}/{corpus}', encoding='utf-8'):
        s = line.replace(' ', '').rstrip('\n')
        for i in range(len(s) - 7):
            ks = byprefix.get(s[i:i + 8])
            if ks:
                for k in ks:
                    if s.startswith(k, i): found.add(k)
    print(corpus, {n: f'{len(k & found)}/{len(k)}' for n, k in sets.items()}, flush=True)
