//! 作废本机的同步进度与账号数据：开关切换只清学习数据与配置的进度，换账号与删号还要清掉整个 `cloud/` 和输入日志。
//! 都以 `cloud.toml` 的路径为准：它与 `cloud/`、`input-log.jsonl` 在同一个目录里
//! （开了完全访问时键盘的学习数据目录就是 App Group 目录；没开完全访问键盘既不联网也不写这个目录）。

use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use qingjian_cloud_proto::Feature;

/// 输入日志文件名（与 `session` 里 `InputLog::open` 用的一致）。
const INPUT_LOG: &str = "input-log.jsonl";

/// 登录后要不要作废旧账号的数据：账号 id 变了要；本机没记过账号（旧版本写的文件、删号后）但留着进度，按换了账号处理。
pub fn should_reset(previous: Option<i64>, now: i64, cloud_dir_exists: bool) -> bool {
    match previous {
        Some(previous) => previous != now,
        None => cloud_dir_exists,
    }
}

/// 改的是「同步」且服务器给的新值与本机原来的不同。
pub fn sync_toggled(feature: Feature, was: bool, now: bool) -> bool {
    feature == Feature::Sync && was != now
}

pub fn cloud_dir(path: &Path) -> Option<PathBuf> {
    path.parent().map(|dir| dir.join("cloud"))
}

/// 只清学习数据与配置的同步进度（剪贴板与输入日志的留着），文件在 `cloud.toml` 同目录的 `cloud/` 下。
pub fn reset_after_sync_toggle(path: &Path) {
    let Some(dir) = cloud_dir(path) else {
        return;
    };
    if let Err(error) = qingjian_cloud_client::reset_sync_progress(&dir) {
        tracing::warn!(%error, "同步进度删不掉");
    }
}

/// 换账号或删号：旧账号留下的都不能带给下一个账号。
/// 整个删 `cloud/`：基线留着，新账号服务器上是空的，会把本机学到的减掉；`outbox.jsonl` 里是旧账号的离线事件，
/// 不删会传到新账号。输入日志在 `cloud/` 外，不删的话新账号一开「上传输入日志」就从偏移 0 把旧账号期间的明文全传上去。
pub fn reset_account_data(path: &Path) {
    let Some(dir) = path.parent() else {
        return;
    };
    ignore_missing(std::fs::remove_dir_all(dir.join("cloud")), "旧的同步进度");
    ignore_missing(std::fs::remove_file(dir.join(INPUT_LOG)), "旧的输入日志");
    // 轮转或备份出来的 `input-log.jsonl.*`
    let rotated = std::fs::read_dir(dir).into_iter().flatten().flatten();
    for entry in rotated {
        if entry
            .file_name()
            .to_string_lossy()
            .starts_with(&format!("{INPUT_LOG}."))
        {
            ignore_missing(std::fs::remove_file(entry.path()), "旧的输入日志副本");
        }
    }
    tracing::info!("换了账号或删了账号，清掉旧的同步进度与输入日志");
}

fn ignore_missing(result: std::io::Result<()>, what: &str) {
    match result {
        Ok(()) => {}
        Err(error) if error.kind() == ErrorKind::NotFound => {}
        Err(error) => tracing::warn!(%error, "{what}删不掉"),
    }
}
