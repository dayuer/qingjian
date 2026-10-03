//! `serve`：起 tokio 运行时、HTTP 服务与清理任务，收到 SIGTERM（`docker stop`）或 Ctrl-C 后优雅退出。

use std::net::SocketAddr;
use std::sync::Arc;

use clap::Args;

use crate::llm::{LlmConfig, Upstream};
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

    /// 上游大模型接口地址（不含 /chat/completions）。
    #[arg(
        long,
        env = "QINGJIAN_LLM_BASE_URL",
        default_value = "https://api.deepseek.com"
    )]
    pub llm_base_url: String,

    /// 上游大模型密钥；不填则不提供代理。
    #[arg(
        long,
        env = "QINGJIAN_LLM_API_KEY",
        default_value = "",
        hide_env_values = true
    )]
    pub llm_api_key: String,

    /// 覆盖输入法请求里的模型名；不填则用输入法设置里的。
    #[arg(long, env = "QINGJIAN_LLM_MODEL")]
    pub llm_model: Option<String>,

    /// 插进请求的跨设备上文最多多少字；0 为不插。
    #[arg(long, env = "QINGJIAN_LLM_CONTEXT_CHARS", default_value_t = 0)]
    pub llm_context_chars: usize,

    /// 等上游回答的超时（秒）。
    #[arg(long, env = "QINGJIAN_LLM_TIMEOUT_SECS", default_value_t = 30)]
    pub llm_timeout_secs: u64,
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
        let mut state = AppState::new(store);
        let llm = LlmConfig {
            base_url: args.llm_base_url,
            api_key: args.llm_api_key,
            model: args.llm_model.filter(|m| !m.trim().is_empty()),
            context_chars: args.llm_context_chars,
            timeout_secs: args.llm_timeout_secs,
        };
        if llm.api_key.trim().is_empty() {
            tracing::info!("没有配置 QINGJIAN_LLM_API_KEY，不提供大模型代理");
        } else {
            tracing::info!(upstream = %llm.base_url, context = llm.context_chars, "大模型代理已启用");
            let upstream = Upstream::new(llm).map_err(|e| ServerError::Upstream(e.to_string()))?;
            state = state.with_llm(upstream);
        }
        tracing::info!(listen = %args.listen, "青简 Cloud 已启动");
        axum::serve(listener, router(state))
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
