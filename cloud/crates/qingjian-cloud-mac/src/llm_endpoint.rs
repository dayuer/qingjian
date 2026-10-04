//! 给输入法云联想用的大模型代理端点：青简 Cloud 服务器的 OpenAI 兼容接口与这台设备的令牌。

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LlmEndpoint {
    /// 不含 `/chat/completions` 的接口地址（`https://服务器/v1`）。
    pub base_url: String,

    /// 设备令牌，当 API 密钥用。
    pub token: String,
}

impl LlmEndpoint {
    pub fn new(server: &str, token: &str) -> Self {
        Self {
            base_url: format!("{}/v1", server.trim().trim_end_matches('/')),
            token: token.trim().to_owned(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::LlmEndpoint;

    #[test]
    fn strips_trailing_slash() {
        let endpoint = LlmEndpoint::new("https://pinyin.synon.ai/", " t ");
        assert_eq!(endpoint.base_url, "https://pinyin.synon.ai/v1");
        assert_eq!(endpoint.token, "t");
    }
}
