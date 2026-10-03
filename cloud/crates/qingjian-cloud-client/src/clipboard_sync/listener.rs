//! 监听线程：先问清本机的设备名，再挂 SSE；一条连接到时限正常结束就立即重连，连不上按退避重试。

use std::sync::Arc;
use std::time::Duration;

use super::{Backoff, Shared, Status};
use crate::ClientError;
use crate::supervise::{Exit, supervise};

/// 令牌被拒后多久再试。
const UNAUTHORIZED_RETRY: Duration = Duration::from_secs(300);

/// 两次重连之间至少隔这么久，防止连接一建立就断时空转。
const MIN_RECONNECT: Duration = Duration::from_secs(1);

pub fn spawn(shared: Arc<Shared>) {
    std::thread::Builder::new()
        .name("cloud-listen".to_owned())
        .spawn(move || {
            supervise(
                "cloud-listen",
                || shared.stopped(),
                || {
                    run(&shared);
                    Exit::Stopped
                },
            )
        })
        .expect("spawn listen thread");
}

fn run(shared: &Shared) {
    let mut backoff = Backoff::new();
    while !shared.stopped() {
        match listen_once(shared) {
            // 连上过：时限到了或服务端重启，马上重连
            Ok(()) => {
                backoff.reset();
                sleep_unless_stopped(shared, MIN_RECONNECT);
            }
            Err(error) => {
                shared.set_error(&error);
                let delay = match error {
                    ClientError::Unauthorized => UNAUTHORIZED_RETRY,
                    _ => backoff.next_delay(),
                };
                sleep_unless_stopped(shared, delay);
            }
        }
    }
}

/// 连上后读到连接结束返回 `Ok`；没连上返回错误。
fn listen_once(shared: &Shared) -> Result<(), ClientError> {
    if !shared.device_known() {
        let whoami = shared.client.whoami()?;
        shared.set_device(whoami.device);
    }
    let stream = shared.client.stream(shared.cursor())?;
    shared.set_status(Status::Online);
    for event in stream {
        if shared.stopped() {
            break;
        }
        match event {
            Ok(event) => shared.deliver(event),
            // 读超时（连接时限）或断线：算正常结束，外层立即重连
            Err(error) => {
                tracing::debug!(%error, "SSE 连接结束");
                break;
            }
        }
    }
    Ok(())
}

/// 分小段睡，退出时不用等完整个退避。
fn sleep_unless_stopped(shared: &Shared, total: Duration) {
    let step = Duration::from_millis(200);
    let mut slept = Duration::ZERO;
    while slept < total && !shared.stopped() {
        std::thread::sleep(step);
        slept += step;
    }
}
