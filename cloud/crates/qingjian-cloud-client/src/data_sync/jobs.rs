//! 后台线程每轮要做的几项同步。某一项服务器说没开（403）就从这一轮起停掉它，别的照常。

use qingjian_cloud_proto::Feature;

use crate::config_sync::{ConfigOutcome, ConfigSync};
use crate::{ClientError, InputLogSync, LearningOutcome, LearningSync};

/// 开着的几项同步；关掉的为 `None`。
pub struct Jobs {
    pub learning: Option<LearningSync>,

    pub settings: Option<ConfigSync>,

    pub logs: Option<InputLogSync>,

    /// 服务器说没开（403）而停掉的功能。只增不减：本实例内不会自动恢复，重新打开后须重建 `DataSync`；
    /// `Feature::Sync` 同时覆盖学习数据与配置两项；所有项被停掉后 `cycle` 仍返回 `Ok`，`last_ok_ms` 照常刷新。
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

#[cfg(test)]
mod tests {
    use qingjian_cloud_proto::Feature;

    use super::Jobs;
    use crate::config_sync::ConfigSync;
    use crate::test_support::fake_server;
    use crate::{Client, LearningSync};

    #[test]
    fn forbidden_stops_the_job_and_is_not_requested_again() {
        let dir = std::env::temp_dir().join(format!("qjc-jobs-{}", uuid::Uuid::new_v4()));
        let (ime, state) = (dir.join("ime"), dir.join("state"));
        std::fs::create_dir_all(&ime).unwrap();
        std::fs::create_dir_all(&state).unwrap();
        let (url, rx) = fake_server("403 Forbidden");
        let client = Client::new(&url, "tok");
        let mut jobs = Jobs {
            learning: Some(LearningSync::open(client.clone(), &ime, &state).unwrap()),
            settings: Some(ConfigSync::open(client, &ime, &state).unwrap()),
            logs: None,
            disabled: Vec::new(),
        };

        assert!(jobs.cycle().is_ok());
        assert!(jobs.learning.is_none());
        assert!(jobs.settings.is_none());
        assert_eq!(jobs.disabled, vec![Feature::Sync]);
        assert!(rx.try_recv().is_ok(), "第一轮应当发过请求");
        while rx.try_recv().is_ok() {}

        assert!(jobs.cycle().is_ok());
        assert!(rx.try_recv().is_err(), "停掉之后不该再发请求");
        std::fs::remove_dir_all(dir).unwrap();
    }
}
