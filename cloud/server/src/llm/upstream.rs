//! 上游接口的 HTTP 客户端。

use std::time::Duration;

use super::LlmConfig;

pub struct Upstream {
    pub config: LlmConfig,

    pub client: reqwest::Client,
}

impl Upstream {
    pub fn new(config: LlmConfig) -> Result<Self, reqwest::Error> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .build()?;
        Ok(Self { config, client })
    }

    pub fn chat_url(&self) -> String {
        format!(
            "{}/chat/completions",
            self.config.base_url.trim_end_matches('/')
        )
    }

    pub fn enabled(&self) -> bool {
        !self.config.api_key.trim().is_empty()
    }
}
