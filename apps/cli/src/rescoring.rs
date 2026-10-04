//! 异步重打分在 CLI 里的等法：壳是停顿后请求、定时器轮询，这里没有停顿，查询完直接请求并阻塞等结果，再查一次。

use std::time::{Duration, Instant};

use qingjian_core::Engine;

/// 最多等多久。
const WAIT: Duration = Duration::from_secs(5);

/// 有整句路径（或词级候选）还没拿到神经分就请求并等到全部后台任务回来；返回是否收到过新分（收到了调用方该重新查询）。
/// 通变和知微是两条后台线程，先回来的不代表另一个也到了，所以等 `rescoring_in_flight` 归零而不是第一次收到结果。
pub fn settle(engine: &mut Engine) -> bool {
    if !engine.rescoring_pending() || !engine.request_rescoring() {
        return false;
    }
    let started = Instant::now();
    let mut updated = false;
    while engine.rescoring_in_flight() {
        if engine.poll_rescoring() {
            updated = true;
            continue;
        }
        if started.elapsed() > WAIT {
            tracing::warn!("等重打分超时");
            break;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    updated
}
