//! 套用 `config.toml`：iOS 键盘用得上的设置推给 Engine，与 Mac 壳的 `host::config::apply_config` 同一套字段。
//! 文件可能被主 App 的设置页改、也可能被同步从 Mac 拉下来改，所以按修改时间判断要不要重读。

use std::time::SystemTime;

use qingjian_core::NoPredictor;
use qingjian_platform::{Config, Scheme};
use qingjian_predict::{CloudPredictor, PredictConfig, PredictProvider};

use super::Session;

impl Session {
    /// 修改时间变了（或第一次）就重读并套用，套用了返回 true；读不了按缺省，键盘照常能用。
    pub fn reload_config(&mut self) -> bool {
        let Some(path) = self.config_path.clone() else {
            return false;
        };
        // 文件不在时记成 UNIX_EPOCH，免得每次轮询都当成「变了」重读一遍（还会重载领域词库）
        let modified = std::fs::metadata(&path)
            .and_then(|m| m.modified())
            .unwrap_or(SystemTime::UNIX_EPOCH);
        if self.config_modified == Some(modified) {
            return false;
        }
        self.config_modified = Some(modified);
        let config = match Config::load(&path) {
            Ok(config) => config,
            Err(error) => {
                tracing::warn!(%error, "config.toml 读不了，按缺省");
                Config::default()
            }
        };
        self.apply(&config);
        tracing::info!("设置已套用");
        true
    }

    fn apply(&mut self, config: &Config) {
        self.engine.set_fuzzy(config.fuzzy);
        self.engine.set_traditional_mode(config.general.traditional);
        self.engine
            .set_full_width_punctuation(config.general.full_width_punctuation);
        self.engine.set_learning(config.general.learning);
        // iOS 键盘是 26 键：全拼与双拼都能用，注音与「只用形码」按全拼
        let shuangpin = match config.general.scheme() {
            Scheme::Shuangpin(scheme) => Some(scheme),
            _ => None,
        };
        self.engine.set_shuangpin(shuangpin);
        if let Err(error) = self
            .engine
            .set_custom_phrases(config.custom_phrases.clone())
        {
            tracing::warn!(%error, "自定义短语没套用");
        }
        self.engine
            .set_extra_dictionaries(qingjian_platform::extra_dictionaries::load(
                Some(&self.dicts_dir),
                None,
                &config.dictionaries,
            ));
        self.apply_prediction(&config.predict);
        self.refresh();
    }

    /// 云联想与 Mac 用同一份 `[predict]`：选青简 Cloud 时地址与令牌来自这台设备的 `cloud.toml`（`llm` 关着就不联想），
    /// 自定义接口照配置用。结果插在首选之后（见 `cloud.rs`）。
    fn apply_prediction(&mut self, predict: &PredictConfig) {
        let effective = match (predict.enabled, predict.provider, &self.cloud) {
            (false, _, _) => None,
            // 手机候选栏窄，只要云端词，不要整句补全
            (true, PredictProvider::Qingjian, Some(cloud)) if cloud.llm => Some(PredictConfig {
                base_url: cloud.llm_base_url(),
                api_key: Some(cloud.token.clone()),
                api_key_env: String::new(),
                sentence: false,
                ..predict.clone()
            }),
            (true, PredictProvider::Custom, _) => Some(PredictConfig {
                sentence: false,
                ..predict.clone()
            }),
            (true, PredictProvider::Qingjian, _) => {
                tracing::info!("云联想要素笺云，这台设备没配置或关了大模型");
                None
            }
        };
        let predictor = effective.and_then(|config| match CloudPredictor::new(&config) {
            Ok(predictor) => Some(predictor),
            Err(error) => {
                tracing::warn!(%error, "云联想启动失败");
                None
            }
        });
        match predictor {
            Some(predictor) => self.engine.set_predictor(Box::new(predictor)),
            None => self.engine.set_predictor(Box::new(NoPredictor)),
        }
    }
}
