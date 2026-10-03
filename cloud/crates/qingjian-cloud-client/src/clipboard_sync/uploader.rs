//! 上传线程：把离线队列逐条发出去；成功后顺带补拉一次。

use std::sync::Arc;
use std::time::Duration;

use super::{Backoff, Shared};
use crate::ClientError;

/// 队列空时多久醒一次检查是否该退出。
const IDLE: Duration = Duration::from_secs(30);

/// 令牌被拒后多久再试（用户可能在服务端重新登记了同名设备）。
const UNAUTHORIZED_RETRY: Duration = Duration::from_secs(300);

pub fn spawn(shared: Arc<Shared>) {
    std::thread::Builder::new()
        .name("cloud-upload".to_owned())
        .spawn(move || run(&shared))
        .expect("spawn upload thread");
}

fn run(shared: &Shared) {
    let mut backoff = Backoff::new();
    let mut delay = Duration::ZERO;
    while !shared.stopped() {
        if !delay.is_zero() {
            shared.wait_for_work(delay);
            delay = Duration::ZERO;
            continue;
        }
        let Some(clip) = shared.outbox().front().cloned() else {
            shared.wait_for_work(IDLE);
            continue;
        };
        match shared.client.push_clip(&clip) {
            Ok(event) => {
                backoff.reset();
                if let Err(error) = shared.outbox().pop_front() {
                    tracing::warn!(%error, "离线队列更新失败");
                }
                tracing::debug!(seq = event.seq, "剪贴板已上传");
                if let Err(error) = shared.catch_up() {
                    tracing::debug!(%error, "上传后补拉失败");
                }
            }
            Err(ClientError::Rejected { status, message }) => {
                tracing::warn!(status, %message, "服务端拒收这条剪贴板，丢弃");
                if let Err(error) = shared.outbox().pop_front() {
                    tracing::warn!(%error, "离线队列更新失败");
                }
            }
            Err(error) => {
                shared.set_error(&error);
                delay = match error {
                    ClientError::Unauthorized => UNAUTHORIZED_RETRY,
                    _ => backoff.next_delay(),
                };
            }
        }
    }
}
