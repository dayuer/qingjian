# 注意：分语料不截断建的表含全部短语合成二元（约 108 万条，<s> 我的 这类高计数），合并后会挤掉真实二元，与直接按上限建的表不等价（外部 55.4 对 57.9）。只作记录，别再用。
# 按权重合并分语料的计数（不截断、min 1 建的表），过滤合并后计数 < 3，二元按计数取前 N。
# 用法：combine.py <输出前缀> <N1,N2,…> <目录:权重>...；输出 <前缀>-<N/10万>
import sys, os
out, Ns = sys.argv[1], [int(x) for x in sys.argv[2].split(',')]
srcs = [(a.rsplit(':', 1)[0], int(a.rsplit(':', 1)[1])) for a in sys.argv[3:]]
def read(path, k):
    d = {}
    for line in open(path, encoding='utf-8'):
        if line.startswith('#'): continue
        p = line.rstrip('\n').split('\t')
        d['\t'.join(p[:k])] = int(p[k])
    return d
uni, bi = {}, {}
for d, w in srcs:
    for key, c in read(f'{d}/lm-unigram.tsv', 1).items(): uni[key] = uni.get(key, 0) + c * w
    for key, c in read(f'{d}/lm-bigram.tsv', 2).items(): bi[key] = bi.get(key, 0) + c * w
allrows = sorted(((c, k) for k, c in bi.items() if c >= 3), key=lambda r: (-r[0], r[1]))
for N in Ns:
  rows = allrows[:N]; o = f'{out}-{N // 100000}'
  os.makedirs(o, exist_ok=True)
  with open(f'{o}/lm-unigram.tsv', 'w', encoding='utf-8') as f:
    f.write('# 词\t计数\n')
    for k, c in sorted(uni.items(), key=lambda kv: -kv[1]): f.write(f'{k}\t{c}\n')
  with open(f'{o}/lm-bigram.tsv', 'w', encoding='utf-8') as f:
    f.write('# 前词\t后词\t计数\n')
    for c, k in rows: f.write(f'{k}\t{c}\n')
  print(o, '一元', len(uni), '二元', len(rows), '最低计数', rows[-1][0], flush=True)
