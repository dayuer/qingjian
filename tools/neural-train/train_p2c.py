#!/usr/bin/env python3
"""拼音 → 汉字的小模型（≤ 20MB）：纯 decoder 因果 LM，格式与 crates/qingjian-neural 的 P2c 一致。

- 训练流：`<eos> [带噪拼音字母] <sep> [汉字] <eos>` 首尾相接；字母按字符当 token，与汉字共用字表。
  字表直接沿用通变的 `vocab.json`（`<pad> <unk> <eos> <sep>` 在前，字母、数字、汉字在后），
  id 全一致，`P2c::new` 才认得。
- 结构：5 层 / hidden 320 / 8 头 / context 128 / 输入输出共享嵌入，张量名与通变一致
  （`tok_emb` / `pos_emb` / `blocks.N.{ln1,ln2,attn.qkv,attn.proj,mlp.fc,mlp.proj}` / `ln_f`）。
- 前向复用 `align_check.py` 的实现（已与 Rust `P2c::convert` 对拍，偏差 < 4e-5）。
- 冒烟前先打印参数量与 fp16 导出字节数（> 20MB 直接退）。

用法：python3 tools/neural-train/train_p2c.py --smoke [--mask-after-sep]
      python3 tools/neural-train/train_p2c.py [--limit N] [--max-steps M] [--out 目录]   # 中等规模试点
"""

import argparse
import array
import hashlib
import json
import math
import os
import pathlib
import random
import re
import signal
import struct
import subprocess
import sys
import time
from typing import Callable

import numpy as np
import torch
import torch.nn.functional as F

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
from align_check import forward  # noqa: E402

# 2026-10-10：三个 worktree 的 .lab 合并迁到主检出 data/archive/，脚本不再指 worktree 路径。
# 默认按**脚本所在仓库**（主检出）解析；在 worktree 里跑时用 QJ_REPO 指主检出。
REPO = pathlib.Path(os.environ.get("QJ_REPO") or pathlib.Path(__file__).resolve().parents[2])
ARCHIVE = pathlib.Path(os.environ.get("QJ_ARCHIVE") or REPO / "data/archive")
LAB = ARCHIVE / "neural-lab"            # 训练产物、模型、venv、验收日志
LEXICON_LAB = ARCHIVE / "lexicon-lab"   # 词库线的中间产物（prose-holdout-clean.tsv、base-cap100…）
DATA = REPO / "data"
TB = LAB / "tongbian"
OUT = LAB / "neural"
CLI_CONFIG = LAB / "cli-config.toml"

N_LAYER, N_EMBD, N_HEAD, CTX = 5, 320, 8, 128      # 缺省是小模型；通变用 --layers/--embd/--heads 覆盖
TB_DIR = LAB / "tongbian"   # 通变 fp16 三件套（热启动起点）


def load_readings() -> dict[str, str]:
    """词库 TSV → 词 / 字读音，**保留音节间的空格**（`你好` → `ni hao`）：简拼噪声要按音节做。"""
    out: dict[str, str] = {}
    for path in (
        LEXICON_LAB / "libs/base-single-w10/dict.tsv",
        DATA / "generated/dict.tsv",
    ):
        if not path.exists():
            continue
        for line in path.open(encoding="utf-8"):
            if line.startswith("#") or not line.strip():
                continue
            fields = line.split("\t")
            if len(fields) >= 2 and fields[0] not in out and fields[1].strip():
                out[fields[0]] = fields[1].strip()
    return out


KEY_ROWS = ("qwertyuiop", "asdfghjkl", "zxcvbnm")
TWO_LETTER_INITIALS = ("zh", "ch", "sh")


def adjacent_keys() -> dict[str, str]:
    """QWERTY 每个键的相邻键：同行左右各一个，上下行各错开一位覆盖三个。错字噪声从这里挑。"""
    out: dict[str, str] = {}
    for row, keys in enumerate(KEY_ROWS):
        for col, key in enumerate(keys):
            near = set()
            for other, other_keys in enumerate(KEY_ROWS):
                if abs(other - row) > 1:
                    continue
                for other_col in (col - 1, col, col + 1):
                    if 0 <= other_col < len(other_keys) and (other, other_col) != (row, col):
                        near.add(other_keys[other_col])
            out[key] = "".join(sorted(near))
    return out


ADJACENT = adjacent_keys()


def initial(syllable: str) -> str:
    """音节的简拼写法：zh / ch / sh 取两个字母，其余取首字母（零声母音节也就取首字母）。"""
    for two in TWO_LETTER_INITIALS:
        if syllable.startswith(two):
            return two
    return syllable[:1]


def render_input(syllables: list[str], rng: random.Random) -> str:
    """按真实打字习惯造输入串：七成全拼、两成混合简拼（每音节一半几率换成声母）、一成全简拼；
    另约 3% 的句子随机敲错一个相邻键。"""
    roll = rng.random()
    if roll < 0.70:
        letters = "".join(syllables)
    elif roll < 0.90:
        letters = "".join(initial(s) if rng.random() < 0.5 else s for s in syllables)
    else:
        letters = "".join(initial(s) for s in syllables)
    if letters and rng.random() < 0.03:
        index = rng.randrange(len(letters))
        near = ADJACENT.get(letters[index], "")
        if near:
            letters = letters[:index] + rng.choice(near) + letters[index + 1 :]
    return letters


def to_pinyin(text: str, readings: dict[str, str], rng: random.Random) -> str | None:
    """整句转带噪拼音：最长匹配切词取出音节，再按 render_input 的规则拼成输入串。"""
    syllables: list[str] = []
    index = 0
    while index < len(text):
        for size in (3, 2, 1):
            word = text[index : index + size]
            if word in readings:
                syllables.extend(readings[word].split())
                index += size
                break
        else:
            return None
    return render_input(syllables, rng)


_SAMPLES: list[tuple[str, str]] | None = None
# 样本池指纹（配方 + 语料文件大小与时间）：写进检查点，续训时对不上就不续
POOL_FINGERPRINT = ""

# 配比（审计定的 500 万句）：对话 : 书面 = 1 : 1。配额按 1.6 倍抽 —— 句级去重要砍掉约三成，
# 不去重前抽 500 万、去重后只剩 350 万（2026-10-08 实测）
DIALOG_QUOTA, DIALOG_SHORT_QUOTA, PROSE_QUOTA = 4_000_000, 800_000, 4_000_000
POOL_TARGET = 5_000_000
# 对话上限 20 字：LCCC 里 13–20 字的整轮话占 11.75%，收到 12 字会把它们整段丢掉
MAX_LEN_DIALOG, MAX_LEN_PROSE = 20, 30
# 可产出行零产出率的闸门：实测正常值 LCCC 2.48% / 维基 6.23%（维基那部分是参考文献、繁体与 Latin），
# 静默丢源是 90% 以上；审计原定 5% 在这份语料上会对维基假警报，实测值已上报
DROP_RATE_LIMIT = 0.15


def run_pattern(vocab: set[str]) -> re.Pattern[str]:
    """字表内字符（汉字 / 字母 / 数字）的连续串：标点、繁体、拉丁一律当分隔符。

    整段语料里能当训练句的是这种**片段**，不是整行 —— 行里混着标点、繁体与拉丁
    （`數學，是研究數量…`），按整行筛会把绝大多数行判掉。"""
    body = "".join(
        ch for ch in vocab if len(ch) == 1 and not ch.isspace() and ch not in "<>[]"
    )
    return re.compile(f"[{re.escape(body)}]+")


class Reservoir:
    """蓄水池抽样：流式喂进来，均匀留 k 条，不把整份语料装进内存。"""

    def __init__(self, k: int, rng: random.Random) -> None:
        self.k, self.rng, self.seen, self.items = k, rng, 0, []

    def offer(self, item: tuple[str, str]) -> None:
        self.seen += 1
        if len(self.items) < self.k:
            self.items.append(item)
        else:
            slot = self.rng.randrange(self.seen)
            if slot < self.k:
                self.items[slot] = item


def sample_source(path: pathlib.Path, segmented: bool, vocab: set[str], readings: dict[str, str],
                  rng: random.Random, plans: list[tuple[str, int, int, int]]) -> tuple[dict[str, list[tuple[str, str]]], dict]:
    """一趟读一份语料，按 `plans`（名字、长度下限、上限、配额）分别蓄水池抽样，同时记丢弃率。

    `segmented` 为真时先去掉空格：LCCC 是**空格分词**的（`道歉 ！ ！ 再有 时间 找 你 去`），
    不去空格就只能抽出单词，抽不出句子。"""
    pattern = run_pattern(vocab)
    pools = {name: Reservoir(quota, rng) for name, _, _, quota in plans}
    low_all, high_all = min(p[1] for p in plans), max(p[2] for p in plans)
    lines = empty = convertible = 0
    pieces = 0
    empty_samples: list[str] = []
    for line in path.open(encoding="utf-8"):
        raw = line.strip()
        if not raw:
            continue
        lines += 1
        text = raw.replace(" ", "") if segmented else raw
        kept = False
        # 「本该产出」的行：含至少一条长度落在档位内的片段。整轮话比上限还长的不算 ——
        # 那是长度档位的取舍，不是管线漏了东西（2026-10-08 把两者混在一起，门槛假警报）
        worth = False
        for match in pattern.finditer(text):
            piece = match.group()
            length = len(piece)
            if not (low_all <= length <= high_all):
                continue
            worth = True
            pieces += 1
            for name, low, high, _ in plans:
                if low <= length <= high:
                    pinyin = to_pinyin(piece, readings, rng)
                    if pinyin and all(ch in vocab for ch in pinyin):
                        pools[name].offer((pinyin, piece))
                        kept = True
                    break
        if worth:
            convertible += 1
            if not kept:
                empty += 1
                if len(empty_samples) < 20:
                    empty_samples.append(raw[:40])
    return ({name: pools[name].items for name in pools},
            {"lines": lines, "worth": convertible, "empty": empty, "pieces": pieces,
             "samples": empty_samples})


def check_pool(name: str, items: list[tuple[str, str]], quota: int, min_long_share: float = 0.0) -> None:
    """池子构成门禁：抽到的句数要够配额，且长句占比不能塌。

    这是抓「源头被静默换成单词」的第二道网：2026-10-08 那次 LCCC 只剩 2–4 字的词
    （5 字以上只有 2000 条），零产出率却很低，只有看长度分布才看得出来。"""
    share = sum(1 for _, text in items if len(text) >= 5) / max(len(items), 1)
    print(f"  池子检查 {name}：{len(items):,} 句（配额 {quota:,}，{len(items) / max(quota, 1):.0%}），"
          f"5 字以上占 {share:.1%}", flush=True)
    if len(items) < quota * 0.9:
        raise SystemExit(f"池子门禁失败：{name} 只抽到 {len(items):,}，不足配额 {quota:,} 的九成")
    if share < min_long_share:
        raise SystemExit(f"池子门禁失败：{name} 5 字以上只占 {share:.1%}，低于 {min_long_share:.0%}（源头可能只剩单词）")


def check_drop_rate(name: str, stats: dict) -> None:
    """丢弃率门禁：**本可产出**（含长度落档的片段）的行里，一条都没转出来的占比 > 5% 就失败，打出样例。

    2026-10-08 那次 LCCC 九成多的行被静默丢掉（空格分词当成转不出拼音），是这类错的第一道网。"""
    rate = stats["empty"] / max(stats["worth"], 1)
    print(f"  丢弃率检查 {name}：行 {stats['lines']:,}（可产出 {stats['worth']:,}），"
          f"片段 {stats['pieces']:,}，可产出里零产出 {stats['empty']:,}（{rate:.2%}）", flush=True)
    if stats["samples"]:
        print("    零产出样例：", " | ".join(stats["samples"][:10]), flush=True)
    if rate > DROP_RATE_LIMIT:
        raise SystemExit(f"丢弃率闸门失败：{name} 可产出行的零产出 {rate:.2%} > {DROP_RATE_LIMIT:.0%}")


def load_samples() -> list[tuple[str, str]]:
    """训练句池：对话 / 书面按配额抽，句级去重。build_streams 与泄漏断言共用，缓存住。"""
    global _SAMPLES
    if _SAMPLES is not None:
        return _SAMPLES
    vocab = set(json.loads((TB / "vocab.json").read_text(encoding="utf-8"))["tokens"])
    readings = load_readings()

    dialog = DATA / "corpus/lccc-lm90.txt"
    prose = DATA / "corpus/zhwiki-lm90.txt"
    fingerprint = hashlib.sha256(
        "|".join(
            [f"vocab={len(vocab)}", f"dlg={DIALOG_QUOTA}", f"short={DIALOG_SHORT_QUOTA}",
             f"prose={PROSE_QUOTA}", f"max={MAX_LEN_DIALOG}/{MAX_LEN_PROSE}"]
            + [f"{p.name}:{p.stat().st_size}:{int(p.stat().st_mtime)}" for p in (dialog, prose) if p.exists()]
        ).encode()
    ).hexdigest()[:12]
    global POOL_FINGERPRINT
    POOL_FINGERPRINT = fingerprint
    cache = OUT / f"samples-{fingerprint}.tsv"
    if cache.exists():
        rows = [tuple(line.split("\t", 1)) for line in cache.read_text(encoding="utf-8").splitlines() if "\t" in line]
        _SAMPLES = [(p, t) for p, t in rows]
        print(f"样本池从缓存读入 {len(_SAMPLES):,} 句（指纹 {fingerprint}）", flush=True)
        return _SAMPLES

    rng = random.Random(20261011)
    dialog_pools, dialog_stats = sample_source(
        dialog, True, vocab, readings, rng,
        [("长句", 5, MAX_LEN_DIALOG, DIALOG_QUOTA - DIALOG_SHORT_QUOTA),
         ("短句", 2, 4, DIALOG_SHORT_QUOTA)],
    )
    prose_pools, prose_stats = sample_source(prose, False, vocab, readings, rng,
                                             [("书面", 2, MAX_LEN_PROSE, PROSE_QUOTA)])
    check_drop_rate("LCCC", dialog_stats)
    check_drop_rate("维基", prose_stats)
    dialog_all = dialog_pools["长句"] + dialog_pools["短句"]
    check_pool("对话", dialog_all, DIALOG_QUOTA, min_long_share=0.5)
    check_pool("书面", prose_pools["书面"], PROSE_QUOTA)
    tagged = ([("对话", pair) for pair in dialog_all] + [("书面", pair) for pair in prose_pools["书面"]])
    print(f"抽样：对话 {len(dialog_all):,}（5–20 字 {len(dialog_pools['长句']):,} + 2–4 字 {len(dialog_pools['短句']):,}）"
          f" + 书面 {len(prose_pools['书面']):,}", flush=True)

    # 去重前先洗牌：不然碰撞一律判给排在前的来源，比例会被压偏
    rng.shuffle(tagged)
    sampled = len(tagged)
    seen: set[str] = set()
    source_counts = {"对话": 0, "书面": 0}
    kept = 0
    for source, pair in tagged:  # 就地去重，不再开第二份列表（800 万条时会吃爆内存）
        if pair[1] in seen:
            continue
        seen.add(pair[1])
        tagged[kept] = pair
        kept += 1
        source_counts[source] += 1
    del tagged[kept:]
    del seen
    out: list[tuple[str, str]] = tagged
    print(f"句级去重后 {len(out):,} 句（去掉 {sampled - kept:,} 条重复）；"
          f"对话 {source_counts['对话']:,} / 书面 {source_counts['书面']:,}", flush=True)
    if len(out) > POOL_TARGET:
        rng.shuffle(out)
        del out[POOL_TARGET:]
        print(f"截到训练上限 {len(out):,} 句", flush=True)
    with cache.open("w", encoding="utf-8") as fh:
        for pinyin, text in out:
            fh.write(f"{pinyin}\t{text}\n")
    _SAMPLES = out
    print(f"样本池已落盘 {cache.name}", flush=True)
    return out


def build_streams(vocab: dict[str, int], limit: int | None, mask_after_sep: bool,
                  skip: Callable[[str], bool] | None = None):
    """把语料拼成训练流：每个样本 `<eos> 拼音 <sep> 汉字 <eos>`，逐 token 给损失权重。

    500 万句合起来上亿 token，用 `array` 存（int32 / float32），别用 Python list —— 那样光整数对象就吃掉几 GB。"""
    rng = random.Random(20261011)
    samples = [pair for pair in load_samples() if not skip or not skip(pair[1])]
    rng.shuffle(samples)
    if limit:
        samples = samples[:limit]
    tokens, weights = array.array("i"), array.array("f")
    skipped = 0
    for pinyin, text in samples:
        piece = (
            [vocab["<eos>"]]
            + [vocab[ch] for ch in pinyin]
            + [vocab["<sep>"]]
            + [vocab[ch] for ch in text]
            + [vocab["<eos>"]]
        )
        if len(piece) > CTX:
            skipped += 1
            continue
        tokens.extend(piece)
        if mask_after_sep:
            # 权重与 target 同一位移：weights[i] 管的是「由 piece[i-1] 预测 piece[i]」。
            # 第一个汉字是 piece[len(pinyin)+2]、由 piece[len(pinyin)+1]（<sep>）预测，故抹掉前 len(pinyin)+2 个权重
            head = len(pinyin) + 2
            weights.extend([0.0] * head + [1.0] * (len(piece) - head))
        else:
            weights.extend([1.0] * len(piece))
    print(f"训练流 {len(tokens):,} token；因超长跳过 {skipped:,} 句", flush=True)
    return (np.frombuffer(tokens, dtype=np.int32), np.frombuffer(weights, dtype=np.float32))


SEED_BASE = 20261008


def batches(tokens: np.ndarray, weights: np.ndarray, batch: int, epochs: int):
    """整条流切成长度 CTX 的窗口（numpy 视图，不复制），每批才把该批拷进 torch。

    每个 epoch 的乱序种子只由 `(SEED_BASE, epoch)` 决定，不靠 rng 递推 —— 续训时按步号
    直接落到同一位置，打乱顺序能精确复现。yield 出 `(epoch, 批内序号, 三件套)`。"""
    usable = ((tokens.shape[0] - 1) // CTX) * CTX
    x = tokens[0:usable].reshape(-1, CTX)
    y = tokens[1 : usable + 1].reshape(-1, CTX)
    w = weights[1 : usable + 1].reshape(-1, CTX)
    total = x.shape[0]
    for epoch in range(epochs):
        order = np.random.default_rng(SEED_BASE + epoch).permutation(total)
        for slot, start in enumerate(range(0, total - batch + 1, batch)):
            index = order[start : start + batch]
            yield epoch, slot, (torch.from_numpy(x[index].astype(np.int64)),
                                torch.from_numpy(y[index].astype(np.int64)),
                                torch.from_numpy(w[index].astype(np.float32)))


WARMUP_STEPS, LR_FLOOR = 1000, 0.1


def lr_at(step: int, total: int, base: float, warmup: int = WARMUP_STEPS, floor: float = LR_FLOOR) -> float:
    """线性预热 warmup 步 + 余弦衰减到 base×floor。纯函数：续训只看 step，不用存调度状态。"""
    if step < warmup:
        return base * (step + 1) / warmup
    progress = min((step - warmup) / max(total - warmup, 1), 1.0)
    return base * (floor + (1 - floor) * 0.5 * (1 + math.cos(math.pi * progress)))


def save_checkpoint(path: pathlib.Path, payload: dict) -> None:
    """先写 `.tmp` 并 fsync，再原子 `os.replace`；上一份留成 `.prev`，写坏了还能退回去。"""
    tmp = path.with_name(path.name + ".tmp")
    with tmp.open("wb") as fh:
        torch.save(payload, fh)
        fh.flush()
        os.fsync(fh.fileno())
    if path.exists():
        os.replace(path, path.with_name(path.stem + ".prev" + path.suffix))
    os.replace(tmp, path)


def load_checkpoint(path: pathlib.Path) -> dict | None:
    """读回检查点并自检（参数名与当前模型一致、步号合法、张量可读）；读不回来当没有。"""
    if not path.exists():
        return None
    try:
        payload = torch.load(path, map_location="cpu", weights_only=False)
        assert set(payload["params"]) == set(init_weights(1).keys()), "参数名与当前模型不符"
        assert int(payload["step"]) >= 0
        for tensor in payload["params"].values():
            assert torch.isfinite(tensor).all(), "参数里有非有限值"
        return payload
    except Exception as error:  # noqa: BLE001 —— 读不回来就退回上一份 / 从头开始
        print(f"检查点回读自检失败（{error}），按没有检查点处理", flush=True)
        previous = path.with_name(path.stem + ".prev" + path.suffix)
        if previous.exists():
            print(f"改用上一份 {previous.name}", flush=True)
            return load_checkpoint(previous) if previous != path else None
        return None


def init_weights(vocab_size: int, seed: int = 0) -> dict[str, torch.Tensor]:
    g = torch.Generator().manual_seed(seed)

    def param(*shape):
        return torch.randn(*shape, generator=g) * 0.02

    w = {
        "tok_emb.weight": param(vocab_size, N_EMBD),
        "pos_emb.weight": param(CTX, N_EMBD),
        "ln_f.weight": torch.ones(N_EMBD),
        "ln_f.bias": torch.zeros(N_EMBD),
    }
    for layer in range(N_LAYER):
        p = f"blocks.{layer}."
        w[p + "ln1.weight"] = torch.ones(N_EMBD)
        w[p + "ln1.bias"] = torch.zeros(N_EMBD)
        w[p + "ln2.weight"] = torch.ones(N_EMBD)
        w[p + "ln2.bias"] = torch.zeros(N_EMBD)
        w[p + "attn.qkv.weight"] = param(3 * N_EMBD, N_EMBD)
        w[p + "attn.qkv.bias"] = torch.zeros(3 * N_EMBD)
        w[p + "attn.proj.weight"] = param(N_EMBD, N_EMBD)
        w[p + "attn.proj.bias"] = torch.zeros(N_EMBD)
        w[p + "mlp.fc.weight"] = param(4 * N_EMBD, N_EMBD)
        w[p + "mlp.fc.bias"] = torch.zeros(4 * N_EMBD)
        w[p + "mlp.proj.weight"] = param(N_EMBD, 4 * N_EMBD)
        w[p + "mlp.proj.bias"] = torch.zeros(N_EMBD)
    return w


def warm_start(params: dict[str, torch.nn.Parameter], source: pathlib.Path) -> None:
    """热启动：把 `source`（三件套目录）里的 fp16 权重读进当前参数。

    张量名与结构必须完全一致（通变的 `tok_emb` / `pos_emb` / `blocks.N.*` / `ln_f`），
    对不上就报错退出 —— 微调最怕「以为加载了、其实随机初始化」。"""
    sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
    from align_check import load_safetensors  # noqa: PLC0415

    weight = source / "model.safetensors"
    loaded = load_safetensors(weight)
    missing = set(params) - set(loaded)
    extra = set(loaded) - set(params)
    if missing or extra:
        raise SystemExit(f"热启动权重对不上：缺 {sorted(missing)[:4]}，多 {sorted(extra)[:4]}")
    for name, tensor in loaded.items():
        if tuple(tensor.shape) != tuple(params[name].shape):
            raise SystemExit(f"{name} 形状不符：权重 {tuple(tensor.shape)} vs 模型 {tuple(params[name].shape)}")
        params[name].data.copy_(tensor)
    print(f"热启动：已从 {weight} 读入 {len(loaded)} 个张量", flush=True)


def trainable(w: dict[str, torch.Tensor]) -> dict[str, torch.nn.Parameter]:
    return {name: torch.nn.Parameter(tensor.clone()) for name, tensor in w.items()}


def verify_safetensors(tensors: dict[str, torch.Tensor], path: pathlib.Path) -> None:
    """写完立刻回读：段长与字节都要对上。candle 报的 `invalid offset for tensor` 就该在这里先炸。"""
    digest = lambda raw: hashlib.sha256(raw).digest()
    with path.open("rb") as fh:
        size = struct.unpack("<Q", fh.read(8))[0]
        header = json.loads(fh.read(size))
        base = 8 + size
        assert set(header) == set(tensors), "张量名不符"
        for name, tensor in tensors.items():
            start, end = header[name]["data_offsets"]
            expect = tensor.detach().to(torch.float16).cpu().contiguous().numpy().tobytes()
            assert end - start == len(expect), f"{name} 段长 {end - start} ≠ {len(expect)}"
            fh.seek(base + start)
            assert digest(fh.read(end - start)) == digest(expect), f"{name} 字节不符"


def save_safetensors(tensors: dict[str, torch.Tensor], path: pathlib.Path) -> None:
    # safetensors 要求每段数据按 8 字节对齐、头部也补齐到 8 的倍数：
    # 不补的话 candle 读出 `invalid offset for tensor`（2026-10-08 踩过）
    # 偏移量是相对「数据区起点」的（不是相对文件开头）：第一段从 0 开始，candle 才认
    header, offset, blobs = {}, 0, []
    for name, tensor in tensors.items():
        raw = tensor.detach().to(torch.float16).cpu().contiguous().numpy().tobytes()
        offset = (offset + 7) // 8 * 8
        header[name] = {
            "dtype": "F16",
            "shape": list(tensor.shape),
            "data_offsets": [offset, offset + len(raw)],
        }
        offset += len(raw)
        blobs.append((header[name]["data_offsets"][0], raw))
    body = json.dumps(header, ensure_ascii=False).encode()
    body += b" " * ((8 - len(body) % 8) % 8)
    with path.open("wb") as fh:
        fh.write(struct.pack("<Q", len(body)))
        fh.write(body)
        written = 0
        for offset, raw in blobs:
            fh.write(b"\0" * (offset - written))
            fh.write(raw)
            written = offset + len(raw)
    verify_safetensors(tensors, path)


def norm(text: str) -> str:
    return "".join(ch for ch in text if ch.isalnum())


def leak_filter() -> Callable[[str], bool]:
    """评测 / 留出 / dev / 外部 / 回放句子的判定函数：规范化精确撞，或共享 8 字片段。

    8 字片段这一层用「评测句的 8-gram 集合」查训练句，方向两边都盖得住（谁包含谁都能查出），
    是 O(训练句 × 几个 gram)，不是 O(训练句 × 评测句) —— 500 万句上后者跑不动。"""
    seen: set[str] = set()
    for name in (
        "sentences.tsv",
        "dialog-holdout-frozen.tsv",
        "prose-holdout-frozen.tsv",
        "external-frozen.tsv",
        "dialog-dev.tsv",
        "prose-dev.tsv",
    ):
        path = DATA / "eval" / name
        if path.exists():
            for line in path.open(encoding="utf-8"):
                if line.strip() and not line.startswith("#"):
                    seen.add(norm(line.split("\t")[0]))
    grams: set[str] = set()
    for text in seen:
        grams.update(text[i : i + 8] for i in range(len(text) - 7))
    print(f"泄漏基线：评测 / 留出 / dev 句 {len(seen):,}，8 字片段 {len(grams):,}", flush=True)

    def is_leaked(text: str) -> bool:
        n = norm(text)
        if n in seen:
            return True
        return len(n) >= 8 and any(n[i : i + 8] in grams for i in range(len(n) - 7))

    return is_leaked


def probe_three(model: pathlib.Path, dev: pathlib.Path) -> dict[str, float]:
    """用同一个贪心探针评三份集合：有上文 dev、空上文 dev、书面干净子集。

    返回 {'ctx': 有上文首选, 'plain': 空上文首选, 'clean': 书面干净子集空上文首选}（百分比）。
    """
    script = pathlib.Path(__file__).resolve().parent / "ctx_probe.py"
    clean = LEXICON_LAB / "mine/prose-holdout-clean.tsv"
    out = {}
    for key, path, extra in (("ctx", dev, ["--ctx-first"]), ("plain", dev, ["--ctx-first"]),
                             ("clean", clean, [])):
        result = subprocess.run(
            [sys.executable, str(script), str(model), str(path), *extra],
            capture_output=True, text=True, timeout=3600,
        )
        label = "有上文" if key == "ctx" else "空上文"
        value = None
        for line in result.stdout.splitlines():
            if line.strip().startswith(label):
                value = float(re.search(r"= ([\d.]+)%", line).group(1))
                break
        if value is None:
            raise RuntimeError(f"探针没读到 {label}：{result.stdout[-300:]}{result.stderr[-200:]}")
        out[key] = value
    return out


def write_model(target: pathlib.Path, params: dict, vocab_list: list[str],
                context_chars: int | None = None) -> None:
    """把权重与两件套写进 `target`（三件套目录，`CharScorer` / CLI 直接加载）。

    `context_chars` 只在**带前文训练**的模型上写：推理侧靠它决定要不要把光标前文喂进去。
    没这个字段（老通变）就是不带前文，推理侧一律不喂——喂了它读不懂反而掉分。"""
    assert context_chars is None or context_chars == CTX_MAX, f"上下文长度要等于 CTX_MAX({CTX_MAX})"
    target.mkdir(parents=True, exist_ok=True)
    save_safetensors({name: p.detach() for name, p in params.items()}, target / "model.safetensors")
    config = {"vocab_size": len(vocab_list), "n_layer": N_LAYER, "n_embd": N_EMBD,
              "n_head": N_HEAD, "context": CTX}
    if context_chars is not None:
        config["context_chars"] = context_chars
    (target / "config.json").write_text(json.dumps(config), encoding="utf-8")
    (target / "vocab.json").write_text(
        json.dumps({"tokens": vocab_list}, ensure_ascii=False), encoding="utf-8"
    )


def dev_top1(cli: pathlib.Path, config: pathlib.Path, repo: pathlib.Path, model: pathlib.Path,
             dev_files: list[pathlib.Path]) -> list[float]:
    """跑 CLI 的 P2C 自由生成评测，取每条 dev 文件的首选百分比（与产品同一套解码）。

    路径一律先 `resolve`：子进程的 cwd 是仓库根，相对路径会按仓库根去找（2026-10-08 踩过，
    全量跑到 14060 步时评测找不到 live 目录、整轮白跑）。"""
    args = [str(cli), "--config", str(config.resolve())]
    out = []
    for dev in dev_files:
        result = subprocess.run(
            args + ["--eval-text", str(dev.resolve()), "--eval-generate", str(model.resolve())],
            cwd=repo, capture_output=True, text=True, timeout=900,
        )
        match = re.search(r"首选\s+([\d.]+)%", result.stdout)
        if not match:
            # 抛普通异常，不能用 SystemExit —— 它继承 BaseException，调用方的 `except Exception`
            # 兜不住，CLI 一出问题整轮训练照样退出（2026-10-08 审计指出）
            raise RuntimeError(f"dev 评测没读到首选：{result.stdout[-400:]}{result.stderr[-400:]}")
        out.append(float(match.group(1)))
    return out


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--smoke", action="store_true")
    parser.add_argument("--epochs", type=int, default=6)
    parser.add_argument("--batch", type=int, default=32)
    parser.add_argument("--lr", type=float, default=3e-4)
    parser.add_argument("--mask-after-sep", action="store_true", help="只在 <sep> 之后算 loss")
    parser.add_argument("--limit", type=int, default=None, help="只用前 N 句；--smoke 默认 40000，否则全量")
    parser.add_argument("--max-steps", type=int, default=None, help="跑够 N 步就停（试点用）")
    parser.add_argument("--out", type=pathlib.Path, default=OUT, help="产物目录")
    parser.add_argument("--build-pool", action="store_true", help="只抽样建句子池并落盘缓存，不训练")
    parser.add_argument("--eval-every", type=int, default=0, help="每 N 步评测一次（0 = 不评）")
    parser.add_argument("--eval-dev", type=pathlib.Path, default=None, help="带上文 dev 文件")
    parser.add_argument("--eval-baseline-clean", type=float, default=36.1,
                        help="通变在书面干净子集上的空上文首选（36.1，贪心探针口径）")
    parser.add_argument("--eval-floor-delta", type=float, default=3.0,
                        help="书面干净子集低于基线这么多就停下")
    parser.add_argument("--context", action="store_true",
                        help="带上文微调：三种上文各 1/3（空 / 同句按词边界切开 / 上一句）")
    parser.add_argument("--per-kind", type=int, default=60000, help="每种上文抽多少条（--context 用）")
    parser.add_argument("--dev", type=pathlib.Path, nargs="*", default=[], help="dev 三列冻结题，按小时报首选")
    parser.add_argument("--dev-every", type=int, default=3600, help="dev 评测间隔（秒）")
    parser.add_argument("--cli", type=pathlib.Path, default=REPO / "target/release/qingjian-cli")
    parser.add_argument("--cli-config", type=pathlib.Path, default=CLI_CONFIG)
    parser.add_argument("--init-from", type=pathlib.Path, default=None,
                        help="热启动：从三件套目录读 fp16 权重（张量名与形状必须一致）")
    parser.add_argument("--device", default=None, help="cpu / mps，缺省有 MPS 用 MPS")
    parser.add_argument("--max-size-mb", type=float, default=20.0,
                        help="fp16 体积闸门（小模型 20；通变热启动后 44MB、要量化成 8 位，另有许可上限）")
    parser.add_argument("--layers", type=int, default=None, help="覆盖层数（通变 8）")
    parser.add_argument("--embd", type=int, default=None, help="覆盖 hidden（通变 448）")
    parser.add_argument("--heads", type=int, default=None, help="覆盖头数（通变 8）")
    parser.add_argument("--resume", action="store_true", help="从 out/ckpt.pt 精确续训（没有就从零开始）")
    parser.add_argument("--ckpt-every", type=int, default=900, help="检查点间隔（秒）")
    args = parser.parse_args()

    if args.build_pool:
        load_samples()
        return 0

    global N_LAYER, N_EMBD, N_HEAD
    N_LAYER = args.layers or N_LAYER
    N_EMBD = args.embd or N_EMBD
    N_HEAD = args.heads or N_HEAD
    device = torch.device(args.device or ("mps" if torch.backends.mps.is_available() else "cpu"))
    print(f"设备 {device}；结构 {N_LAYER} 层 / {N_EMBD} / {N_HEAD} 头 / context {CTX}", flush=True)

    out = args.out.resolve()
    limit = args.limit if args.limit is not None else (40000 if args.smoke else None)
    out.mkdir(parents=True, exist_ok=True)
    vocab_list = json.loads((TB / "vocab.json").read_text(encoding="utf-8"))["tokens"]
    vocab = {token: index for index, token in enumerate(vocab_list)}

    if args.context:
        import context_data  # noqa: PLC0415

        readings = load_readings()
        vocab_set = set(vocab_list)
        rng = random.Random(20261010)
        samples = context_data.build_context_samples(readings, vocab_set, rng, args.per_kind, report=True)
        forbidden = leak_filter()
        kept = [s for s in samples if not (forbidden(s[2]) or (s[0] and forbidden(s[0])))]
        dropped = len(samples) - len(kept)
        print(f"带上文样本 {len(samples):,} 条；剔除命中留出集的 {dropped:,} 条（{dropped/len(samples):.2%}）", flush=True)
        # 断言：剔完必须一条不剩（目标与上文都查）
        bad = sum(1 for ctx, _p, text in kept if forbidden(text) or (ctx and forbidden(ctx)))
        if bad:
            raise RuntimeError(f"泄漏断言失败：剔完仍有 {bad} 条命中留出集")
        samples = kept
        digest = hashlib.sha256()
        for ctx, pinyin, text in samples:
            digest.update(f"{ctx}\t{pinyin}\t{text}\n".encode())
        print(f"剔除后 {len(samples):,} 条；泄漏断言 0 ✓；训练集 sha256 {digest.hexdigest()[:16]}", flush=True)
        tokens, weights = context_data.build_stream(vocab, samples, limit, CTX)
        weights0 = init_weights(len(vocab_list))
        # 其余流程与空上文那条完全一致
        pool = samples
        leaked = 0
        is_leaked = None
    else:
        is_leaked = leak_filter()
        pool = load_samples()
    if not args.context:
        leaked = sum(1 for _, text in pool if is_leaked(text))
        print(f"泄漏断言：池 {len(pool):,} 句，与评测/留出/dev 撞 {leaked:,} 句 → 已从训练集剔除", flush=True)
        tokens, weights = build_streams(vocab, limit, args.mask_after_sep, skip=is_leaked if leaked else None)

    weights0 = init_weights(len(vocab_list))
    total = sum(t.numel() for t in weights0.values())
    probe = out / "probe.safetensors"
    save_safetensors(weights0, probe)
    size_mb = probe.stat().st_size / 1048576
    print(f"参数量 {total / 1e6:.2f} M；fp16 导出 {size_mb:.1f} MB", flush=True)
    if size_mb > args.max_size_mb:
        raise SystemExit(f"体积闸门失败：{size_mb:.1f} MB > {args.max_size_mb:.0f} MB")

    params = trainable(weights0)
    if args.init_from:
        warm_start(params, args.init_from)
    for tensor in params.values():
        tensor.data = tensor.data.to(device)
    optimizer = torch.optim.AdamW(params.values(), lr=args.lr, weight_decay=0.1)
    epochs = 1 if args.smoke else args.epochs
    stop = args.max_steps if args.max_steps is not None else (200 if args.smoke else None)
    per_epoch = max(1, ((len(tokens) - 1) // CTX) // args.batch)
    steps_total = max(1, per_epoch * epochs)

    ckpt, done_file = out / "ckpt.pt", out / "done"
    dev_log = out / "dev.log"
    losses: list[float] = []
    ctx_chars = CTX_MAX if args.context else None
    best, stale = -1.0, 0
    start_step = 0
    interrupted = False

    def checkpoint_payload(step: int) -> dict:
        return {
            "params": {name: p.detach().cpu() for name, p in params.items()},
            "optimizer": optimizer.state_dict(),
            "step": step,
            "losses": losses[-200:],
            "best": best,
            "stale": stale,
            "torch_rng": torch.get_rng_state(),
            "pool": POOL_FINGERPRINT,
        }

    def on_signal(signum, _frame):
        nonlocal interrupted
        interrupted = True
        print(f"收到信号 {signum}，本步结束后存检查点退出", flush=True)

    signal.signal(signal.SIGTERM, on_signal)
    signal.signal(signal.SIGINT, on_signal)

    if done_file.exists() and args.resume:
        print("已有完成标记 done，不重跑", flush=True)
        return 0
    if args.resume:
        payload = load_checkpoint(ckpt)
        if payload and payload.get("pool") not in (None, POOL_FINGERPRINT):
            print(f"检查点的样本池指纹 {payload['pool']} 与当前 {POOL_FINGERPRINT} 不符 → 不续训，从头开始",
                  flush=True)
            payload = None
        if payload:
            for name, tensor in payload["params"].items():
                params[name].data.copy_(tensor)
            optimizer.load_state_dict(payload["optimizer"])
            start_step, losses = int(payload["step"]), list(payload["losses"])
            best, stale = float(payload.get("best", -1.0)), int(payload.get("stale", 0))
            torch.set_rng_state(payload["torch_rng"])
            print(f"从检查点续训：step {start_step}（最好 dev {best:.1f}%，已连跌 {stale} 次）", flush=True)
        else:
            print("没有可用检查点，从头开始", flush=True)

    base_lr = args.lr
    nan_streak = 0
    next_ckpt = time.time() + args.ckpt_every
    next_dev = time.time() + args.dev_every
    for epoch, slot, (x, y, w) in batches(tokens, weights, args.batch, epochs):
        index = epoch * per_epoch + slot
        if index < start_step:
            continue
        base_lr_now = base_lr
        lr = lr_at(index, steps_total, base_lr_now)
        for group in optimizer.param_groups:
            group["lr"] = lr
        x, y, w = x.to(device), y.to(device), w.to(device)
        logits = forward(params, x, N_LAYER, N_HEAD)
        per_token = F.cross_entropy(
            logits.reshape(-1, len(vocab_list)), y.reshape(-1), reduction="none"
        )
        loss = (per_token * w.reshape(-1)).sum() / w.sum().clamp(min=1)
        if not torch.isfinite(loss):
            # 非有限就跳过这一步（不 step、不计 loss）；连 50 步就回滚到最近的检查点并把学习率减半。
            # 回滚只把权重退回去，批的位置退不回去（迭代器不能倒带），后面的数据照跑。
            nan_streak += 1
            optimizer.zero_grad()
            print(f"step {index} loss 非有限（连续 {nan_streak}）→ 跳过", flush=True)
            if nan_streak >= 50:
                payload = load_checkpoint(ckpt)
                if payload:
                    for name, tensor in payload["params"].items():
                        params[name].data.copy_(tensor)
                    optimizer.load_state_dict(payload["optimizer"])
                base_lr *= 0.5
                nan_streak = 0
                print(f"连续 50 步非有限 → 回滚到检查点，学习率减半到 {base_lr:.2e}", flush=True)
            continue
        nan_streak = 0
        optimizer.zero_grad()
        loss.backward()
        torch.nn.utils.clip_grad_norm_(params.values(), 1.0)
        optimizer.step()
        losses.append(float(loss.detach()))
        if index % 20 == 0:
            print(f"step {index} loss {float(loss.detach()):.3f} lr {lr:.2e}", flush=True)

        if time.time() >= next_ckpt:
            save_checkpoint(ckpt, checkpoint_payload(index))
            next_ckpt = time.time() + args.ckpt_every
        if interrupted:
            save_checkpoint(ckpt, checkpoint_payload(index))
            print(f"中断：已存检查点（step {index}）退出，外层会用 --resume 续上", flush=True)
            return 1

        if args.eval_every and args.eval_dev and index > 0 and index % args.eval_every == 0:
            live = out / "live"
            write_model(live, params, vocab_list, ctx_chars)
            save_checkpoint(out / f"ckpt-{index}.pt", checkpoint_payload(index))
            scores = probe_three(live, args.eval_dev)
            print(f"step {index} 有上文 {scores['ctx']:.1f}% / 空上文 {scores['plain']:.1f}% / "
                  f"书面干净 {scores['clean']:.1f}%（基线 {args.eval_baseline_clean:.1f}）", flush=True)
            if scores["clean"] < args.eval_baseline_clean - args.eval_floor_delta:
                save_checkpoint(out / "ckpt.pt", checkpoint_payload(index))
                write_model(out / "stopped", params, vocab_list, ctx_chars)
                print(f"书面干净子集比基线低 {args.eval_baseline_clean - scores['clean']:.1f} 点 → 停在这里", flush=True)
                return 1
        if args.dev and time.time() >= next_dev:
            # 每小时只往 dev.log 写一行：步数、滑动 loss、各 dev 文件的首选。
            # 监控归监控：评测自己出错不许把几小时的主训练一起带走（2026-10-08 相对路径那次就毁了 14060 步）
            try:
                live = out / "live"
                write_model(live, params, vocab_list, ctx_chars)
                tops = dev_top1(args.cli, args.cli_config, REPO, live, args.dev)
            except Exception as error:  # noqa: BLE001 —— 监控失败只记一行，接着训
                line = f"{time.strftime('%H:%M')} step {index} dev 评测失败：{error}"
                print(f"dev {line}", flush=True)
                with dev_log.open("a", encoding="utf-8") as fh:
                    fh.write(line + "\n")
            else:
                recent = sum(losses[-20:]) / len(losses[-20:])
                line = (f"{time.strftime('%H:%M')} step {index} loss {recent:.3f} "
                        + " ".join(f"{f.stem} {t:.1f}%" for f, t in zip(args.dev, tops)))
                print(f"dev {line}", flush=True)
                with dev_log.open("a", encoding="utf-8") as fh:
                    fh.write(line + "\n")
                mean = sum(tops) / len(tops)
                if mean > best:
                    best, stale = mean, 0
                    write_model(out / "best", params, vocab_list, ctx_chars)
                    print(f"dev 均值 {mean:.1f}% 创新高 → 存 best/", flush=True)
                else:
                    stale += 1
                    if stale >= 2:
                        print(f"dev 首选连跌两次（最好 {best:.1f}%，现 {mean:.1f}%）→ 早停", flush=True)
                        break
            next_dev = time.time() + args.dev_every
        if stop is not None and index >= stop:
            break

    write_model(out, params, vocab_list, ctx_chars)
    if best < 0:  # 没跑过 dev（或冒烟）：best 就用最后的
        write_model(out / "best", params, vocab_list, ctx_chars)
    done_file.write_text(f"step={len(losses)} best={best:.1f}\n", encoding="utf-8")
    print(f"写出 {out}；loss {losses[0]:.3f} → {losses[-1]:.3f}；最好 dev {best:.1f}%", flush=True)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
