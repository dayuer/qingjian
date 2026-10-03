//! 连接状态，壳用来改菜单栏图标与提示。

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Status {
    /// 刚启动，还没连上过。
    Connecting,

    /// SSE 连着或刚刚请求成功。
    Online,

    /// 连不上，正在按退避重试；带最近一次的错误。
    Offline(String),

    /// 令牌被拒，要用户改配置。
    Unauthorized,
}
