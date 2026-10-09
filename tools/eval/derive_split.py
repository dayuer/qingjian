#!/usr/bin/env python3
"""从冻结的整句评测题派生「分段上屏」题：把一句在词边界处切开，前段并进上文（像聊天里同一条消息前半句已经上屏），只评后段。

冻结题不改，派生题另存；切分点用固定种子，同一份输入、同一个词库永远切成同一份。
切词按读音对齐（词库词、词频对数和最大），切不开的句子跳过。

用法：python3 tools/eval/derive_split.py <词库 dict.tsv> <冻结题 tsv> <输出 tsv> [种子]
"""

import collections
import hashlib
import math
import random
import sys


def load_readings(path):
    readings = collections.defaultdict(set)
    freq = {}
    for line in open(path, encoding="utf-8"):
        if line.startswith("#"):
            continue
        parts = line.rstrip("\n").split("\t")
        if len(parts) < 3:
            continue
        word, syllables, count = parts[0], parts[1], int(parts[2])
        readings[word].add(syllables.replace(" ", ""))
        freq[word] = max(freq.get(word, 0), count)
    total = sum(freq.values())
    logf = {w: math.log((f + 1) / total) for w, f in freq.items()}
    return readings, logf


def segment(text, pinyin, readings, logf, maxlen=8):
    """返回 [(词, 拼音起点)]，切不开返回 None。"""
    n, m = len(text), len(pinyin)
    frontier = collections.defaultdict(dict)
    frontier[0][0] = (0.0, None)
    for i in range(n):
        for j, (score, _) in list(frontier[i].items()):
            for k in range(1, min(maxlen, n - i) + 1):
                word = text[i:i + k]
                for reading in readings.get(word, ()):
                    if pinyin.startswith(reading, j):
                        s = score + logf[word]
                        cur = frontier[i + k].get(j + len(reading))
                        if cur is None or s > cur[0]:
                            frontier[i + k][j + len(reading)] = (s, (i, j, word))
    if m not in frontier[n]:
        return None
    out = []
    i, j = n, m
    while i > 0:
        _, (pi, pj, word) = frontier[i][j]
        out.append((word, pj))
        i, j = pi, pj
    return out[::-1]


def main():
    dict_path, src, dst = sys.argv[1:4]
    seed = int(sys.argv[4]) if len(sys.argv) > 4 else 20261009
    readings, logf = load_readings(dict_path)
    rnd = random.Random(seed)
    kept = skipped = 0
    with open(dst, "w", encoding="utf-8") as out:
        for line in open(src, encoding="utf-8"):
            parts = line.rstrip("\n").split("\t")
            text, pinyin = parts[0], parts[1]
            context = parts[2] if len(parts) > 2 else ""
            words = segment(text, pinyin, readings, logf)
            if not words or len(words) < 2:
                skipped += 1
                continue
            cut = rnd.randrange(1, len(words))
            head = "".join(w for w, _ in words[:cut])
            start = words[cut][1]
            out.write(f"{text[len(head):]}\t{pinyin[start:]}\t{context}{head}\n")
            kept += 1
    digest = hashlib.sha256(open(dst, "rb").read()).hexdigest()
    src_digest = hashlib.sha256(open(src, "rb").read()).hexdigest()
    print(f"{dst}: {kept} 题（跳过 {skipped}），sha256 {digest[:16]}，派生自 {src} sha256 {src_digest[:16]}，种子 {seed}")


if __name__ == "__main__":
    main()
