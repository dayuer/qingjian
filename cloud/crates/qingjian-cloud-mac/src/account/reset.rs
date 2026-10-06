//! 作废本机的同步进度：开关切换只清学习数据与配置的进度，换账号还要清掉剪贴板进度、下载的输入日志与输入日志本体。
//! 目录由调用方给：`support` 是 `QingjianCloud/`（剪贴板进度、`data/` 里是学习数据 / 配置 / 输入日志的进度），
//! `ime` 是输入法数据目录（输入日志本体 `input-log.jsonl` 在这里）。

use std::io::ErrorKind;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use qingjian_cloud_proto::Feature;

/// 输入日志文件名（输入法写，`InputLogSync` 读）。
const INPUT_LOG: &str = "input-log.jsonl";

/// 剪贴板同步进度与离线队列的文件名、学习数据等进度所在的子目录、下载的别的设备日志的子目录。
const SUPPORT_ENTRIES: [&str; 4] = ["state.json", "outbox.jsonl", "data", "input-log"];

/// 换账号后请输入法把输入日志的写入端丢掉缓冲并重开：见 [`take_input_log_reset`]。
static INPUT_LOG_RESET: AtomicBool = AtomicBool::new(false);

/// 登录后要不要作废旧账号的数据：账号 id 变了要；本机没记过账号（旧版本写的文件、删号后）但留着进度，按换了账号处理。
pub fn should_reset(previous: Option<i64>, now: i64, progress_exists: bool) -> bool {
    match previous {
        Some(previous) => previous != now,
        None => progress_exists,
    }
}

/// 改的是「同步」且服务器给的新值与本机原来的不同。
pub fn sync_toggled(feature: Feature, was: bool, now: bool) -> bool {
    feature == Feature::Sync && was != now
}

/// 只清学习数据与配置的同步进度（剪贴板与输入日志的留着）。调用前先停掉 `DataSync`。
pub fn reset_after_sync_toggle(support: &Path) {
    if let Err(error) = qingjian_cloud_client::reset_sync_progress(&support.join("data")) {
        tracing::warn!(%error, "同步进度删不掉");
    }
}

/// 请输入法重开输入日志（换账号清了日志文件之后调）。
pub fn request_input_log_reset() {
    INPUT_LOG_RESET.store(true, Ordering::Relaxed);
}

/// 输入法每秒问一次：true 表示换账号清过输入日志，要丢掉写入端缓冲里旧账号的输入、清空文件再重开。取走即清。
pub fn take_input_log_reset() -> bool {
    INPUT_LOG_RESET.swap(false, Ordering::Relaxed)
}

/// 清空云端输入记录之后清本机：输入日志本体截断、它的上传进度删掉，并让输入法丢掉写入端缓冲重开。
/// 学习数据与配置的进度不动（清的是输入记录，不是学到的东西）。
pub fn clear_input_log_files(support: &Path, ime: &Path) {
    ignore_missing(
        std::fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(ime.join(INPUT_LOG))
            .map(|_| ()),
        "输入日志",
    );
    ignore_missing(
        std::fs::remove_file(support.join("input-log-state.json")),
        "输入日志上传进度",
    );
    request_input_log_reset();
}

/// 本机留着任何一份旧账号的同步进度。
pub fn progress_exists(support: &Path) -> bool {
    SUPPORT_ENTRIES
        .iter()
        .any(|name| support.join(name).exists())
}

/// 换账号：旧账号留下的都不能带给下一个账号。基线留着，新账号服务器上是空的，会把本机学到的减掉；
/// `outbox.jsonl` 里是旧账号的离线事件，不删会传到新账号；输入日志本体在输入法数据目录里，不删的话
/// 新账号一开「上传输入日志」就从偏移 0 把旧账号期间的明文全传上去。调用前先停掉同步线程。
pub fn reset_account_data(support: &Path, ime: &Path) {
    for name in SUPPORT_ENTRIES {
        let path = support.join(name);
        let result = if path.is_dir() {
            std::fs::remove_dir_all(&path)
        } else {
            std::fs::remove_file(&path)
        };
        ignore_missing(result, "旧的同步进度");
    }
    // 输入法进程以追加模式握着这个文件，删掉它会让之后的输入全写进已删除的 inode；截断则继续往同一个文件写。
    // 输入法缓冲区里还没刷出的最多 19 条旧账号输入，要靠 [`request_input_log_reset`] 通知输入法丢掉并重开。
    ignore_missing(
        std::fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(ime.join(INPUT_LOG))
            .map(|_| ()),
        "旧的输入日志",
    );
    // 轮转或备份出来的 `input-log.jsonl.*`
    let prefix = format!("{INPUT_LOG}.");
    for entry in std::fs::read_dir(ime).into_iter().flatten().flatten() {
        if entry.file_name().to_string_lossy().starts_with(&prefix) {
            ignore_missing(std::fs::remove_file(entry.path()), "旧的输入日志副本");
        }
    }
    tracing::info!("换了账号，清掉旧的同步进度与输入日志");
}

fn ignore_missing(result: std::io::Result<()>, what: &str) {
    match result {
        Ok(()) => {}
        Err(error) if error.kind() == ErrorKind::NotFound => {}
        Err(error) => tracing::warn!(%error, "{what}删不掉"),
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn dirs(name: &str) -> (PathBuf, PathBuf, PathBuf) {
        let root = std::env::temp_dir().join(format!("qjc-reset-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let support = root.join("QingjianCloud");
        let ime = root.join("Qingjian");
        std::fs::create_dir_all(support.join("data")).unwrap();
        std::fs::create_dir_all(support.join("input-log")).unwrap();
        std::fs::create_dir_all(&ime).unwrap();
        (root, support, ime)
    }

    #[test]
    fn same_account_keeps_progress_other_account_or_unknown_resets() {
        assert!(!should_reset(Some(7), 7, true));
        assert!(should_reset(Some(7), 8, false));
        assert!(should_reset(None, 7, true));
        assert!(!should_reset(None, 7, false));
    }

    #[test]
    fn only_sync_value_change_counts_as_toggle() {
        assert!(sync_toggled(Feature::Sync, false, true));
        assert!(sync_toggled(Feature::Sync, true, false));
        assert!(!sync_toggled(Feature::Sync, true, true));
        assert!(!sync_toggled(Feature::Clipboard, false, true));
    }

    #[test]
    fn sync_toggle_removes_learning_and_config_progress_only() {
        let (root, support, _ime) = dirs("toggle");
        let data = support.join("data");
        for name in [
            "learning-base.json",
            "learning-state.json",
            "config-state.json",
        ] {
            std::fs::write(data.join(name), "x").unwrap();
        }
        std::fs::write(data.join("input-log-state.json"), "x").unwrap();
        std::fs::write(support.join("state.json"), "x").unwrap();
        std::fs::write(support.join("outbox.jsonl"), "x").unwrap();
        reset_after_sync_toggle(&support);
        for name in [
            "learning-base.json",
            "learning-state.json",
            "config-state.json",
        ] {
            assert!(!data.join(name).exists(), "{name} 应该被删");
        }
        assert!(data.join("input-log-state.json").exists());
        assert!(support.join("state.json").exists());
        assert!(support.join("outbox.jsonl").exists());
        std::fs::remove_dir_all(root).unwrap();
    }

    /// A 记了日志、换成 B 登录：B 名下收不到 A 的任何一行。
    #[test]
    fn account_switch_removes_input_log_and_every_offset() {
        let (root, support, ime) = dirs("switch");
        std::fs::write(support.join("data/input-log-state.json"), "offset").unwrap();
        std::fs::write(support.join("data/learning-base.json"), "x").unwrap();
        std::fs::write(support.join("state.json"), "x").unwrap();
        std::fs::write(support.join("outbox.jsonl"), "x").unwrap();
        std::fs::write(support.join("input-log/other.jsonl"), "x").unwrap();
        std::fs::write(support.join("config.toml"), "token = \"sjt_a\"").unwrap();
        std::fs::write(ime.join("input-log.jsonl"), "A 的明文\n").unwrap();
        std::fs::write(ime.join("input-log.jsonl.1"), "A 的旧明文\n").unwrap();
        std::fs::write(ime.join("user.tsv"), "keep").unwrap();
        assert!(progress_exists(&support));

        reset_account_data(&support, &ime);

        // 输入法进程以追加模式握着这个文件：必须截断而不是删除，之后它继续写同一个 inode
        assert!(ime.join("input-log.jsonl").exists());
        assert_eq!(
            std::fs::metadata(ime.join("input-log.jsonl"))
                .unwrap()
                .len(),
            0
        );
        assert!(!ime.join("input-log.jsonl.1").exists());
        assert!(!support.join("data").exists());
        assert!(!support.join("input-log").exists());
        assert!(!support.join("state.json").exists());
        assert!(!support.join("outbox.jsonl").exists());
        // 别的文件不动：配置文件由登录流程自己改，输入法的学习数据属于本机
        assert!(support.join("config.toml").exists());
        assert!(ime.join("user.tsv").exists());
        assert!(!progress_exists(&support));
        std::fs::remove_dir_all(root).unwrap();
    }

    /// 输入法的 BufWriter 以 O_APPEND 打开：截断后它的写入落在新的文件末尾，不会写向已删除的 inode。
    #[test]
    fn writer_holding_the_file_keeps_logging_after_reset() {
        use std::io::Write;
        let (root, support, ime) = dirs("append");
        let log = ime.join("input-log.jsonl");
        let mut writer = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log)
            .unwrap();
        writer.write_all("A 的明文\n".as_bytes()).unwrap();

        reset_account_data(&support, &ime);
        writer.write_all("B 的输入\n".as_bytes()).unwrap();

        assert_eq!(std::fs::read_to_string(&log).unwrap(), "B 的输入\n");
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn missing_input_log_is_not_created_by_reset() {
        let (root, support, ime) = dirs("nolog");
        reset_account_data(&support, &ime);
        assert!(!ime.join("input-log.jsonl").exists());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn input_log_reset_request_is_taken_once() {
        request_input_log_reset();
        assert!(take_input_log_reset());
        assert!(!take_input_log_reset());
    }

    #[test]
    fn reset_with_nothing_to_delete_is_fine() {
        let root = std::env::temp_dir().join(format!("qjc-reset-none-{}", std::process::id()));
        reset_account_data(&root.join("a"), &root.join("b"));
        reset_after_sync_toggle(&root.join("a"));
    }
}
