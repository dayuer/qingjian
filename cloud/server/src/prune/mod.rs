//! 剪贴板保留策略：只留最近 N 条、且不超过 M 天，每小时清一次。

use std::sync::Arc;
use std::time::Duration;

use crate::Store;

mod retention;

pub use retention::Retention;

/// 清理间隔。
const INTERVAL: Duration = Duration::from_secs(3600);

/// 后台循环：启动时先清一次，之后按 [`INTERVAL`]。
pub async fn run(store: Arc<Store>, retention: Retention) {
    let mut ticker = tokio::time::interval(INTERVAL);
    loop {
        ticker.tick().await;
        let cutoff = crate::now_ms() - i64::from(retention.days) * 86_400_000;
        match store.prune(retention.keep, cutoff) {
            Ok(0) => {}
            Ok(removed) => tracing::info!(removed, "清理过期剪贴板"),
            Err(error) => tracing::warn!(%error, "清理剪贴板失败"),
        }
    }
}
