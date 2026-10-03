//! 键盘连青简 Cloud 的配置（`cloud.toml`）：服务器地址与这台设备的令牌，外加两个开关。
//! 没有这个文件、或地址 / 令牌是空的，键盘就完全离线。

use std::path::Path;

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct CloudConfig {
    /// 服务器地址，如 `https://pinyin.synon.ai`。
    pub server: String,

    /// 这台设备的令牌（服务器上 `qingjian-cloud device add <名字>` 生成）。
    pub token: String,

    /// 用服务器的大模型：润色；再开 `candidates` 才在组字时补候选。
    pub llm: bool,

    /// 组字时让大模型补候选（缺省关：要等网络、帮助不大，准确率交给服务器上的纠错闭环）。
    pub candidates: bool,

    /// 记输入日志并上传，服务器上的纠错闭环（tuner）靠它找打错、分几次才选完的地方。
    pub logs: bool,

    /// 与别的设备同步学习数据（词频、用户词、个人 n-gram 等）。
    pub sync: bool,
}

impl Default for CloudConfig {
    fn default() -> Self {
        Self {
            server: String::new(),
            token: String::new(),
            llm: true,
            candidates: false,
            logs: true,
            sync: true,
        }
    }
}

impl CloudConfig {
    /// 读不了、格式不对或没填全都当没配置，键盘照常离线用。
    pub fn load(path: &Path) -> Option<Self> {
        let text = std::fs::read_to_string(path).ok()?;
        let config: Self = toml::from_str(&text)
            .inspect_err(|error| tracing::warn!(%error, "cloud.toml 格式不对，按离线用"))
            .ok()?;
        (!config.server.trim().is_empty() && !config.token.trim().is_empty()).then_some(config)
    }

    /// 大模型代理的接口地址（OpenAI 兼容，不含 `/chat/completions`）。
    pub fn llm_base_url(&self) -> String {
        format!("{}/v1", self.server.trim().trim_end_matches('/'))
    }
}
