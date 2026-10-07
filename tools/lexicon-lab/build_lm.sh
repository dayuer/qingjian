#!/bin/bash
# 用法：build_lm.sh <输出名> <语料…>；分词词库固定 .lab/bm/dict-mixed（f98d297e），全局按计数截 MAXB（缺省 500 万）
set -euo pipefail
W=/Users/liyuqing/sproot/qingjian-dict-hunt; cd $W
name=$1; shift; out=.lab/bm/lm-$name; rm -rf $out; mkdir -p $out
./target/release/qingjian-dict-convert --out-dir $out bigram --dict .lab/bm/dict-mixed/dict.tsv --max-bigrams ${MAXB:-5000000} --min-count ${MINC:-3} \
  --phrases assets/lexicon/phrases.tsv --phrases assets/lexicon/domain_words.tsv \
  --brand assets/lexicon/brand.tsv --brand assets/lexicon/mixed_words.tsv "$@" > $out/bigram.log 2>&1
echo "$name: $(wc -l < $out/lm-bigram.tsv) bigrams $(wc -l < $out/lm-unigram.tsv) unigrams"
