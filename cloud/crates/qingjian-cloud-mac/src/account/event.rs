//! 账号操作的结果：建空间、配对轮询与切开关等后台线程经通道交给主线程拍子。

use std::fmt;

use qingjian_cloud_proto::{Consents, Feature};

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

    /// 服务器上的开关（切换成功或刷新得到）。
    Consents(Consents),

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
            Self::Consents(consents) => f.debug_tuple("Consents").field(consents).finish(),
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
