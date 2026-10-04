//! 登录成功的响应：会话令牌与账号、会话的 id。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionGrant {
    /// `sjt_` 开头的会话令牌，之后放在 `Authorization: Bearer …` 里。
    pub token: String,

    pub user_id: i64,

    pub session_id: i64,

    /// 这次登录新建了账号。
    pub new_user: bool,
}
