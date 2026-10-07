//! 实验：读 lm.qj，对标准输入每行 `前词\t后词`（前词空 = 句首）打印 log P(后词 | 前词)；第一行先打词表规模与二元条数。

use std::io::BufRead;

use qingjian_core::sentence::LanguageModel;
use qingjian_lm::BigramModel;

fn main() {
    let path = std::env::args().nth(1).expect("lm.qj 路径");
    let model = BigramModel::from_path(std::path::Path::new(&path)).expect("读不了模型");
    println!("#\t{}\t{}", model.word_count(), model.bigram_count());
    for line in std::io::stdin().lock().lines() {
        let line = line.unwrap();
        let mut it = line.split('\t');
        let (prev, word) = (it.next().unwrap_or(""), it.next().unwrap_or(""));
        let prev = (!prev.is_empty()).then_some(prev);
        let lp = model
            .log_prob(prev, word)
            .map_or("-".to_owned(), |v| format!("{v:.3}"));
        println!("{line}\t{lp}");
    }
}
