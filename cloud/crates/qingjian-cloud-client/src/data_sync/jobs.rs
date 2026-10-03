//! 后台线程每轮要做的几项同步。

use crate::config_sync::{ConfigOutcome, ConfigSync};
use crate::{ClientError, InputLogSync, LearningOutcome, LearningSync};

/// 开着的几项同步；关掉的为 `None`。
pub struct Jobs {
    pub learning: Option<LearningSync>,

    pub settings: Option<ConfigSync>,

    pub logs: Option<InputLogSync>,
}

impl Jobs {
    pub fn cycle(&mut self) -> Result<(LearningOutcome, ConfigOutcome), ClientError> {
        let outcome = match &mut self.learning {
            Some(learning) => learning.cycle()?,
            None => LearningOutcome::default(),
        };
        let config = match &mut self.settings {
            Some(settings) => settings.cycle()?,
            None => ConfigOutcome::Unchanged,
        };
        if let Some(logs) = &mut self.logs {
            let logged = logs.cycle()?;
            if logged.uploaded > 0 || logged.downloaded > 0 || logged.cleared {
                tracing::info!(?logged, "输入日志同步");
            }
        }
        Ok((outcome, config))
    }
}
