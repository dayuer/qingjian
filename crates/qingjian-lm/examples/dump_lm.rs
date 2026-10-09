//! 把 `lm.qj` 倒回 `lm-unigram.tsv`（按词编号顺序）与 `lm-bigram.tsv`（按前词分组的顺序），元数据打到标准错误。
//!
//! 一元按编号顺序写，补的词接在末尾时原有的词编号不变，倒出再 `dict-convert pack lm` 与原件逐字节相同。
//! 节的布局与 `WordEntry`（12 字节：文本偏移、计数、长度、对齐）/ `Successor`（8 字节：后词编号、计数）一致，
//! 那两个类型改了这里跟着改。
//!
//! 用法：cargo run --release -p qingjian-lm --example dump_lm -- <lm.qj> <输出目录>

use std::io::Write;
use std::path::Path;

use qingjian_format::{Container, Kind};

fn u32_at(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let source = args.next().ok_or("用法：dump_lm <lm.qj> <输出目录>")?;
    let target = args.next().ok_or("用法：dump_lm <lm.qj> <输出目录>")?;
    let container = Container::open(Path::new(&source), Kind::LanguageModel)?;
    eprint!("{}", container.metadata().to_toml()?);
    let words = container.bytes(*b"WORD")?;
    let entries = container.bytes(*b"ENTR")?;
    let offsets = container.bytes(*b"OFFS")?;
    let successors = container.bytes(*b"SUCC")?;
    let count = entries.len() / 12;
    let text = |id: usize| -> Result<&str, std::str::Utf8Error> {
        let start = u32_at(entries, id * 12) as usize;
        let len = usize::from(u16::from_le_bytes([
            entries[id * 12 + 8],
            entries[id * 12 + 9],
        ]));
        std::str::from_utf8(&words[start..start + len])
    };
    let target = Path::new(&target);
    let mut unigram =
        std::io::BufWriter::new(std::fs::File::create(target.join("lm-unigram.tsv"))?);
    for id in 0..count {
        writeln!(unigram, "{}\t{}", text(id)?, u32_at(entries, id * 12 + 4))?;
    }
    unigram.flush()?;
    let mut bigram = std::io::BufWriter::new(std::fs::File::create(target.join("lm-bigram.tsv"))?);
    for previous in 0..count {
        let start = u32_at(offsets, previous * 4) as usize;
        let end = u32_at(offsets, previous * 4 + 4) as usize;
        for slot in start..end {
            let word = u32_at(successors, slot * 8) as usize;
            let pair = u32_at(successors, slot * 8 + 4);
            writeln!(bigram, "{}\t{}\t{}", text(previous)?, text(word)?, pair)?;
        }
    }
    bigram.flush()?;
    eprintln!("words={count} bigrams={}", successors.len() / 8);
    Ok(())
}
