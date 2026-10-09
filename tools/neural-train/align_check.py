#!/usr/bin/env python3
"""对拍：Python 侧的前向与 Rust `P2c::convert` 在同一条拼音上必须给出同一个汉字与同一个分数。

先加载通变的三件套（`examples/dump_qjm.rs` 倒出来的），按 safetensors 里的真实张量名
（`tok_emb` / `pos_emb` / `blocks.N.attn.qkv` / `attn.proj` / `ln1` / `ln2` / `mlp.fc` / `mlp.proj` / `ln_f`）
实现前向，再用 `[<eos>] + 拼音字母 + [<sep>]` 作前缀贪心解码，逐字累加 log 概率 —— 与 Rust 侧比。
对不上就不准开训（用户定的规矩）。

用法：python3 tools/neural-train/align_check.py <参考输出.tsv> [模型目录]
  参考输出.tsv 由 `cargo run --release -p qingjian-neural --example p2c_probe -- <目录> <拼音…>` 生成。
"""

import json
import math
import pathlib
import struct
import sys

import numpy as np
import torch

CTX = 128


def load_safetensors(path: pathlib.Path) -> dict[str, torch.Tensor]:
    with path.open("rb") as fh:
        size = struct.unpack("<Q", fh.read(8))[0]
        header = json.loads(fh.read(size))
        base = 8 + size
        out = {}
        for name, info in header.items():
            if name == "__metadata__":
                continue
            start, end = info["data_offsets"]
            fh.seek(base + start)
            raw = np.frombuffer(fh.read(end - start), dtype=np.float16).astype(np.float32)
            out[name] = torch.from_numpy(raw.reshape(info["shape"]).copy())
    return out


def ln(x, w, b, eps=1e-5):
    return torch.nn.functional.layer_norm(x, (x.shape[-1],), w, b, eps)


def forward(weights: dict[str, torch.Tensor], idx: torch.Tensor, n_layer: int, n_head: int) -> torch.Tensor:
    embd = weights["tok_emb.weight"]
    batch, length = idx.shape
    x = embd[idx] + weights["pos_emb.weight"][:length]
    head = embd.shape[1] // n_head
    # 因果掩码：广播到 (batch, head, length, length)
    mask = torch.triu(torch.full((length, length), float("-inf")), diagonal=1)[None, None]
    for layer in range(n_layer):
        prefix = f"blocks.{layer}."
        h = ln(x, weights[prefix + "ln1.weight"], weights[prefix + "ln1.bias"])
        qkv = h @ weights[prefix + "attn.qkv.weight"].T + weights[prefix + "attn.qkv.bias"]
        q, k, v = qkv.split(embd.shape[1], dim=-1)
        q, k, v = (t.view(batch, length, n_head, head).transpose(1, 2) for t in (q, k, v))
        a = (q @ k.transpose(-2, -1)) / math.sqrt(head) + mask
        a = torch.softmax(a, dim=-1) @ v
        a = a.transpose(1, 2).reshape(batch, length, embd.shape[1])
        x = x + a @ weights[prefix + "attn.proj.weight"].T + weights[prefix + "attn.proj.bias"]
        h = ln(x, weights[prefix + "ln2.weight"], weights[prefix + "ln2.bias"])
        h = torch.nn.functional.gelu(h @ weights[prefix + "mlp.fc.weight"].T + weights[prefix + "mlp.fc.bias"])
        x = x + h @ weights[prefix + "mlp.proj.weight"].T + weights[prefix + "mlp.proj.bias"]
    return ln(x, weights["ln_f.weight"], weights["ln_f.bias"]) @ embd.T


def decode(weights, vocab, key: str, n_layer: int, n_head: int, max_chars: int = 16):
    """贪心解码，返回 (汉字, 分数)。分数 = 生成字符（含结束的 <eos>）的 log 概率之和。"""
    ids = [vocab["<eos>"]] + [vocab.get(ch, vocab["<unk>"]) for ch in key] + [vocab["<sep>"]]
    text, score = "", 0.0
    for _ in range(max_chars):
        logits = forward(weights, torch.tensor([ids[-CTX:]]), n_layer, n_head)[0, -1]
        logp = torch.log_softmax(logits, dim=-1)
        token = int(torch.argmax(logp))
        score += float(logp[token])
        if token == vocab["<eos>"]:
            break
        text += next((ch for ch, i in vocab.items() if i == token), "?")
        ids.append(token)
    return text, score


def main() -> int:
    reference = pathlib.Path(sys.argv[1])
    model_dir = pathlib.Path(sys.argv[2] if len(sys.argv) > 2 else ".lab/tongbian")
    cfg = json.loads((model_dir / "config.json").read_text(encoding="utf-8"))
    vocab_list = json.loads((model_dir / "vocab.json").read_text(encoding="utf-8"))["tokens"]
    vocab = {token: index for index, token in enumerate(vocab_list)}
    weights = load_safetensors(model_dir / "model.safetensors")

    worst = 0.0
    bad = 0
    # 只比每条拼音的**首位候选**：Rust 侧一个 key 会打三行（beam 前三），我们这边是贪心
    first: dict[str, tuple[str, str]] = {}
    for line in reference.read_text(encoding="utf-8").splitlines():
        if not line.strip():
            continue
        key, want_text, want_score = line.split("\t")
        first.setdefault(key, (want_text, want_score))
    for key, (want_text, want_score) in first.items():
        text, score = decode(weights, vocab, key, cfg["n_layer"], cfg["n_head"])
        delta = abs(score - float(want_score))
        worst = max(worst, delta)
        ok = text == want_text and delta < 1e-3
        bad += 0 if ok else 1
        print(f"{key}\t{text}\t{score:.6f}\tRust {want_text} {want_score}\tΔ {delta:.6f}\t{'✓' if ok else '✗'}")
    print(f"最大分数偏差 {worst:.6f}；不一致 {bad} 条")
    return 1 if bad else 0


if __name__ == "__main__":
    raise SystemExit(main())
