//! 内存探针：加载前、加载后、生成一次后各停一下（读一行标准输入），给外面的 `footprint -p <pid>` 量 phys_footprint。
//! 用法见 `tools/neural-quant/memory.sh`。

use std::io::BufRead;
use std::path::PathBuf;

use qingjian_neural::{CharScorer, P2c};

fn pause(stage: &str) {
    println!("{stage}");
    let mut line = String::new();
    let _ = std::io::stdin().lock().read_line(&mut line);
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = PathBuf::from(std::env::args().nth(1).ok_or("给模型路径")?);
    pause("before");
    let scorer = CharScorer::load(&path)?;
    pause("loaded");
    let p2c = P2c::new(scorer.model(), scorer.vocab()).ok_or("不是 P2C 模型")?;
    let top = p2c.convert("zhegecanguandedianhuashishenme", 4, 32)?;
    eprintln!(
        "{}",
        top.first().map(|c| c.text.as_str()).unwrap_or_default()
    );
    pause("generated");
    Ok(())
}
