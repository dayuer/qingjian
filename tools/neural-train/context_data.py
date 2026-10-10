#!/usr/bin/env python3
"""带上文训练数据：三种上文各占三分之一。

- **空上文**：保住原有能力（序列与原格式完全一致）。
- **同一句按词边界切开**：前段当上文、后段当要转换的输入 —— 对应聊天里分段上屏。
- **同一篇文档的上一句**：上一句当上文。

序列格式 `<eos> [上文 ≤32 字] 拼音 <sep> 汉字 <eos>`，文字与字母靠 token 自身区分，不新增 token。

**泄漏断言**：目标句 **与上文** 都不许命中对话 / 书面 / 外部三份冻结留出集与派生 dev 集 ——
逐句精确匹配（去空格后相等）必须为 0，失败直接 RuntimeError。
"""

import os
import pathlib
import random
import re

# 2026-10-10：三个 worktree 的 .lab 合并迁到主检出 data/archive/，脚本不再指 worktree 路径。
# 默认按**脚本所在仓库**（主检出）解析；在 worktree 里跑时用 QJ_REPO 指主检出。
REPO = pathlib.Path(os.environ.get("QJ_REPO") or pathlib.Path(__file__).resolve().parents[2])
ARCHIVE = pathlib.Path(os.environ.get("QJ_ARCHIVE") or REPO / "data/archive")
LAB = ARCHIVE / "neural-lab"            # 训练产物、模型、venv、验收日志
LEXICON_LAB = ARCHIVE / "lexicon-lab"   # 词库线的中间产物（prose-holdout-clean.tsv、base-cap100…）
DATA = REPO / "data"
EVAL = DATA / "eval"
CORPUS = DATA / "corpus"
CTX_MAX = 32              # 上文最多 32 字
CTX_DEV = pathlib.Path(__file__).resolve().parent / "context-dev.tsv"
CLEAN_PROSE = LEXICON_LAB / "mine/prose-holdout-clean.tsv"
LEAK_SETS = ("dialog-holdout-frozen.tsv", "prose-holdout-frozen.tsv", "external-frozen.tsv")


def norm(text: str) -> str:
    return "".join(ch for ch in text if ch.isalnum())


def leak_sentences() -> set[str]:
    """三份冻结留出集的句子（去空格规范化）+ 派生 dev 集的**目标与上文**。"""
    out = set()
    for name in LEAK_SETS:
        path = EVAL / name
        if path.exists():
            for line in path.open(encoding="utf-8"):
                if line.strip() and not line.startswith("#"):
                    out.add(norm(line.split("\t")[0]))
    if CTX_DEV.exists():
        for line in CTX_DEV.read_text(encoding="utf-8").splitlines():
            fields = line.split("\t")
            out.add(norm(fields[0]))                       # 上文也要进泄漏名单
            if len(fields) > 2:
                out.add(norm(fields[2]))
    return out


def assert_no_leak(samples: list[tuple[str, str, str]], forbidden: set[str]) -> None:
    """目标与上文都不许命中留出集；命中就 RuntimeError 并打出前几条。"""
    bad = [(ctx, target) for ctx, _pinyin, target in samples
           if norm(target) in forbidden or (ctx and norm(ctx) in forbidden)]
    if bad:
        raise RuntimeError(f"泄漏断言失败：{len(bad)} 条命中留出集，前三条 {bad[:3]}")


def segmented(text: str, readings: dict[str, str]) -> list[str] | None:
    """最长匹配分词；有字没读音就返回 None。"""
    words, index = [], 0
    while index < len(text):
        for size in (3, 2, 1):
            word = text[index : index + size]
            if word in readings:
                words.append(word)
                index += size
                break
        else:
            return None
    return words


def plain_pinyin(words: list[str], readings: dict[str, str]) -> str:
    """按词拼出全拼（与冻结题同一约定：清洗库读音、音节直接拼接）。"""
    return "".join(readings[word].replace(" ", "") for word in words)


def in_vocab(text: str, vocab: set[str]) -> bool:
    """整串字符都在字表里。`usable` 只看「有读音」，而读音表比 8180 字表宽——
    `皝` 这类罕见字有读音却没进字表，只查读音的话会在建流时才炸（2026-10-10 连炸两次）。"""
    return all(ch in vocab for ch in text)


def usable(text: str, readings: dict[str, str], low: int = 2, high: int = 24) -> list[str] | None:
    """够格当样本的句子：长度合适、逐字有读音。"""
    if not (low <= len(text) <= high):
        return None
    return segmented(text, readings)


def build_stream(vocab: dict[str, int], samples: list[tuple[str, str, str]], limit: int | None,
                 ctx_len: int = 128):
    """`(上文, 拼音, 汉字)` → `<eos> 上文 拼音 <sep> 汉字 <eos>` 的 token 流与权重。

    权重与 target 同位：`weights[i]` 管的是「由 piece[i-1] 预测 piece[i]」，
    所以第一个汉字（piece[len(上文)+len(拼音)+2]，由 `<sep>` 预测）之前全置 0 —— 上文与拼音部分不算 loss。"""
    import array
    import random
    import numpy as np

    rng = random.Random(20261011)
    rng.shuffle(samples)
    if limit:
        samples = samples[:limit]
    tokens, weights = array.array("i"), array.array("f")
    skipped, unknown = 0, 0
    for ctx, pinyin, text in samples:
        missing = {ch for ch in ctx + pinyin + text if ch not in vocab}
        if missing:
            unknown += 1
            continue
        piece = (
            [vocab["<eos>"]]
            + [vocab[ch] for ch in ctx]
            + [vocab[ch] for ch in pinyin]
            + [vocab["<sep>"]]
            + [vocab[ch] for ch in text]
            + [vocab["<eos>"]]
        )
        if len(piece) > ctx_len:
            skipped += 1
            continue
        tokens.extend(piece)
        head = len(ctx) + len(pinyin) + 2
        weights.extend([0.0] * head + [1.0] * (len(piece) - head))
    print(f"带上文训练流 {len(tokens):,} token；因超长跳过 {skipped:,} 条；因字表外字符跳过 {unknown:,} 条", flush=True)
    return np.frombuffer(tokens, dtype=np.int32), np.frombuffer(weights, dtype=np.float32)


SENT_END = re.compile(r"[。！？!?]")


def sentences_of(line: str) -> list[str]:
    """把一行切成句子：LCCC 去掉分词空格即可整句；维基一行是**段落**，按句末标点切开。

    同一条目里的「上一句」用行内相邻句 —— 语料文件没有条目标记，一行（段落）当条目用。"""
    text = line.replace(" ", "")
    return [s for s in (piece.strip() for piece in SENT_END.split(text)) if s]


CACHE = pathlib.Path(__file__).resolve().parent / ".cache-ctx-samples.tsv"


def build_context_samples(readings: dict[str, str], vocab: set[str], rng: random.Random,
                          per_kind: int, lccc_share: float = 0.5, report: bool = False
                          ) -> list[tuple[str, str, str]]:
    """三种上文各 `per_kind` 条，**每一种里 LCCC 对话与维基书面各占 `lccc_share`**。

    返回 `(上文, 拼音, 汉字)`；空上文的上下文是空串。
    配额按来源分别装（2026-10-10 修：原先三桶共用、LCCC 在前装满就 break → 实测 100% 对话、0% 书面）。"""
    key = f"v=3 k={per_kind} share={lccc_share} vocab={len(vocab)}"
    if CACHE.exists() and CACHE.read_text(encoding="utf-8").splitlines()[:1] == [f"# {key}"]:
        rows = [l.split("\t") for l in CACHE.read_text(encoding="utf-8").splitlines()[1:] if l.strip()]
        print(f"带上文样本从缓存读入 {len(rows):,} 条（{key}）", flush=True)
        return [(r[0], r[1], r[2]) for r in rows]
    quota = int(per_kind * lccc_share), per_kind - int(per_kind * lccc_share)
    pools = {tag: {"empty": [], "split": [], "previous": []} for tag in ("lccc", "zhwiki")}
    for name, tag in (("lccc-lm90.txt", "lccc"), ("zhwiki-lm90.txt", "zhwiki")):
        path = CORPUS / name
        if not path.exists():
            continue
        want = quota[0] if tag == "lccc" else quota[1]
        # 上一句**跨行保留**：维基一行是段落、段内可能只有一句，行内重置就凑不出「上一句」，
        # 书面那一桶实测只能到 45,807 条（配比 81.4% 对话）。语料没有条目标记，用相邻行当条目代理。
        previous_text = None
        for line in path.open(encoding="utf-8"):
            sentences = sentences_of(line)
            for text in sentences:
                words = usable(text, readings)
                if words is None:
                    previous_text = None
                    continue
                pinyin = plain_pinyin(words, readings)
                if not in_vocab(pinyin, vocab) or not in_vocab(text, vocab):
                    previous_text = None
                    continue
                pool = pools[tag]
                ok_ctx = lambda c: in_vocab(c, vocab)               # 上文也必须在字表里
                if len(pool["empty"]) < want:
                    pool["empty"].append(("", pinyin, text))
                if len(pool["split"]) < want and len(words) >= 3:
                    cut = rng.randrange(1, len(words))
                    head = words[:cut]
                    if len("".join(head)) <= CTX_MAX and ok_ctx("".join(head)):
                        tail_pinyin = plain_pinyin(words[cut:], readings)
                        if in_vocab(tail_pinyin, vocab):
                            pool["split"].append(("".join(head), tail_pinyin, "".join(words[cut:])))
                # 上一句当上文：超长就取**末 32 字**（规格本来就是「上文 ≤32 字」）——
                # 只收「前一句本身 ≤32 字」的话，维基段落里的短句对不够，
                # 书面那一桶只能凑到 45,807 条、配比 81.4% 对话，被 1:1 断言拦下（2026-10-10）
                if len(pool["previous"]) < want and previous_text and ok_ctx(previous_text[-CTX_MAX:]):
                    pool["previous"].append((previous_text[-CTX_MAX:], pinyin, text))
                previous_text = text
                if all(len(pool[k]) >= want for k in pool):
                    break
            if all(len(pools[t][k]) >= (quota[0] if t == "lccc" else quota[1])
                   for t in ("lccc", "zhwiki") for k in ("empty", "split", "previous")):
                break
    for kind, label in (("empty", "空上文"), ("split", "切开"), ("previous", "上一句")):
        a, b = len(pools["lccc"][kind]), len(pools["zhwiki"][kind])
        if report:
            share = a / max(a + b, 1)
            print(f"  {label}：对话 {a:,} / 书面 {b:,}（对话占 {share:.1%}）", flush=True)
        if a and b and abs(a / (a + b) - lccc_share) > 0.01:
            raise RuntimeError(f"配比断言失败：{label} 里对话占 {a / (a + b):.1%}，超出 {lccc_share:.0%}±1%")
    out = (pools["lccc"]["empty"] + pools["zhwiki"]["empty"]
           + pools["lccc"]["split"] + pools["zhwiki"]["split"]
           + pools["lccc"]["previous"] + pools["zhwiki"]["previous"])
    CACHE.write_text(f"# {key}\n" + "".join(f"{c}\t{p}\t{t}\n" for c, p, t in out), encoding="utf-8")
    return out


def derive_context_dev(readings: dict[str, str], seed: int = 20261010) -> int:
    """从**冻结留出集**派生带上文 dev：每句按词边界切开，写 `上文\\t后段拼音\\t后段汉字`。

    拼音按清洗库读音重算（句子被切开了，冻结题那一列是全句拼音，用不上）；两个模型读同一份文件。
    """
    rng = random.Random(seed)
    rows = []
    # 书面那一份用**干净子集**（子串命中语料的句子已剔除），其余两份用冻结留出集
    sources = [EVAL / "dialog-holdout-frozen.tsv", CLEAN_PROSE, EVAL / "external-frozen.tsv"]
    for path in sources:
        if not path.exists():
            continue
        lines = [l.split("\t")[0] for l in path.read_text(encoding="utf-8").splitlines()
                 if l.strip() and not l.startswith("#")]
        rng.shuffle(lines)
        for text in lines[:200]:                          # 每份留出集抽 200 句，共 600
            words = segmented(text, readings)
            if words is None or len(words) < 3 or len(text) > 24:
                continue
            cut = rng.randrange(1, len(words))
            head = words[:cut]
            if len("".join(head)) > CTX_MAX:
                continue
            tail_pinyin = plain_pinyin(words[cut:], readings)
            rows.append(("".join(head), tail_pinyin, "".join(words[cut:])))
    CTX_DEV.write_text("".join(f"{h}\t{p}\t{t}\n" for h, p, t in rows), encoding="utf-8")
    return len(rows)
