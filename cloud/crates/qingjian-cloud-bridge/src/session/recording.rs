//! 「记录中」：暂停就把输入日志换成什么都不写的 NoInputLogger，恢复再把 InputLog 装回去。
//! 只停「记」，学习、提示、记一笔照常；已经记下的照常上传（上传器不看这里）。

use qingjian_core::NoInputLogger;
use qingjian_learning::InputLog;

use super::Session;
use crate::recording::{RecordingPause, RecordingState, Until};

impl Session {
    /// 顺带处理到期：定时暂停过了点，就把日志装回去。键盘每次刷新都会问，所以到期最多晚一次刷新。
    pub fn recording_state(&mut self, now: i64) -> RecordingState {
        let Some(dir) = self.user_dir.clone().filter(|_| self.logs_on) else {
            return RecordingState::Off;
        };
        match RecordingPause::load(&dir).until(now) {
            Some(Until::At(_)) => RecordingState::PausedTimed,
            Some(Until::Forever) => RecordingState::PausedForever,
            None => {
                if self.logger_paused {
                    self.engine
                        .set_input_logger(Box::new(InputLog::open(dir.join("input-log.jsonl"))));
                    self.logger_paused = false;
                }
                RecordingState::Recording
            }
        }
    }

    /// `seconds` 为 None 是一直暂停。
    pub fn pause_recording(&mut self, seconds: Option<i64>, now: i64) {
        let Some(dir) = self.user_dir.clone().filter(|_| self.logs_on) else {
            return;
        };
        let written = match seconds {
            Some(seconds) => RecordingPause::pause_for(&dir, seconds, now),
            None => RecordingPause::pause_forever(&dir),
        };
        if let Err(error) = written {
            tracing::warn!(%error, "暂停记录没写上");
            return;
        }
        self.engine.set_input_logger(Box::new(NoInputLogger));
        self.logger_paused = true;
    }

    pub fn resume_recording(&mut self, now: i64) {
        if let Some(dir) = self.user_dir.clone()
            && let Err(error) = RecordingPause::resume(&dir)
        {
            tracing::warn!(%error, "恢复记录没删掉暂停文件");
        }
        let _ = self.recording_state(now);
    }
}
