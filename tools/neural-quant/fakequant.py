# 假量化：把通变 safetensors 里选中的权重量化再反量化回 fp16，原地写回同一布局（偏移不变），导出三件套。
# 模式：none（对照，必须与原文件逐字节相同）/ int8（逐输出通道对称，[-127,127]）/ q4（沿输入维每 32 个一组对称，[-7,7]）。
# 用法：fakequant.py <源目录> <输出目录> <模式> [--emb]   --emb 连 tok_emb（共享输出层）一起，逐行量化
import json, struct, sys, shutil, os, hashlib
import numpy as np
src, dst, mode = sys.argv[1], sys.argv[2], sys.argv[3]
emb = '--emb' in sys.argv
LINEAR = ('attn.qkv.weight', 'attn.proj.weight', 'mlp.fc.weight', 'mlp.proj.weight')
raw = bytearray(open(f'{src}/model.safetensors', 'rb').read())
n = struct.unpack('<Q', raw[:8])[0]; header = json.loads(raw[8:8 + n]); base = 8 + n
def quant(w):
    w = w.astype(np.float32)
    if mode == 'int8':
        s = np.abs(w).max(axis=1, keepdims=True) / 127; s[s == 0] = 1
        return np.clip(np.round(w / s), -127, 127) * s
    if mode == 'q4':
        rows, cols = w.shape; g = w.reshape(rows, cols // 32, 32)
        s = np.abs(g).max(axis=2, keepdims=True) / 7; s[s == 0] = 1
        return (np.clip(np.round(g / s), -7, 7) * s).reshape(rows, cols)
    raise SystemExit(mode)
stats = []
for name, meta in header.items():
    if name == '__metadata__': continue
    pick = name.endswith(LINEAR) or (emb and name == 'tok_emb.weight')
    if mode == 'none' or not pick: continue
    assert meta['dtype'] == 'F16'
    a, b = meta['data_offsets']
    w = np.frombuffer(bytes(raw[base + a:base + b]), dtype='<f2').reshape(meta['shape'])
    q = quant(w).astype('<f2')
    rel = float(np.linalg.norm(q.astype(np.float32) - w.astype(np.float32)) / np.linalg.norm(w.astype(np.float32)))
    stats.append((name, rel))
    raw[base + a:base + b] = q.tobytes()
os.makedirs(dst, exist_ok=True)
open(f'{dst}/model.safetensors', 'wb').write(raw)
for f in ('config.json', 'vocab.json'): shutil.copy(f'{src}/{f}', f'{dst}/{f}')
print(f'{mode}{" +emb" if emb else ""}: 量化 {len(stats)} 个张量，相对误差 均 {np.mean([r for _, r in stats]) if stats else 0:.4f} 最大 {max([r for _, r in stats], default=0):.4f}',
      'sha', hashlib.sha256(raw).hexdigest()[:8])
