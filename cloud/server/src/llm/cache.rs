//! 回答缓存：相同的请求体（插完上文之后）在有效期内直接返回上次的回答，各设备共用。
//! 只缓存非流式、成功的回答。容量小，满了清掉最旧的一半。

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// 有效期。
const TTL: Duration = Duration::from_secs(600);

/// 最多缓存多少条。
const CAPACITY: usize = 512;

#[derive(Default)]
pub struct ResponseCache {
    entries: Mutex<HashMap<u64, (Instant, Vec<u8>)>>,
}

impl ResponseCache {
    pub fn get(&self, key: u64) -> Option<Vec<u8>> {
        let entries = self.entries.lock().unwrap_or_else(|p| p.into_inner());
        entries
            .get(&key)
            .filter(|(at, _)| at.elapsed() < TTL)
            .map(|(_, body)| body.clone())
    }

    pub fn insert(&self, key: u64, body: Vec<u8>) {
        let mut entries = self.entries.lock().unwrap_or_else(|p| p.into_inner());
        if entries.len() >= CAPACITY {
            let mut ages: Vec<(u64, Instant)> =
                entries.iter().map(|(k, (at, _))| (*k, *at)).collect();
            ages.sort_by_key(|(_, at)| *at);
            for (key, _) in ages.into_iter().take(CAPACITY / 2) {
                entries.remove(&key);
            }
        }
        entries.insert(key, (Instant::now(), body));
    }

    /// 请求体的指纹（FNV-1a）。
    pub fn key(body: &[u8]) -> u64 {
        body.iter().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
            (hash ^ u64::from(*byte)).wrapping_mul(0x0100_0000_01b3)
        })
    }
}
