//! 命令行两条主力用法的冒烟测试：裸拼音查询要出候选、`--replay` 要能读日志出指标。
//!
//! 52ffa83 给 `--homophone-snapshot` 插参数时把 `replay` 的 `#[arg(long)]` 抢走了，`replay` 变成
//! 位置参数 —— 裸查询与 `--replay` 一起哑掉，而 CI 里只有 `--homophone-snapshot` 那条路，没人发现。
//! 这两个测试跑真二进制（`apps/cli/src/args.rs` 里另有解析层面的守卫），词库按 CLI 自己的缺省顺序挑
//! （开发机是 `data/generated/dict.qj`，CI 是仓库里的 `assets/lexicon/dict.tsv`）。

use std::path::{Path, PathBuf};
use std::process::Command;

/// 仓库根：CLI 的数据路径都按工作目录找，测试要把工作目录摆在这儿。
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("apps/cli 之上应该是仓库根")
        .to_owned()
}

/// 评测用的干净配置：把云联想关掉，免得吃进开发机上的个人配置。
fn config_file(dir: &Path) -> PathBuf {
    let path = dir.join("smoke-config.toml");
    std::fs::write(&path, "[predict]\nenabled = false\n").unwrap();
    path
}

fn cli() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_qingjian-cli"));
    command.current_dir(repo_root());
    command
}

fn run(args: &[&str]) -> (bool, String) {
    let output = cli().args(args).output().expect("跑不起 CLI");
    let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&output.stderr));
    (output.status.success(), text)
}

#[test]
fn a_bare_pinyin_query_prints_candidates() {
    let dir = std::env::temp_dir().join(format!("qingjian-cli-smoke-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let config = config_file(&dir);
    let (ok, text) = run(&["--config", config.to_str().unwrap(), "nihao"]);
    std::fs::remove_dir_all(&dir).ok();
    assert!(ok, "裸拼音查询失败：{text}");
    assert!(text.contains("你好"), "nihao 没出「你好」：{text}");
}

#[test]
fn replay_reads_a_log_and_prints_metrics() {
    let dir = std::env::temp_dir().join(format!("qingjian-cli-replay-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let config = config_file(&dir);
    let log = dir.join("input-log.jsonl");
    std::fs::write(
        &log,
        r#"{"t":"2026-09-05T20:00:00+08:00","event":"commit","id":1,"scope":"nihao","keys":"nihao","pinyin":"ni'hao","corrected":false,"text":"你好","source":"word","index":0,"top":["你好"],"scheme":"","english":false}
{"t":"2026-09-05T20:00:02+08:00","event":"commit","id":2,"scope":"zaijian","keys":"zaijian","pinyin":"zai'jian","corrected":false,"text":"再见","source":"word","index":0,"top":["再见"],"scheme":"","english":false}
"#,
    )
    .unwrap();
    let (ok, text) = run(&[
        "--config",
        config.to_str().unwrap(),
        "--replay",
        log.to_str().unwrap(),
    ]);
    std::fs::remove_dir_all(&dir).ok();
    assert!(ok, "--replay 失败：{text}");
    assert!(text.contains("回放评测"), "没有回放报告：{text}");
    assert!(text.contains("词"), "报告里没有按来源的分组：{text}");
}
