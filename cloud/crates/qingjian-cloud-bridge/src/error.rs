//! 打开会话时可能出的错；C ABI 那边只拿到空指针，原因写进日志。

use thiserror::Error;

#[derive(Debug, Error)]
pub enum BridgeError {
    #[error("failed to open dictionary: {0}")]
    Dictionary(#[from] qingjian_dictionary::DictionaryError),
}
