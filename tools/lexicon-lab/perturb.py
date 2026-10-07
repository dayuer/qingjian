# 给整句题注入敲错与简拼（期望答案不变）：一部分题改敲一处错（邻键替换 / 相邻互换 / 漏一个字母），
# 一部分把最后一个音节截成声母。用法：perturb.py <题目tsv> <输出tsv> [敲错比例] [简拼比例]
import sys, random
sys.path.insert(0, '/Users/liyuqing/sproot/qingjian-lexicon/tools/lexicon-lab')
import trainer

ROWS = ['qwertyuiop', 'asdfghjkl', 'zxcvbnm']
POS = {c: (r, i) for r, row in enumerate(ROWS) for i, c in enumerate(row)}


def neighbors(c):
    r, i = POS[c]
    out = []
    for dr, di in ((0, -1), (0, 1), (-1, 0), (-1, 1), (1, -1), (1, 0)):
        rr, ii = r + dr, i + di
        if 0 <= rr < 3 and 0 <= ii < len(ROWS[rr]):
            out.append(ROWS[rr][ii])
    return out


def typo(pinyin, rnd):
    i = rnd.randrange(len(pinyin))
    kind = rnd.random()
    if kind < 0.5:
        return pinyin[:i] + rnd.choice(neighbors(pinyin[i])) + pinyin[i + 1:]
    if kind < 0.8 and i + 1 < len(pinyin):
        return pinyin[:i] + pinyin[i + 1] + pinyin[i] + pinyin[i + 2:]
    return pinyin[:i] + pinyin[i + 1:]


INITIALS = ('zh', 'ch', 'sh')


def abbreviate_last(text, pinyin, seg):
    words = seg.segment(text, pinyin)
    if not words:
        return None
    last = words[-1]
    reading = next((r for r in trainer_spaced[last] if pinyin.endswith(r.replace(' ', ''))), None)
    if reading is None:
        return None
    syllable = reading.split(' ')[-1]
    initial = syllable[:2] if syllable.startswith(INITIALS) else syllable[0]
    if initial == syllable:
        return None
    return pinyin[: len(pinyin) - len(syllable)] + initial


header, rows, readings, freq = trainer.load_dict(f'{trainer.BASE}/dict.tsv')
trainer_spaced = {}
for w, syl, f in rows:
    trainer_spaced.setdefault(w, set()).add(syl)
seg = trainer.Segmenter(readings, freq)
p_typo = float(sys.argv[3]) if len(sys.argv) > 3 else 0.15
p_abbr = float(sys.argv[4]) if len(sys.argv) > 4 else 0.10
rnd = random.Random(23)
out = open(sys.argv[2], 'w', encoding='utf-8')
counts = {'原样': 0, '敲错': 0, '简拼': 0}
for line in open(sys.argv[1], encoding='utf-8'):
    parts = line.rstrip('\n').split('\t')
    text, pinyin = parts[0], parts[1]
    ctx = parts[2] if len(parts) > 2 else ''
    x = rnd.random()
    new = None
    if x < p_typo and len(pinyin) >= 4:
        new, kind = typo(pinyin, rnd), '敲错'
    elif x < p_typo + p_abbr:
        new, kind = abbreviate_last(text, pinyin, seg), '简拼'
    if new and new != pinyin:
        out.write(f'{text}\t{new}\t{ctx}\n')
        counts[kind] += 1
    else:
        out.write(line if line.endswith('\n') else line + '\n')
        counts['原样'] += 1
print(counts)
