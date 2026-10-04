//! 账号操作的结果：登录窗口的回调与后台线程（换令牌、切开关、取账号）经通道交给主线程拍子。

use std::fmt;

use qingjian_cloud_proto::{Consents, Feature};

#[derive(Clone, PartialEq, Eq)]
pub enum AccountEvent {
    /// 网页登录窗口回跳：回跳的地址。
    Callback(String),

    /// 用户关掉登录窗口：静默回到未登录，不提示。
    Canceled,

    /// 登录没成：窗口出错，或用一次性码换令牌失败；带给用户看的原因。
    LoginFailed(String),

    /// 用一次性码换到了令牌，以及服务器上的开关（取不到为全关）。
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
}

/// 手写 `Debug`：令牌与一次性码不能进日志。
impl fmt::Debug for AccountEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Callback(_) => f.write_str("Callback(<url>)"),
            Self::Canceled => f.write_str("Canceled"),
            Self::LoginFailed(reason) => f.debug_tuple("LoginFailed").field(reason).finish(),
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
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_hides_token_and_handoff() {
        let signed_in = AccountEvent::SignedIn {
            token: "sjt_SECRET".to_owned(),
            user_id: 7,
            consents: Consents::default(),
        };
        let callback = AccountEvent::Callback("sujian://auth?handoff=SECRET".to_owned());
        let text = format!("{signed_in:?} {callback:?}");
        assert!(!text.contains("SECRET"), "{text}");
    }
}
