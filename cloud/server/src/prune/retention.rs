//! 剪贴板的保留策略。

/// 保留策略。
#[derive(Debug, Clone, Copy)]
pub struct Retention {
    /// 最多保留多少条剪贴板。
    pub keep: usize,

    /// 最多保留多少天。
    pub days: u32,
}
