//! 上传线程：把离线队列逐条发出去；成功后顺带补拉一次。

use std::sync::Arc;
use std::time::Duration;

use super::backoff::retry_delay;
use super::{Backoff, Shared};
use crate::ClientError;
use crate::supervise::{Exit, supervise};

/// 队列空时多久醒一次检查是否该退出。
const IDLE: Duration = Duration::from_secs(30);

pub fn spawn(shared: Arc<Shared>) {
    std::thread::Builder::new()
        .name("cloud-upload".to_owned())
        .spawn(move || {
            supervise(
                "cloud-upload",
                || shared.stopped(),
                || {
                    run(&shared);
                    Exit::Stopped
                },
            )
        })
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
            // 服务器上关了剪贴板说明用户撤回了同意：攒着的明文不留，重新打开时也不补传
            Err(error @ ClientError::Forbidden(_)) => {
                shared.set_error(&error);
                if let Err(error) = shared.outbox().clear() {
                    tracing::warn!(%error, "离线队列清空失败");
                }
                delay = retry_delay(&error, &mut backoff);
            }
            Err(error) => {
                shared.set_error(&error);
                delay = retry_delay(&error, &mut backoff);
            }
        }
    }
}
