//! 把 `.qjm` 里的三件套（config.json / vocab.json / model.safetensors）原样倒出来。
//!
//! 训练脚本要沿用现有模型的字表与权重做对拍，但 `.qjm` 是 `.qj` 容器、节偏移不该在 Python 里猜 ——
//! 用库里自己的读取器倒，格式改了这里跟着改。
//!
//! 用法：cargo run --release -p qingjian-neural --example dump_qjm -- <模型.qjm> <输出目录>

use std::path::Path;

use qingjian_format::{Container, Kind};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let source = args.next().ok_or("用法：dump_qjm <模型.qjm> <输出目录>")?;
    let target = args.next().ok_or("用法：dump_qjm <模型.qjm> <输出目录>")?;
    let target = Path::new(&target);
    std::fs::create_dir_all(target)?;
    let container = Container::open(Path::new(&source), Kind::Model)?;
    for (tag, name) in [
        (*b"CONF", "config.json"),
        (*b"VOCB", "vocab.json"),
        (*b"SAFT", "model.safetensors"),
    ] {
        let body = container.bytes(tag)?;
        println!("{}: {} 字节", name, body.len());
        std::fs::write(target.join(name), body)?;
    }
    Ok(())
}
