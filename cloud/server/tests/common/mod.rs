//! 测试用：在后台线程起一个真服务端（内存库、随机端口），返回地址与存储。
#![allow(dead_code)] // 各测试文件只用到其中一部分

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use qingjian_cloud_server::{AppState, Store, router};

pub struct TestServer {
    pub url: String,

    pub store: Arc<Store>,

    runtime: Option<tokio::runtime::Runtime>,
}

impl TestServer {
    pub fn start() -> Self {
        Self::start_on(
            "127.0.0.1:0".parse().unwrap(),
            Arc::new(Store::in_memory().unwrap()),
        )
    }

    /// 指定地址与存储，用来模拟「服务器停了又起来」。
    pub fn start_on(addr: SocketAddr, store: Arc<Store>) -> Self {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();
        let listener = runtime
            .block_on(tokio::net::TcpListener::bind(addr))
            .unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let app = router(AppState::new(store.clone()));
        runtime.spawn(async move { axum::serve(listener, app).await.unwrap() });
        Self {
            url,
            store,
            runtime: Some(runtime),
        }
    }

    pub fn addr(&self) -> SocketAddr {
        self.url.trim_start_matches("http://").parse().unwrap()
    }

    /// 停掉服务（连接一并断开）。
    pub fn stop(mut self) -> Arc<Store> {
        if let Some(runtime) = self.runtime.take() {
            runtime.shutdown_timeout(Duration::from_secs(1));
        }
        self.store.clone()
    }
}

/// 一直试到 `check` 返回 `Some` 或超时。
pub fn wait_for<T>(timeout: Duration, mut check: impl FnMut() -> Option<T>) -> Option<T> {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if let Some(value) = check() {
            return Some(value);
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    None
}

/// 一个没被占用的本机端口。
pub fn free_addr() -> SocketAddr {
    std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
}
