//! 最近一次整句重排的分段耗时：真机验收时壳把它写进系统日志，看停键到重排的时间花在哪一段。

/// 一次整句重排（通变）从送出到被主线程取走的分段，毫秒。
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct RescoreStats {
    /// 送出后在打分线程队列里等了多久。
    pub queue_ms: f64,

    /// 前向打分本身。
    pub forward_ms: f64,

    /// 结果放进信道到主线程取走。
    pub main_ms: f64,

    /// 这次打了几条整句路径。
    pub paths: usize,

    /// 最长那条路径的字数。
    pub max_chars: usize,
}
