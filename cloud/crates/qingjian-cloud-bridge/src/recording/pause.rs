//! 「记录中」的暂停状态：`<user_dir>/cloud/recording-pause.json`，`{"until": 秒}` 或 `{"until": null}`（一直暂停）。
//! 落盘是因为键盘进程随时被杀，重开要还记得在暂停；文件坏了按没暂停算（宁可多记也不让标记卡死在「已暂停」）。

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// 暂停到什么时候。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Until {
    At(i64),
    Forever,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct RecordingPause {
    /// `Some(秒)` 定时，`None` 一直暂停；文件不存在时整个结构是缺省（没暂停）。
    until: Option<i64>,

    #[serde(skip)]
    present: bool,
}

impl RecordingPause {
    fn path(user_dir: &Path) -> PathBuf {
        user_dir.join("cloud").join("recording-pause.json")
    }

    pub fn load(user_dir: &Path) -> Self {
        std::fs::read_to_string(Self::path(user_dir))
            .ok()
            .and_then(|text| serde_json::from_str::<Self>(&text).ok())
            .map(|mut pause| {
                pause.present = true;
                pause
            })
            .unwrap_or_default()
    }

    /// 现在（`now` 秒）还在不在暂停。
    pub fn until(&self, now: i64) -> Option<Until> {
        if !self.present {
            return None;
        }
        match self.until {
            None => Some(Until::Forever),
            Some(at) if at > now => Some(Until::At(at)),
            Some(_) => None,
        }
    }

    pub fn pause_for(user_dir: &Path, seconds: i64, now: i64) -> std::io::Result<()> {
        Self::write(user_dir, Some(now + seconds))
    }

    pub fn pause_forever(user_dir: &Path) -> std::io::Result<()> {
        Self::write(user_dir, None)
    }

    pub fn resume(user_dir: &Path) -> std::io::Result<()> {
        match std::fs::remove_file(Self::path(user_dir)) {
            Err(error) if error.kind() != std::io::ErrorKind::NotFound => Err(error),
            _ => Ok(()),
        }
    }

    /// 读的一方是键盘进程，随时可能在读，所以走 `write_atomic`（同目录写 tmp 再改名）。
    fn write(user_dir: &Path, until: Option<i64>) -> std::io::Result<()> {
        let text = serde_json::to_string(&Self {
            until,
            present: true,
        })
        .map_err(std::io::Error::other)?;
        crate::cloud_config::write_atomic(&Self::path(user_dir), text.as_bytes(), true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// 同 `memory/tests` 的写法：临时目录按测试名 + 进程号，先清再建。
    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("qj-recording-{name}-{}", std::process::id()));
        std::fs::remove_dir_all(&dir).ok();
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn no_file_means_not_paused() {
        let dir = temp_dir("none");
        assert_eq!(RecordingPause::load(&dir).until(1_000), None);
    }

    #[test]
    fn timed_pause_expires() {
        let dir = temp_dir("timed");
        RecordingPause::pause_for(&dir, 3600, 1_000).unwrap();
        assert_eq!(
            RecordingPause::load(&dir).until(1_000),
            Some(Until::At(4_600))
        );
        assert_eq!(
            RecordingPause::load(&dir).until(4_600),
            None,
            "到点就不算暂停了"
        );
    }

    #[test]
    fn forever_and_resume() {
        let dir = temp_dir("forever");
        RecordingPause::pause_forever(&dir).unwrap();
        assert_eq!(
            RecordingPause::load(&dir).until(9_999_999),
            Some(Until::Forever)
        );
        RecordingPause::resume(&dir).unwrap();
        assert_eq!(RecordingPause::load(&dir).until(1), None);
    }

    #[test]
    fn broken_file_is_not_paused() {
        let dir = temp_dir("broken");
        std::fs::create_dir_all(dir.join("cloud")).unwrap();
        std::fs::write(dir.join("cloud/recording-pause.json"), "{oops").unwrap();
        assert_eq!(RecordingPause::load(&dir).until(1), None);
    }
}
