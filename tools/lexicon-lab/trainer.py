# 解码器在环的代价修正（实验版）：题目 = 句子 + 全拼 + 上文；每轮把修正量烘焙进 TSV 词库与语言模型，
# 用 qingjian-cli --eval-text 原样解码，按「正确切分 − 首选切分」的词差更新每个词的修正量（nat，加在 log 概率上）。
import json, math, os, subprocess, sys, collections, shutil, random, argparse

LAB = '/Users/liyuqing/sproot/qingjian-lexicon/.lab'
CLI = '/Users/liyuqing/sproot/qingjian-lexicon/target/release/qingjian-cli'
LM_SPEC = os.environ.get('QJ_LM', '')
BASE = os.environ.get('QJ_BASE', f'{LAB}/libs/base-single-w10')


def load_dict(path):
    readings = collections.defaultdict(set)
    freq = {}
    header = []
    rows = []
    for line in open(path, encoding='utf-8'):
        if line.startswith('#'):
            header.append(line)
            continue
        parts = line.rstrip('\n').split('\t')
        if len(parts) < 3:
            header.append(line)
            continue
        word, syl, f = parts[0], parts[1], int(parts[2])
        rows.append((word, syl, f))
        readings[word].add(syl.replace(' ', ''))
        freq[word] = max(freq.get(word, 0), f)
    return header, rows, readings, freq


class Segmenter:
    def __init__(self, readings, freq):
        self.readings = readings
        total = sum(freq.values())
        self.logf = {w: math.log((f + 1) / total) for w, f in freq.items()}
        self.maxlen = 8

    def segment(self, text, pinyin):
        """按读音对齐把 text 切成词库词（词频对数和最大）；切不了返回 None。"""
        n, m = len(text), len(pinyin)
        # 按字位置推进的 DP，状态 (字下标, 拼音下标)
        frontier = collections.defaultdict(dict)
        frontier[0][0] = (0.0, None)
        for i in range(n):
            for j, (score, back) in list(frontier[i].items()):
                for k in range(1, min(self.maxlen, n - i) + 1):
                    w = text[i:i + k]
                    rs = self.readings.get(w)
                    if not rs:
                        continue
                    for r in rs:
                        if pinyin.startswith(r, j):
                            s = score + self.logf[w]
                            cur = frontier[i + k].get(j + len(r))
                            if cur is None or s > cur[0]:
                                frontier[i + k][j + len(r)] = (s, (i, j, w))
        end = frontier[n].get(m)
        if end is None:
            return None
        words = []
        i, j = n, m
        while i > 0:
            s, back = frontier[i][j]
            pi, pj, w = back
            words.append(w)
            i, j = pi, pj
        return words[::-1]


def bake(delta, out_dir, header, rows, pairs=None):
    """修正量 δ（nat）：词库词频乘 e^δ（进词图与词级预选看它），语言模型那边写 delta.tsv，运行时加在 log 概率上。"""
    os.makedirs(out_dir, exist_ok=True)
    for f in os.listdir(BASE):
        if f in ('dict.qj', 'dict.tsv', 'lm-unigram.tsv', 'lm-bigram.tsv'):
            continue
        dst = os.path.join(out_dir, f)
        if not os.path.lexists(dst):
            os.symlink(os.path.join(BASE, f), dst)
    with open(f'{out_dir}/dict.tsv', 'w', encoding='utf-8') as o:
        o.writelines(header)
        for w, syl, f in rows:
            d = delta.get(w, 0.0)
            o.write(f'{w}\t{syl}\t{max(1, round(f * math.exp(d))) if d else f}\n')
    with open(f'{out_dir}/delta.tsv', 'w', encoding='utf-8') as o:
        for w, d in delta.items():
            o.write(f'{w}\t{d:.4f}\n')
    with open(f'{out_dir}/pair.tsv', 'w', encoding='utf-8') as o:
        for (a, b), d in (pairs or {}).items():
            o.write(f'{a}\t{b}\t{d:.4f}\n')


def decode(lib, questions, tag, shards=8):
    """questions: 三列 TSV 行；返回与之对应的首选文本列表。"""
    run = f'{LAB}/run-{lib}'
    os.makedirs(f'{run}/data', exist_ok=True)
    for name, target in (('data/generated', f'{LAB}/libs/{lib}'),
                         ('data/eval', '/Users/liyuqing/sproot/qingjian/data/eval'),
                         ('assets', '/Users/liyuqing/sproot/qingjian-lexicon/assets'),
                         ('apps', '/Users/liyuqing/sproot/qingjian-lexicon/apps')):
        p = f'{run}/{name}'
        if os.path.lexists(p):
            os.remove(p)
        os.symlink(target, p)
    work = f'{LAB}/work/{tag}'
    shutil.rmtree(work, ignore_errors=True)
    os.makedirs(work)
    size = (len(questions) + shards - 1) // shards
    procs = []
    for s in range(shards):
        part = questions[s * size:(s + 1) * size]
        if not part:
            continue
        qf = f'{work}/q{s}.tsv'
        open(qf, 'w', encoding='utf-8').writelines(part)
        env = dict(os.environ, QJ_LM=LM_SPEC, QJ_DELTA=f'{LAB}/libs/{lib}/delta.tsv', QJ_PAIR=f'{LAB}/libs/{lib}/pair.tsv')
        procs.append((s, subprocess.Popen(
            [CLI, '--config', '/tmp/eval-config.toml', '--eval-text', qf, '--eval-details', f'{work}/d{s}.jsonl', '--misses', '0'],
            cwd=run, env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)))
    for _, p in procs:
        p.wait()
    tops = []
    for s, _ in procs:
        for line in open(f'{work}/d{s}.jsonl', encoding='utf-8'):
            r = json.loads(line)
            tops.append((r['text'], r['pinyin'], r.get('top') or '', r.get('context') or ''))
    return tops


_SEG = {}


def seg_cache(seg, text, pinyin):
    key = (text, pinyin)
    if key not in _SEG:
        _SEG[key] = seg.segment(text, pinyin)
    return _SEG[key]


def bigrams(words, context):
    """相邻词对；没有上文时句首词的前词是 <s>，有上文时不知道引擎看到的前词，跳过首词那一对。"""
    out = [] if context else [('<s>', words[0])]
    out += list(zip(words, words[1:]))
    return out


def errors(seg, rows):
    """一轮解码结果 → (词差, 词出现数, 词对差, 词对出现数, 切不开的条数)。"""
    diff, seen, pdiff, pseen = (collections.Counter() for _ in range(4))
    skipped = 0
    for sentence, pinyin, top, context in rows:
        g = seg_cache(seg, sentence, pinyin)
        if g is None:
            skipped += 1
            continue
        gb = bigrams(g, context)
        for w in g:
            seen[w] += 1
        for b in gb:
            pseen[b] += 1
        if top == sentence:
            continue
        p = seg_cache(seg, top, pinyin) if top else None
        if p is None:
            skipped += 1
            continue
        gc, pc = collections.Counter(g), collections.Counter(p)
        for w, c in (gc - pc).items():
            diff[w] += c
        for w, c in (pc - gc).items():
            diff[w] -= c
            seen[w] += c
        gbc, pbc = collections.Counter(gb), collections.Counter(bigrams(p, context))
        for b, c in (gbc - pbc).items():
            pdiff[b] += c
        for b, c in (pbc - gbc).items():
            pdiff[b] -= c
            pseen[b] += c
    return diff, seen, pdiff, pseen, skipped


def register_of():
    """哪句是对话、哪句是书面：从两份源题目文件建表，用来把开发集分开报（产品只叠一套修正量）。"""
    table = {}
    for side in ('dialog', 'prose'):
        path = f'{LAB}/q/{side}-clean.tsv'
        if not os.path.exists(path):
            continue
        for line in open(path, encoding='utf-8'):
            table.setdefault(line.split('\t')[0].strip(), side)
    return table


def train(args):
    header, rows, readings, freq = load_dict(f'{BASE}/dict.tsv')
    seg = Segmenter(readings, freq)
    lines = open(args.questions, encoding='utf-8').readlines()
    random.Random(7).shuffle(lines)
    ndev = max(1, len(lines) // 10)
    dev, train_q = lines[:ndev], lines[ndev:]
    REGISTER = register_of()
    delta, total, steps = {}, collections.Counter(), 0
    pairs, ptotal = {}, collections.Counter()
    best, best_epoch, best_avg = -1.0, -1, {}
    log = open(f'{LAB}/train-{args.name}.log', 'a')
    acc = lambda rs: sum(r[2] == r[0] for r in rs) / max(1, len(rs))
    for epoch in range(args.epochs):
        cur = f'{args.name}-e{epoch}'
        bake(delta, f'{LAB}/libs/{cur}', header, rows, pairs)
        tr = decode(cur, train_q, cur)
        avg = {w: v / steps for w, v in total.items()} if steps else {}
        pavg = {b: v / steps for b, v in ptotal.items() if abs(v / steps) > 0.01} if steps else {}
        avg_lib = f'{args.name}-a{epoch}'
        bake(avg, f'{LAB}/libs/{avg_lib}', header, rows, pavg)
        dv = decode(avg_lib, dev, avg_lib)
        by_side = {}
        for side in ('dialog', 'prose'):
            part = [r for r in dv if REGISTER.get(r[1] and r[0].strip(), '?') == side]
            if part:
                by_side[side] = f'{side} {acc(part)*100:.2f}%({len(part)})'
        side_text = ' '.join(by_side.values())
        msg = (f'epoch {epoch} 训练首选(当前权重) {acc(tr)*100:.2f}% 开发首选(平均权重) {acc(dv)*100:.2f}%'
               f' [{side_text}] 修正词数 {len(avg)} 词对数 {len(pavg)}')
        print(msg, flush=True); log.write(msg + '\n'); log.flush()
        if acc(dv) > best:
            best, best_epoch, best_avg = acc(dv), epoch, avg
            json.dump(avg, open(f'{LAB}/delta-{args.name}-best.json', 'w'), ensure_ascii=False)
            json.dump([[a, b, v] for (a, b), v in pavg.items()], open(f'{LAB}/pair-{args.name}-best.json', 'w'), ensure_ascii=False)
        elif epoch - best_epoch >= args.patience:
            print(f'早停：开发集第 {best_epoch} 轮最好 {best*100:.2f}%', flush=True)
            break
        diff, seen, pdiff, pseen, skipped = errors(seg, tr)
        if args.unigram:
            for w, g in diff.items():
                step = args.lr * g / (seen[w] + args.k)
                delta[w] = max(-args.clip, min(args.clip, delta.get(w, 0.0) + step))
        for b, g in pdiff.items():
            step = args.plr * g / (pseen[b] + args.k)
            pairs[b] = max(-args.clip, min(args.clip, pairs.get(b, 0.0) + step))
        steps += 1
        for w, v in delta.items():
            total[w] += v
        for b, v in pairs.items():
            ptotal[b] += v
        print(f'  更新 {len(diff)} 个词、{len(pdiff)} 个词对，切不开 {skipped}', flush=True)
    print(f'最好：第 {best_epoch} 轮，开发首选 {best*100:.2f}%，库 {args.name}-a{best_epoch}', flush=True)


if __name__ == '__main__':
    ap = argparse.ArgumentParser()
    ap.add_argument('--questions', required=True)
    ap.add_argument('--name', required=True)
    ap.add_argument('--epochs', type=int, default=4)
    ap.add_argument('--lr', type=float, default=0.5)
    ap.add_argument('--k', type=float, default=3.0)
    ap.add_argument('--clip', type=float, default=2.0)
    ap.add_argument('--patience', type=int, default=2)
    ap.add_argument('--plr', type=float, default=1.0)
    ap.add_argument('--unigram', action='store_true')
    train(ap.parse_args())
