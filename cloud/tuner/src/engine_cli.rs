//! 调上游的 `qingjian-cli`：批量查候选、回放评测。用的是和输入法同一个引擎与产品数据，判断「现在打不打得出来」最准。
//!
//! CLI 的输出是给人看的，这里只依赖两种行：查询时的 `   1. 候选  …`，回放时每种来源一行的 `命中数 a / b`。
//! 上游改了这两种格式，`cargo test -p qingjian-cloud-tuner` 里的解析测试会先红。

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::error::TunerError;

/// 一次命令行最多查多少个输入。
const QUERY_CHUNK: usize = 200;

/// 查询时每个输入看前几个候选。
const QUERY_LIMIT: usize = 9;

pub struct EngineCli {
    /// `qingjian-cli` 可执行文件。
    binary: PathBuf,

    /// 产品数据目录（含 `dict.qj`、`lm.qj`、`dicts/`），CLI 在它的上两级目录里运行（按 `data/generated/` 找数据）。
    data_dir: PathBuf,

    /// 输入法的 `config.toml`（模糊音、双拼、领域词库）；没有为 `None`。
    config: Option<PathBuf>,
}

/// 回放的命中数。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ReplayScore {
    pub hits: u32,

    pub total: u32,
}

impl ReplayScore {
    pub fn rate(self) -> f64 {
        if self.total == 0 {
            0.0
        } else {
            f64::from(self.hits) / f64::from(self.total)
        }
    }
}

impl EngineCli {
    pub fn new(binary: PathBuf, data_dir: PathBuf, config: Option<PathBuf>) -> Self {
        Self {
            binary,
            data_dir,
            config,
        }
    }

    fn command(&self, learning: &Path) -> Command {
        let mut command = Command::new(&self.binary);
        command
            .arg("--dict")
            .arg(self.data_dir.join("dict.qj"))
            .arg("--user-dict")
            .arg(learning.join("user.tsv"))
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .env("RUST_LOG", "error");
        if let Some(root) = self.data_dir.parent().and_then(Path::parent) {
            command.current_dir(root);
        }
        if let Some(config) = &self.config {
            command.arg("--config").arg(config);
            for domain in domains(config) {
                let path = self.data_dir.join("dicts").join(format!("{domain}.qj"));
                if path.exists() {
                    command.arg("--extra-dict").arg(path);
                }
            }
        } else {
            // 没有配置时与输入法缺省一致：只开成语。显式给空配置：不传的话 CLI 会去读跑它的那台机器上输入法的
            // config.toml（Mac 上开着云联想又没令牌时直接退出，tuner 的端到端测试就是这么挂的）
            command
                .arg("--config")
                .arg("/dev/null")
                .arg("--extra-dict")
                .arg(self.data_dir.join("dicts/idioms.qj"));
        }
        command
    }

    /// 每个输入的前几个候选。查询模式退出时会把学习数据写回，所以先拷一份临时的给它。
    pub fn candidates(
        &self,
        inputs: &[String],
        learning: &Path,
    ) -> Result<HashMap<String, Vec<String>>, TunerError> {
        let scratch = tempfile::tempdir()?;
        copy_dir(learning, scratch.path())?;
        let mut result = HashMap::new();
        for chunk in inputs.chunks(QUERY_CHUNK) {
            let output = self
                .command(scratch.path())
                .arg("--limit")
                .arg(QUERY_LIMIT.to_string())
                .args(chunk)
                .output()?;
            if !output.status.success() {
                return Err(TunerError::Cli(format!("查询失败：{}", output.status)));
            }
            result.extend(parse_candidates(&String::from_utf8_lossy(&output.stdout)));
        }
        Ok(result)
    }

    /// 回放一份日志，所有来源的命中数加总。
    pub fn replay(&self, log: &Path, learning: &Path) -> Result<ReplayScore, TunerError> {
        let output = self
            .command(learning)
            .arg("--replay")
            .arg(log)
            .arg("--misses")
            .arg("0")
            .output()?;
        if !output.status.success() {
            return Err(TunerError::Cli(format!("回放失败：{}", output.status)));
        }
        Ok(parse_replay(&String::from_utf8_lossy(&output.stdout)))
    }
}

/// `config.toml` 里 `[dictionaries] domains` 开着的领域词库；没写时输入法缺省只开成语。
fn domains(config: &Path) -> Vec<String> {
    let parsed = std::fs::read_to_string(config)
        .ok()
        .and_then(|text| text.parse::<toml::Table>().ok());
    let listed = parsed
        .as_ref()
        .and_then(|t| t.get("dictionaries"))
        .and_then(|d| d.get("domains"))
        .and_then(|d| d.as_array())
        .map(|list| {
            list.iter()
                .filter_map(|v| v.as_str().map(str::to_owned))
                .collect()
        });
    listed.unwrap_or_else(|| vec!["idioms".to_owned()])
}

fn parse_candidates(stdout: &str) -> HashMap<String, Vec<String>> {
    let mut result: HashMap<String, Vec<String>> = HashMap::new();
    let mut current: Option<String> = None;
    for line in stdout.lines() {
        if let Some(input) = line.strip_prefix("> ") {
            current = Some(input.trim().to_owned());
            result.entry(input.trim().to_owned()).or_default();
            continue;
        }
        let Some(input) = &current else {
            continue;
        };
        let trimmed = line.trim_start();
        let Some((number, rest)) = trimmed.split_once(". ") else {
            continue;
        };
        if number.is_empty() || !number.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }
        // 候选与后面的标注（[句]、词性释义）之间至少两个空格
        let text = rest.split("  ").next().unwrap_or(rest).trim();
        if !text.is_empty() {
            result
                .entry(input.clone())
                .or_default()
                .push(text.to_owned());
        }
    }
    result
}

fn parse_replay(stdout: &str) -> ReplayScore {
    let mut score = ReplayScore::default();
    for line in stdout.lines() {
        let Some((_, tail)) = line.split_once("命中数") else {
            continue;
        };
        let mut numbers = tail
            .split('/')
            .map(|part| part.trim().parse::<u32>().unwrap_or(0));
        score.hits += numbers.next().unwrap_or(0);
        score.total += numbers.next().unwrap_or(0);
    }
    score
}

fn copy_dir(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        if entry.file_type()?.is_file() {
            std::fs::copy(entry.path(), to.join(entry.file_name()))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_query_output() {
        let out = "> qingjian\n  切分: qing jian\n   1. 请见  [句]\n   2. 青简\n   3. 请柬  n. invitation\n      … 还有 158 个\n  parse 89µs\n> shiguo\n   1. 试过      v. have tried\n";
        let parsed = parse_candidates(out);
        assert_eq!(parsed["qingjian"], ["请见", "青简", "请柬"]);
        assert_eq!(parsed["shiguo"], ["试过"]);
    }

    #[test]
    fn parses_replay_output() {
        let out = "回放评测（内存学习，不写文件）\n词          3 条  首选  66.7%  前五 100.0%  命中数 2 / 3\n句          2 条  首选  50.0%  命中数 1 / 2\n";
        assert_eq!(parse_replay(out), ReplayScore { hits: 3, total: 5 });
    }
}
