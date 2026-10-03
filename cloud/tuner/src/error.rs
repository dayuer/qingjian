//! 纠错闭环的错误。

#[derive(Debug, thiserror::Error)]
pub enum TunerError {
    #[error("server: {0}")]
    Server(#[from] qingjian_cloud_client::ClientError),

    #[error("LLM: {0}")]
    Llm(String),

    #[error("qingjian-cli: {0}")]
    Cli(String),

    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}
