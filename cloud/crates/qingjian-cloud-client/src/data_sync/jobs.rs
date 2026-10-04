//! 后台线程每轮要做的几项同步。某一项服务器说没开（403）就从这一轮起停掉它，别的照常。

use qingjian_cloud_proto::Feature;

use crate::config_sync::{ConfigOutcome, ConfigSync};
use crate::{ClientError, InputLogSync, LearningOutcome, LearningSync};

/// 开着的几项同步；关掉的为 `None`。
pub struct Jobs {
    pub learning: Option<LearningSync>,

    pub settings: Option<ConfigSync>,

    pub logs: Option<InputLogSync>,

    /// 服务器说没开（403）而停掉的功能。
    pub disabled: Vec<Feature>,
}

impl Jobs {
    pub fn cycle(&mut self) -> Result<(LearningOutcome, ConfigOutcome), ClientError> {
        let outcome = match self.learning.as_mut().map(LearningSync::cycle) {
            Some(Err(ClientError::Forbidden(_))) => {
                self.learning = None;
                self.disable(Feature::Sync);
                LearningOutcome::default()
            }
            Some(result) => result?,
            None => LearningOutcome::default(),
        };
        let config = match self.settings.as_mut().map(ConfigSync::cycle) {
            Some(Err(ClientError::Forbidden(_))) => {
                self.settings = None;
                self.disable(Feature::Sync);
                ConfigOutcome::Unchanged
            }
            Some(result) => result?,
            None => ConfigOutcome::Unchanged,
        };
        match self.logs.as_mut().map(InputLogSync::cycle) {
            Some(Err(ClientError::Forbidden(_))) => {
                self.logs = None;
                self.disable(Feature::InputLog);
            }
            Some(Err(error)) => return Err(error),
            Some(Ok(logged)) if logged.uploaded > 0 || logged.downloaded > 0 || logged.cleared => {
                tracing::info!(?logged, "输入日志同步");
            }
            Some(Ok(_)) | None => {}
        }
        Ok((outcome, config))
    }

    fn disable(&mut self, feature: Feature) {
        tracing::warn!(feature = feature.as_str(), "服务器上没开这项功能，停止同步");
        if !self.disabled.contains(&feature) {
            self.disabled.push(feature);
        }
    }
}
