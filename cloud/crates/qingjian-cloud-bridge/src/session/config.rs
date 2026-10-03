//! 套用 `config.toml`：iOS 键盘用得上的设置推给 Engine，与 Mac 壳的 `host::config::apply_config` 同一套字段。
//! 文件可能被主 App 的设置页改、也可能被同步从 Mac 拉下来改，所以按修改时间判断要不要重读。

use std::time::SystemTime;

use qingjian_platform::{Config, Scheme};

use super::Session;

impl Session {
    /// 修改时间变了（或第一次）就重读并套用，套用了返回 true；读不了按缺省，键盘照常能用。
    pub fn reload_config(&mut self) -> bool {
        let Some(path) = self.config_path.clone() else {
            return false;
        };
        let modified = std::fs::metadata(&path).and_then(|m| m.modified()).ok();
        if self.config_modified.is_some() && modified == self.config_modified {
            return false;
        }
        self.config_modified = modified.or(Some(SystemTime::UNIX_EPOCH));
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
        self.refresh();
    }
}
