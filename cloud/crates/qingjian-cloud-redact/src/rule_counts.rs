//! 各类规则各命中了几处，供服务端记计数、评测用。

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RuleCounts {
    pub phone: u32,

    pub landline: u32,

    pub id_card: u32,

    pub bank_card: u32,

    pub email: u32,

    pub url: u32,

    pub address: u32,

    /// 微信号、QQ 号。
    pub account: u32,
}
