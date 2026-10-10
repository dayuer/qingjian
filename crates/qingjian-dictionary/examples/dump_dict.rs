//! 把 `dict.qj` 倒回 `词\t音节\t词频` 的 TSV（写到标准输出），元数据打到标准错误。
//!
//! 随包数据只有 `.qj`（data-vN 压缩包），要在上次发版那份上补几条词时，先倒回 TSV、补行、再 `dict-convert pack dict`
//! （见 `tools/release/shipped-patch/`）。倒出再打包的条目与元数据与原件相同。
//!
//! 用法：cargo run --release -p qingjian-dictionary --example dump_dict -- <dict.qj> > dict.tsv

use std::io::Write;
use std::path::Path;

use qingjian_dictionary::Dictionary;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let source = std::env::args()
        .nth(1)
        .ok_or("用法：dump_dict <dict.qj> > dict.tsv")?;
    let dictionary = Dictionary::open_qj(Path::new(&source))?;
    if let Some(metadata) = dictionary.metadata() {
        eprint!("{}", metadata.to_toml()?);
    }
    let mut out = std::io::BufWriter::new(std::io::stdout().lock());
    for entry in dictionary.entries() {
        writeln!(out, "{}\t{}\t{}", entry.text, entry.pinyin, entry.frequency)?;
    }
    out.flush()?;
    Ok(())
}
