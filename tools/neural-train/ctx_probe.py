#!/usr/bin/env python3
"""带上文/空上文两套口径的首选对比（贪心解码，两个模型用同一套，保证同口径）。

序列 `<eos> [上文] 拼音 <sep> 汉字 <eos>`；空上文时就是原格式。
读 `上文\\t拼音\\t汉字` 或 `句子\\t拼音\\t前文` 两种三列文件。

用法：python3 tools/neural-train/ctx_probe.py <模型三件套目录> <dev文件> [--limit N]
"""

import argparse
import json
import pathlib
import sys

import torch

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
from align_check import decode, forward, load_safetensors  # noqa: E402


def greedy(weights, vocab, ctx: str, keys: str, n_layer: int, n_head: int, max_chars: int = 40):
    """从 `<eos> [上文] 拼音 <sep>` 出发贪心解码，返回 (汉字, 分数)。

    与 `align_check.decode` 同一实现，只是前缀多了上文 —— 两个模型必须走同一条路，
    否则「涨了」可能只是解码器不同。"""
    ids = [vocab["<eos>"]]
    if ctx:
        ids.extend(vocab.get(ch, vocab["<unk>"]) for ch in ctx)
    ids.extend(vocab.get(ch, vocab["<unk>"]) for ch in keys)
    ids.append(vocab["<sep>"])
    text, score = "", 0.0
    for _ in range(max_chars):
        logits = forward(weights, torch.tensor([ids[-128:]]), n_layer, n_head)[0, -1]
        logp = torch.log_softmax(logits, dim=-1)
        token = int(torch.argmax(logp))
        score += float(logp[token])
        if token == vocab["<eos>"]:
            break
        text += next((ch for ch, i in vocab.items() if i == token), "?")
        ids.append(token)
    return text, score


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("model", type=pathlib.Path)
    parser.add_argument("dev", type=pathlib.Path)
    parser.add_argument("--limit", type=int, default=0)
    parser.add_argument("--ctx-first", action="store_true",
                        help="列序是 `上文\t拼音\t汉字`（派生 dev）；缺省按冻结题的 `句子\t拼音\t前文`")
    args = parser.parse_args()

    cfg = json.loads((args.model / "config.json").read_text(encoding="utf-8"))
    vocab_list = json.loads((args.model / "vocab.json").read_text(encoding="utf-8"))["tokens"]
    vocab = {token: index for index, token in enumerate(vocab_list)}
    weights = load_safetensors(args.model / "model.safetensors")

    rows = []
    for line in args.dev.read_text(encoding="utf-8").splitlines():
        if not line.strip() or line.startswith("#"):
            continue
        fields = line.split("\t")
        if args.ctx_first:                           # `上文\t拼音\t汉字`
            if len(fields) >= 3:
                rows.append((fields[0].strip(), fields[1].strip(), fields[2].strip()))
        elif len(fields) >= 3:                       # 冻结题：`句子\t拼音\t前文`（第三列才是上文）
            rows.append((fields[2].strip(), fields[1].strip(), fields[0].strip()))
        elif len(fields) == 2:
            rows.append(("", fields[1].strip(), fields[0].strip()))
    if args.limit:
        rows = rows[: args.limit]

    def run(use_ctx: bool) -> tuple[int, int, int, int]:
        """返回 (首选, 前三, 对字, 总字)。贪心只有一条路径，前三没有意义，与首选同值。"""
        top1 = chars_ok = chars_all = 0
        for c, keys, want in rows:
            prefix = c if use_ctx else ""
            got, _ = greedy(weights, vocab, prefix, keys, cfg["n_layer"], cfg["n_head"])
            top1 += got == want
            for a, b in zip(got, want):
                chars_ok += a == b
            chars_all += len(want)
        return top1, top1, chars_ok, chars_all

    with_ctx = run(True)
    plain = run(False)
    print(f"{args.model.name} / {args.dev.name}：{len(rows)} 句")
    print(f"  有上文 首选 {with_ctx[0]}/{len(rows)} = {with_ctx[0]/max(len(rows),1):.1%}  字准 {with_ctx[2]/max(with_ctx[3],1):.1%}")
    print(f"  空上文 首选 {plain[0]}/{len(rows)} = {plain[0]/max(len(rows),1):.1%}  字准 {plain[2]/max(plain[3],1):.1%}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
