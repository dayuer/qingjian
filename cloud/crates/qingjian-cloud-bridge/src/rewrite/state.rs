//! 一次改写走到哪了；数值就是 C 接口 `qj_rewrite_status` 的返回值。

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RewriteState {
    Idle,

    Pending,

    Ready(String),

    /// 网络错误、服务器没开大模型。
    Failed,

    /// 模型给的不合用（空的，或比原文长出一大截），已丢掉；键盘上跟网络失败的说法不一样。
    Rejected,
}

impl RewriteState {
    pub fn code(&self) -> u32 {
        match self {
            Self::Idle => 0,
            Self::Pending => 1,
            Self::Ready(_) => 2,
            Self::Failed => 3,
            Self::Rejected => 4,
        }
    }
}
