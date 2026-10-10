#!/bin/bash
# 在上次发版的随包数据（data-v3：dict.qj 7fafe1b8、lm.qj f9fb7b44）上补本目录里的几条词，不重跑建库管线。
# 嗯（en）在 Unihan 里只有 ń / ňg / ǹg，诶不在 8105 表里，建库时都没进来，en / ei 打不出这两个字。
# 做法：倒回 TSV（倒出再打包与原件相同）→ 接上本目录的行 → 按原元数据重打 → 核对哈希。
# 口径：频次都按随包数据自己的口径，用里面已有叹词相对 LCCC 字频的比值中位数换算（各 TSV 头部写了比值）。
#
# 用法：tools/release/shipped-patch/patch.sh <随包数据目录> <输出目录>
#   例：tools/release/shipped-patch/patch.sh data/generated.shipped /tmp/shipped-patched
set -euo pipefail
cd "$(dirname "$0")/../../.."
here=tools/release/shipped-patch
src=${1:?用法：patch.sh <随包数据目录> <输出目录>}
out=${2:?用法：patch.sh <随包数据目录> <输出目录>}
mkdir -p "$out"

sha8() { shasum -a 256 "$1" | cut -c1-8; }
check() { [[ "$(sha8 "$1")" == "$2" ]] || { echo "$1 的哈希是 $(sha8 "$1")，期望 $2" >&2; exit 1; }; }

check "$src/dict.qj" 7fafe1b8
check "$src/lm.qj" f9fb7b44
cargo build -q --release -p qingjian-dict-convert -p qingjian-dictionary --example dump_dict -p qingjian-lm --example dump_lm
bin=target/release

$bin/examples/dump_dict "$src/dict.qj" > "$out/dict.tsv" 2>/dev/null
grep -v '^#' "$here/dict.tsv" >> "$out/dict.tsv"
$bin/qingjian-dict-convert --out-dir "$out" pack dict --name "青简基础词库" --license "MIT AND Unicode-3.0"

$bin/examples/dump_lm "$src/lm.qj" "$out" 2>/dev/null
grep -v '^#' "$here/lm-unigram.tsv" >> "$out/lm-unigram.tsv"
grep -v '^#' "$here/lm-bigram.tsv" >> "$out/lm-bigram.tsv"
$bin/qingjian-dict-convert --out-dir "$out" pack lm --name "青简语言模型（中文维基 + LCCC，青简词库分词）" \
  --license "CC-BY-SA-4.0 AND MIT" --attribution "中文维基百科（CC BY-SA 4.0）；LCCC（清华大学 CoAI，MIT）"

check "$out/dict.qj" 0c24a7ae
check "$out/lm.qj" 94d18e77
echo "已补好：$out/dict.qj（0c24a7ae）、$out/lm.qj（94d18e77）"
