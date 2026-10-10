//! 用库里的 P2c 解码一组拼音，打印候选与分数（训练脚本拿它对拍）。
//!
//! 加载方式与 `scorer.rs::load_loose` 完全一致：三件套目录 → config.json / vocab.json / model.safetensors。
//!
//! 用法：cargo run --release -p qingjian-neural --example p2c_probe -- <模型目录> <拼音1> <拼音2> ...
//!       cargo run --release -p qingjian-neural --example p2c_probe -- <模型目录> --keys-file <文件>
//!
//! `--keys-file` 逐行读拼音（`句子\t拼音\t前文` 三列取第二列，单列则整行就是拼音）：
//! 拼音一律从文件来，不走 shell 参数 —— `$KEYS` 在 zsh 里不做词分割，多条会被拼成一条（2026-10-08 踩过）。

use std::path::PathBuf;

use candle_core::{DType, Device};
use candle_nn::VarBuilder;
use qingjian_neural::{CharLm, ModelConfig, P2c, Vocab};

/// 读 key：三列取第二列，否则整行。
fn read_keys(path: &PathBuf) -> Result<Vec<String>, std::io::Error> {
    let mut keys = Vec::new();
    for line in std::fs::read_to_string(path)?.lines() {
        let line = line.trim_end();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let key = match line.split('\t').nth(1) {
            Some(second) if !second.is_empty() => second,
            _ => line,
        };
        keys.push(key.trim().to_owned());
    }
    Ok(keys)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let dir = PathBuf::from(args.next().ok_or("用法：p2c_probe <模型目录> <拼音…>")?);
    let mut keys: Vec<String> = Vec::new();
    let mut rest = args.collect::<Vec<_>>().into_iter().peekable();
    while let Some(item) = rest.next() {
        if item == "--keys-file" {
            let path = PathBuf::from(rest.next().ok_or("--keys-file 后面要跟文件路径")?);
            keys.extend(read_keys(&path)?);
        } else {
            keys.push(item);
        }
    }

    let device = Device::Cpu;
    let text = std::fs::read_to_string(dir.join("config.json"))?;
    let cfg: ModelConfig = serde_json::from_str(&text)?;
    let vb = unsafe {
        VarBuilder::from_mmaped_safetensors(&[dir.join("model.safetensors")], DType::F32, &device)?
    };
    let model = CharLm::load(vb, cfg, device)?;
    let vocab = Vocab::load(&dir.join("vocab.json"))?;
    let p2c = P2c::new(&model, &vocab).ok_or("字表里没有 <sep>：这不是 P2C 模型")?;
    for key in keys {
        for candidate in p2c.convert("", &key, 4, 16)?.iter().take(3) {
            println!("{key}\t{}\t{:.6}", candidate.text, candidate.score);
        }
    }
    Ok(())
}
