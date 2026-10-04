//! 关掉或重新打开「同步」后，本机学习数据与配置的同步进度要作废。

use std::io::ErrorKind;
use std::path::Path;

use crate::config_sync::STATE_FILE as CONFIG_STATE;
use crate::learning::{BASE_FILE as LEARNING_BASE, STATE_FILE as LEARNING_STATE};

/// 删 `state_dir` 里 `LearningSync` 与 `ConfigSync` 的进度文件（学习数据基线与游标、配置版本号）。
/// 服务端关掉 `sync` 会删云端数据，本机基线还认为「已经在服务端」的话，重新打开只推增量、补不回去，配置还会走 409。
/// 剪贴板与输入日志的进度文件不动。调用前先停掉 [`crate::DataSync`]，否则线程会把文件写回来。
pub fn reset_sync_progress(state_dir: &Path) -> std::io::Result<()> {
    for name in [LEARNING_BASE, LEARNING_STATE, CONFIG_STATE] {
        for file in [name.to_owned(), format!("{name}.tmp")] {
            match std::fs::remove_file(state_dir.join(file)) {
                Ok(()) => {}
                Err(error) if error.kind() == ErrorKind::NotFound => {}
                Err(error) => return Err(error),
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::reset_sync_progress;

    #[test]
    fn removes_learning_and_config_progress_only() {
        let dir = std::env::temp_dir().join(format!("qj-reset-{}", std::process::id()));
        std::fs::remove_dir_all(&dir).ok();
        std::fs::create_dir_all(&dir).unwrap();
        let gone = [
            "learning-base.json",
            "learning-state.json",
            "config-state.json",
        ];
        let kept = [
            "state.json",
            "outbox.jsonl",
            "input-log-state.json",
            "clipboard.json",
        ];
        for name in gone.iter().chain(&kept) {
            std::fs::write(dir.join(name), "x").unwrap();
        }
        reset_sync_progress(&dir).unwrap();
        for name in gone {
            assert!(!dir.join(name).exists(), "{name} 应该被删");
        }
        for name in kept {
            assert!(dir.join(name).exists(), "{name} 应该还在");
        }
        // 文件或目录不存在也算成功
        reset_sync_progress(&dir).unwrap();
        reset_sync_progress(&dir.join("missing")).unwrap();
        std::fs::remove_dir_all(&dir).ok();
    }
}
