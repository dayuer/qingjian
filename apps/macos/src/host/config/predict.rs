//! 云联想真正生效的配置：`[predict]` 选青简 Cloud 时，地址与令牌来自菜单栏里青简 Cloud 的配置（不写进会同步的 config.toml）；
//! 选自定义接口时照填的用。

use qingjian_platform::Config;
use qingjian_predict::{PredictConfig, PredictProvider};

use crate::host::Host;

impl Host {
    /// 失败的原因是给用户看的中文（青简 Cloud 还没配置）。
    pub(crate) fn effective_predict(&self, config: &Config) -> Result<PredictConfig, String> {
        let predict = &config.predict;
        match predict.provider {
            PredictProvider::Custom => Ok(predict.clone()),
            PredictProvider::Qingjian => {
                let endpoint = qingjian_cloud_mac::llm_endpoint().ok_or_else(|| {
                    "青简 Cloud 还没配置：在菜单栏「中☁ → 青简 Cloud ›」里填服务器地址与设备令牌".to_owned()
                })?;
                Ok(PredictConfig {
                    base_url: endpoint.base_url,
                    api_key: Some(endpoint.token),
                    api_key_env: String::new(),
                    ..predict.clone()
                })
            }
        }
    }
}
