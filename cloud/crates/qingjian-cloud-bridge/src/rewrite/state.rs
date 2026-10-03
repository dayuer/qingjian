//! 一次润色走到哪了；数值就是 C 接口 `qj_rewrite_status` 的返回值。

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RewriteState {
    Idle,

    Pending,

    Ready(String),

    /// 网络错误、服务器没开大模型，或模型没给出可用的文字。
    Failed,
}

impl RewriteState {
    pub fn code(&self) -> u32 {
        match self {
            Self::Idle => 0,
            Self::Pending => 1,
            Self::Ready(_) => 2,
            Self::Failed => 3,
        }
    }
}
