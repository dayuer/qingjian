//! 内存探针：加载前、加载后、跑完之后、归还空闲页之后各停一下（读一行标准输入），给外面的 `footprint -p <pid>` 量 phys_footprint。
//! 第二个参数 `generate`（缺省）跑一次自由生成；`fuse` 照 iOS 键盘只给 8 条整句路径打分，连打 `ROUNDS`（缺省 50）轮。
//! 用法见 `tools/neural-quant/memory.sh`。

use std::io::BufRead;
use std::path::PathBuf;

use qingjian_neural::{CharScorer, P2c};

const KEYS: &str = "zhegecanguandedianhuashishenme";

const PATHS: [&str; 8] = [
    "这个餐馆的电话是什么",
    "这个参观的电话是什么",
    "这个餐馆的电话是神么",
    "这个餐馆的店话是什么",
    "这个参观的店话是什么",
    "这个餐馆得电话是什么",
    "这个餐馆的电话事什么",
    "这各餐馆的电话是什么",
];

unsafe extern "C" {
    fn malloc_zone_pressure_relief(zone: *mut std::ffi::c_void, goal: usize) -> usize;
}

fn pause(stage: &str) {
    println!("{stage}");
    let mut line = String::new();
    let _ = std::io::stdin().lock().read_line(&mut line);
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = PathBuf::from(std::env::args().nth(1).ok_or("给模型路径")?);
    let fuse = std::env::args().nth(2).as_deref() == Some("fuse");
    pause("before");
    let scorer = CharScorer::load(&path)?;
    pause("loaded");
    if fuse {
        let rounds: usize = std::env::var("ROUNDS")
            .ok()
            .and_then(|r| r.parse().ok())
            .unwrap_or(50);
        for round in 0..rounds {
            scorer.score_p2c("", &KEYS[..10 + round % 21], &PATHS)?;
        }
        eprintln!("fused {rounds}");
    } else {
        let p2c = P2c::new(scorer.model(), scorer.vocab()).ok_or("不是 P2C 模型")?;
        let top = p2c.convert("", KEYS, 4, 32)?;
        eprintln!(
            "generated {}",
            top.first().map(|c| c.text.as_str()).unwrap_or_default()
        );
    }
    pause("generated");
    // 归还分配器攒着的空闲页，剩下的才是真占着的
    let released = unsafe { malloc_zone_pressure_relief(std::ptr::null_mut(), 0) };
    eprintln!("released {released}");
    pause("relieved");
    Ok(())
}
