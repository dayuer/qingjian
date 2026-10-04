//! Apple 登录的发起方：决定服务端用哪个 client_id 核对 `identity_token` 的 `aud`。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AppleClient {
    /// iOS App：client_id 是 App 的 bundle id。
    Ios,

    /// 网页登录页：client_id 是 Services ID。
    Web,
}
