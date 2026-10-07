# 从整句题（句子\t全拼\t上文）造词级题：每句抽一个 1–3 字的词，拼音只给这个词，上文 = 原上文 + 句中它前面的字。
import sys, random
sys.path.insert(0, '/Users/liyuqing/sproot/qingjian-lexicon/tools/lexicon-lab')
import trainer
header, rows, readings, freq = trainer.load_dict(f'{trainer.BASE}/dict.tsv')
seg = trainer.Segmenter(readings, freq)
rnd = random.Random(11)
out = open(sys.argv[2], 'w', encoding='utf-8')
n = 0
for line in open(sys.argv[1], encoding='utf-8'):
    parts = line.rstrip('\n').split('\t')
    text, pinyin = parts[0], parts[1]
    ctx = parts[2] if len(parts) > 2 else ''
    words = seg.segment(text, pinyin)
    if not words:
        continue
    # 词与它在拼音里的那一段一起切出来
    spans, i, j = [], 0, 0
    ok = True
    for w in words:
        r = next((r for r in readings[w] if pinyin.startswith(r, j)), None)
        if r is None:
            ok = False
            break
        spans.append((w, r, text[:i]))
        i += len(w); j += len(r)
    if not ok:
        continue
    pool = [s for s in spans if 1 <= len(s[0]) <= 3]
    if not pool:
        continue
    w, r, before = rnd.choice(pool)
    out.write(f'{w}\t{r}\t{(ctx + before)[-20:]}\n')
    n += 1
print(n)
