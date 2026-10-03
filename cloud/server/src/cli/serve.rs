//! `serve`：起 tokio 运行时、HTTP 服务与清理任务，收到 SIGTERM（`docker stop`）或 Ctrl-C 后优雅退出。

use std::net::SocketAddr;
use std::sync::Arc;

use clap::Args;

use crate::prune::{self, Retention};
use crate::{AppState, ServerError, Store, router};

#[derive(Debug, Args)]
pub struct ServeArgs {
    /// 监听地址。
    #[arg(long, env = "QINGJIAN_CLOUD_LISTEN", default_value = "0.0.0.0:8080")]
    pub listen: SocketAddr,

    /// 剪贴板最多保留多少条。
    #[arg(long, env = "QINGJIAN_CLOUD_CLIP_KEEP", default_value_t = 200)]
    pub clip_keep: usize,

    /// 剪贴板最多保留多少天。
    #[arg(long, env = "QINGJIAN_CLOUD_CLIP_DAYS", default_value_t = 30)]
    pub clip_days: u32,
}

pub fn run(store: Store, args: ServeArgs) -> Result<(), ServerError> {
    let runtime = tokio::runtime::Runtime::new()?;
    runtime.block_on(async move {
        let store = Arc::new(store);
        let retention = Retention {
            keep: args.clip_keep,
            days: args.clip_days,
        };
        tokio::spawn(prune::run(store.clone(), retention));
        let listener = tokio::net::TcpListener::bind(args.listen).await?;
        tracing::info!(listen = %args.listen, "青简 Cloud 已启动");
        axum::serve(listener, router(AppState::new(store)))
            .with_graceful_shutdown(shutdown())
            .await?;
        tracing::info!("已退出");
        Ok(())
    })
}

async fn shutdown() {
    let ctrl_c = tokio::signal::ctrl_c();
    #[cfg(unix)]
    {
        let mut term =
            match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
                Ok(term) => term,
                Err(_) => {
                    let _ = ctrl_c.await;
                    return;
                }
            };
        tokio::select! {
            _ = ctrl_c => {}
            _ = term.recv() => {}
        }
    }
    #[cfg(not(unix))]
    let _ = ctrl_c.await;
}
