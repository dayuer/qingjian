//! 键盘内存探针：照键盘扩展的用法驱动会话——开会话、加载随包 8 位通变、连打句子（每键之后与停键时按 250ms 轮询）、
//! 重排回来后上屏首选。开会话前、模型接上后、打完之后各停一下（读一行标准输入），给外面的 `footprint -p <pid>` 量。
//! 用法：`keyboard_probe <键盘 Data 目录> <题目 tsv（第二列拼音）> [句数]`，驱动脚本见 `tools/neural-quant/keyboard.sh`。

use std::io::BufRead;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use qingjian_cloud_bridge::{MODEL_ACTIVE, Session};

fn pause(stage: &str) {
    println!("{stage}");
    let mut line = String::new();
    let _ = std::io::stdin().lock().read_line(&mut line);
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let data = PathBuf::from(args.next().ok_or("给键盘 Data 目录")?);
    let questions = PathBuf::from(args.next().ok_or("给题目 tsv")?);
    let count: usize = args.next().and_then(|n| n.parse().ok()).unwrap_or(50);
    let keys: Vec<String> = std::fs::read_to_string(&questions)?
        .lines()
        .filter_map(|line| line.split('\t').nth(1).map(str::to_owned))
        .filter(|k| !k.is_empty() && k.bytes().all(|b| b.is_ascii_lowercase()))
        .take(count)
        .collect();
    pause("before");
    let mut session = Session::open(&data, None, None, None)?;
    session.load_model(&data.join("models/hanzhang-tongbian-q8.qjm"), true);
    let started = Instant::now();
    while session.model_state() != MODEL_ACTIVE {
        if started.elapsed() > Duration::from_secs(60) {
            return Err("模型没接上".into());
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    pause("loaded");
    let mut rescored = 0;
    let mut waits = Vec::new();
    for sentence in &keys {
        for c in sentence.chars() {
            session.push(c);
            session.poll();
        }
        let typed = Instant::now();
        for _ in 0..8 {
            std::thread::sleep(Duration::from_millis(250));
            if session.poll() {
                rescored += 1;
                waits.push(typed.elapsed().as_millis());
                break;
            }
        }
        session.commit(0);
    }
    waits.sort_unstable();
    let pick = |q: f64| {
        waits
            .get(((waits.len() as f64 * q) as usize).min(waits.len().saturating_sub(1)))
            .copied()
            .unwrap_or(0)
    };
    eprintln!(
        "sentences {} rescored {rescored} 停键到重排 p50 {} p95 {} ms",
        keys.len(),
        pick(0.5),
        pick(0.95)
    );
    pause("typed");
    Ok(())
}
