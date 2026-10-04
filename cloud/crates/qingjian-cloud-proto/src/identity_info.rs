//! 账号的一种登录方式。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdentityInfo {
    /// `apple` / `email`（以后 `google` / `x`）。
    pub provider: String,

    /// 给用户看的标识（邮箱地址；Apple 没给邮箱时为空）。
    pub label: Option<String>,
}
