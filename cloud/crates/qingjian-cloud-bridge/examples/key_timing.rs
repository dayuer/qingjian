//! 键盘按键耗时探针：用真实产品数据开一个 `Session`，逐键 `push` 并计时，看每键同步部分各场景多慢。
//! 用法：`cargo run --release -p qingjian-cloud-bridge --example key_timing -- <Data 目录> [--model]`
//! 数据目录按 iOS 包的样子拼（dict.qj、lm.qj、english.qj、dicts/、models/hanzhang-tongbian/）。
//! 加 `--model` 先等通变模型加载好再打字（键盘启动后就是这个状态）。Mac 上的数字只用来比相对，不等于 iPhone 的绝对值。

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use qingjian_cloud_bridge::Session;

/// 一个场景：名字与要敲的键。
const SCENARIOS: [(&str, &str); 8] = [
    ("纯拼音整句", "woxiangqushangbanle"),
    ("英文 android", "android"),
    ("英文 hello", "hellothere"),
    ("简拼 wmdx", "wmdxqsb"),
    ("拼音打错", "woxianggushangbanle"),
    ("中英夹杂", "woxiangxuehaorust"),
    ("长拼音", "jintiantianqihenhaowomenyiqiqukanbangongshi"),
    ("英文长词", "internationalization"),
];

fn percentile(sorted: &[Duration], q: f64) -> Duration {
    let rank = ((q * sorted.len() as f64).ceil() as usize).clamp(1, sorted.len());
    sorted[rank - 1]
}

fn ms(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1000.0
}

fn open(data: &Path, user: &Path, model: bool) -> Session {
    let mut session = Session::open(data, Some(user), None, None).expect("打开会话");
    if model {
        let path = data.join("models/hanzhang-tongbian/hanzhang-tongbian-small.qjm");
        assert!(session.load_model(&path, true), "加载模型");
        while session.model_state() != qingjian_cloud_bridge::MODEL_ACTIVE {
            std::thread::sleep(Duration::from_millis(20));
        }
    }
    session
}

fn type_keys(session: &mut Session, keys: &str) -> Vec<Duration> {
    keys.chars()
        .map(|c| {
            let started = Instant::now();
            session.push(c);
            // Swift 每键还要取 preedit 与整栏候选
            let _ = session.preedit().len() + session.entries().len();
            started.elapsed()
        })
        .collect()
}

fn report(name: &str, times: &[Duration]) {
    let mut sorted = times.to_vec();
    sorted.sort();
    let line: Vec<String> = times.iter().map(|t| format!("{:.1}", ms(*t))).collect();
    println!(
        "{:<14} {:>5} {:>8.2} {:>8.2} {:>8.2}  {}",
        name,
        times.len(),
        ms(percentile(&sorted, 0.5)),
        ms(percentile(&sorted, 0.99)),
        ms(*sorted.last().unwrap()),
        line.join(" ")
    );
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let data = PathBuf::from(args.get(1).expect("用法：key_timing <Data 目录> [--model]"));
    let model = args.iter().any(|a| a == "--model");
    let user = std::env::temp_dir().join(format!("qj-key-timing-{}", std::process::id()));
    std::fs::create_dir_all(&user).unwrap();
    println!(
        "模型：{}",
        if model {
            "通变已加载"
        } else {
            "不加载"
        }
    );
    println!(
        "{:<14} {:>5} {:>8} {:>8} {:>8}  逐键 ms",
        "场景", "键数", "p50", "p99", "最慢"
    );
    for (name, keys) in SCENARIOS {
        // 每个场景一个新会话：英文词表是懒加载的，要看到「第一次见到像英文」那一键
        let mut session = open(data.as_path(), &user, model);
        let times = type_keys(&mut session, keys);
        report(name, &times);
        session.clear();
    }
    // 英文已经触发过之后：同样的纯拼音每键多慢（之后每个键都带着英文补全与纠错）
    for (name, keys) in [("纯拼音整句", SCENARIOS[0].1), ("长拼音", SCENARIOS[6].1)] {
        let mut session = open(data.as_path(), &user, model);
        for c in "hello".chars() {
            session.push(c);
        }
        session.clear();
        let times = type_keys(&mut session, keys);
        report(&format!("英文已加载+{name}"), &times);
        session.clear();
    }
    // 内存告警卸掉英文表后，下一个像英文的输入又装回来：装回那一键多慢，连做 8 轮
    let mut session = open(data.as_path(), &user, model);
    let mut reloads = Vec::new();
    for _ in 0..8 {
        session.unload_english();
        session.clear();
        let times = type_keys(&mut session, "andr");
        reloads.push(times[3]);
    }
    println!(
        "卸载后重装英文表（第 4 键 r）8 轮：{}",
        reloads
            .iter()
            .map(|t| format!("{:.1}", ms(*t)))
            .collect::<Vec<_>>()
            .join(" ")
    );
    // 键盘每 0.25 秒在主线程上跑一次的轮询
    let mut session = open(data.as_path(), &user, model);
    let mut polls = Vec::new();
    for _ in 0..40 {
        let started = Instant::now();
        session.poll();
        let _ = session.model_state();
        polls.push(started.elapsed());
    }
    polls.sort();
    println!(
        "poll() 每 0.25 秒一次：p50 {:.2} ms / 最慢 {:.2} ms",
        ms(percentile(&polls, 0.5)),
        ms(*polls.last().unwrap())
    );
    std::fs::remove_dir_all(&user).ok();
}
