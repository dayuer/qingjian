//! 键盘连青简 Cloud 的配置（`cloud.toml`）：服务器地址与这台设备的令牌，外加两个开关。
//! 没有这个文件、或地址 / 令牌是空的，键盘就完全离线。

mod status;
mod switches;

use std::path::Path;

use serde::{Deserialize, Serialize};

pub use self::status::CloudStatus;
pub use self::switches::CloudSwitches;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct CloudConfig {
    /// 服务器地址，如 `https://pinyin.synon.ai`。
    pub server: String,

    /// 这台设备的令牌（服务器上 `qingjian-cloud device add <名字>` 生成）。
    pub token: String,

    /// 用服务器的大模型：润色，以及 `config.toml` 里 `[predict]` 开着时的云联想（与 Mac 同一个开关、同步）。
    pub llm: bool,

    /// 记输入日志并上传，服务器上的纠错闭环（tuner）靠它找打错、分几次才选完的地方。
    pub logs: bool,

    /// 与别的设备同步学习数据（词频、用户词、个人 n-gram 等）。
    pub sync: bool,

    /// 跨设备剪贴板：候选栏提示别的设备刚复制的文字，点按钮把本机剪贴板发出去。
    pub clipboard: bool,
}

impl Default for CloudConfig {
    fn default() -> Self {
        Self {
            server: String::new(),
            token: String::new(),
            llm: true,
            logs: true,
            sync: true,
            clipboard: true,
        }
    }
}

impl CloudConfig {
    /// 读不了、格式不对或没填全都当没配置，键盘照常离线用。
    pub fn load(path: &Path) -> Option<Self> {
        let config = Self::read(path)?;
        (!config.server.trim().is_empty() && !config.token.trim().is_empty()).then_some(config)
    }

    /// 设置页用：原样读出（没填全也读），读不了返回 `None`。
    pub fn read(path: &Path) -> Option<Self> {
        let text = std::fs::read_to_string(path).ok()?;
        toml::from_str(&text)
            .inspect_err(|error| tracing::warn!(%error, "cloud.toml 格式不对，按离线用"))
            .ok()
    }

    /// 设置页改开关：读出现有文件（没有按缺省），只换开关再写回，地址与令牌不动。
    pub fn save_switches(path: &Path, switches: CloudSwitches) -> Result<(), String> {
        let mut config = Self::read(path).unwrap_or_default();
        switches.apply_to(&mut config);
        config.save(path)
    }

    /// 整份写回（这份文件只有这几项，不用保留注释）。
    pub fn save(&self, path: &Path) -> Result<(), String> {
        let text = toml::to_string(self).map_err(|e| e.to_string())?;
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        std::fs::write(path, text).map_err(|e| e.to_string())
    }

    /// 大模型代理的接口地址（OpenAI 兼容，不含 `/chat/completions`）。
    pub fn llm_base_url(&self) -> String {
        format!("{}/v1", self.server.trim().trim_end_matches('/'))
    }
}
