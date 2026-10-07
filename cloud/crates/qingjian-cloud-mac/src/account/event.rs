//! 账号操作的结果：建空间、配对轮询与切开关等后台线程经通道交给主线程拍子。

use std::fmt;

use qingjian_cloud_proto::{Consents, Feature, SessionInfo};

#[derive(Clone, PartialEq, Eq)]
pub enum AccountEvent {
    /// 开通或加入没成（网络、码不对、名额满等）；带给用户看的原因。
    JoinFailed(String),

    /// 拿到会话令牌（建空间成功或旧设备允许了），以及服务器上的开关（取不到为全关）。
    SignedIn {
        token: String,

        /// 账号 id，用来分辨「同账号重登」与「换账号」。
        user_id: i64,

        consents: Consents,
    },

    /// 服务器上的开关（切换成功或刷新账号时一起带回来）；刷新账号时还带上同一空间里的设备。
    Account {
        consents: Consents,

        /// 同一空间里的设备；只切开关时不带（`None` 表示设备列表照旧）。
        sessions: Option<Vec<SessionInfo>>,
    },

    /// 解绑了同一空间里的另一台设备（带给用户看的设备名）。
    Revoked(String),

    /// 切换开关时服务器说这项不能开（403）：显示为关。
    Forbidden(Feature),

    /// 令牌失效（401）：清掉令牌，回到未登录。
    SignedOut,

    /// 切换开关等操作失败：给用户看的失败原因，显示在菜单里。
    Failed(String),

    /// 云端输入记录已清空（本机日志与上传进度也清了），菜单里给一句确认。
    Cleared,
}

/// 手写 `Debug`：令牌与一次性码不能进日志。
impl fmt::Debug for AccountEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::JoinFailed(reason) => f.debug_tuple("JoinFailed").field(reason).finish(),
            Self::SignedIn {
                user_id, consents, ..
            } => f
                .debug_struct("SignedIn")
                .field("token", &"<hidden>")
                .field("user_id", user_id)
                .field("consents", consents)
                .finish(),
            Self::Account { consents, sessions } => f
                .debug_struct("Account")
                .field("consents", consents)
                .field("sessions", &sessions.as_ref().map(Vec::len))
                .finish(),
            Self::Revoked(name) => f.debug_tuple("Revoked").field(name).finish(),
            Self::Forbidden(feature) => f.debug_tuple("Forbidden").field(feature).finish(),
            Self::SignedOut => f.write_str("SignedOut"),
            Self::Failed(reason) => f.debug_tuple("Failed").field(reason).finish(),
            Self::Cleared => f.write_str("Cleared"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_hides_token() {
        let signed_in = AccountEvent::SignedIn {
            token: "sjt_SECRET".to_owned(),
            user_id: 7,
            consents: Consents::default(),
        };
        let text = format!("{signed_in:?}");
        assert!(!text.contains("SECRET"), "{text}");
    }
}
