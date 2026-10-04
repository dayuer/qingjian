//! 菜单里账号那几行要的状态：登没登录、是不是正在登录、四个开关、最近一次失败的原因。

use qingjian_cloud_proto::Consents;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountMenu {
    pub signed_in: bool,

    /// 网页登录窗口开着，或正在用一次性码换令牌。
    pub signing_in: bool,

    pub consents: Consents,

    /// 最近一次登录或切换开关失败的原因。
    pub note: Option<String>,
}
