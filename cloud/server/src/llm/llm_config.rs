//! 代理的配置，来自命令行参数或环境变量（见 `cli/serve.rs`）。

#[derive(Debug, Clone)]
pub struct LlmConfig {
    /// 上游接口地址，不含 `/chat/completions`，如 `https://api.deepseek.com`。
    pub base_url: String,

    /// 上游密钥；为空时代理返回 503。
    pub api_key: String,

    /// 非空时覆盖请求里的模型名。
    pub model: Option<String>,

    /// 插进请求的跨设备上文最多多少个字符；0 表示不插。
    pub context_chars: usize,

    /// 等上游回答的超时（秒）。
    pub timeout_secs: u64,
}
