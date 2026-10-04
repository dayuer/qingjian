//! 账号操作的结果：登录窗口的回调与后台线程（换令牌、切开关、取账号）经通道交给主线程拍子。

use qingjian_cloud_proto::{Consents, Feature};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccountEvent {
    /// 网页登录窗口结束：回跳的地址，或失败 / 取消的原因。
    Callback(Result<String, String>),

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

    /// 给用户看的失败原因，显示在菜单里。
    Failed(String),
}
