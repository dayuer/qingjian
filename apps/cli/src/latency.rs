//! 耗时分位数：回放与评测报告「按键同步部分」的 p50 / p99 用。

use std::time::Duration;

/// 一组耗时，攒着算分位数。
#[derive(Debug, Default, Clone)]
pub struct Latencies(Vec<Duration>);

impl Latencies {
    pub fn push(&mut self, elapsed: Duration) {
        self.0.push(elapsed);
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// `quantile` 在 0 到 1 之间；空集返回 0。最近秩法：排好序取第 ⌈q·n⌉ 个。
    pub fn quantile(&self, quantile: f64) -> Duration {
        if self.0.is_empty() {
            return Duration::ZERO;
        }
        let mut sorted = self.0.clone();
        sorted.sort();
        let rank = ((quantile * sorted.len() as f64).ceil() as usize).clamp(1, sorted.len());
        sorted[rank - 1]
    }

    /// 报告里的一句：`p50 0.8 ms / p99 3.1 ms`。
    pub fn summary(&self) -> String {
        format!(
            "p50 {:.1} ms / p99 {:.1} ms",
            self.quantile(0.5).as_secs_f64() * 1000.0,
            self.quantile(0.99).as_secs_f64() * 1000.0,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quantiles_use_nearest_rank() {
        let mut latencies = Latencies::default();
        for ms in [5, 1, 3, 2, 4] {
            latencies.push(Duration::from_millis(ms));
        }
        assert_eq!(latencies.quantile(0.5), Duration::from_millis(3));
        assert_eq!(latencies.quantile(0.99), Duration::from_millis(5));
        assert_eq!(latencies.quantile(0.0), Duration::from_millis(1));
        assert_eq!(Latencies::default().quantile(0.5), Duration::ZERO);
    }
}
