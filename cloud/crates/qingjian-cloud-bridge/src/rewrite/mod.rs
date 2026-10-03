//! 润色：把光标前的一段话经服务器的大模型代理改得更通顺，后台线程发请求，主线程轮询结果。
//! 同一时间只认最新一次：用户又点了润色或开始打字，旧请求回来也丢掉。

mod state;

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use qingjian_cloud_client::Client;
use serde_json::{Value, json};

pub use self::state::RewriteState;

const PROMPT: &str = "你是中文写作助手。把用户给的这段文字改得更通顺自然：修正错别字、语病和标点，\
保持原意、人称、语气和大致长度，不要添加新内容。只输出改好的文字，不要解释，不要加引号。";

/// 模型名只是占位：服务器配置了模型时以服务器的为准。
const MODEL: &str = "deepseek-v4-flash";

pub struct Rewriter {
    client: Client,

    state: Arc<Mutex<RewriteState>>,

    /// 每次开始或作废都加一，后台线程回来时对不上就不写结果。
    generation: Arc<AtomicU64>,
}

impl Rewriter {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            state: Arc::new(Mutex::new(RewriteState::Idle)),
            generation: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn start(&self, text: &str) {
        let text = text.trim().to_owned();
        if text.is_empty() {
            return;
        }
        let generation = self.generation.fetch_add(1, Ordering::SeqCst) + 1;
        *lock(&self.state) = RewriteState::Pending;
        let client = self.client.clone();
        let state = self.state.clone();
        let current = self.generation.clone();
        let spawned = std::thread::Builder::new()
            .name("cloud-rewrite".to_owned())
            .spawn(move || {
                let result = rewrite(&client, &text);
                if current.load(Ordering::SeqCst) != generation {
                    return;
                }
                *lock(&state) = match result {
                    Some(text) => RewriteState::Ready(text),
                    None => RewriteState::Failed,
                };
            });
        if spawned.is_err() {
            *lock(&self.state) = RewriteState::Failed;
        }
    }

    pub fn status(&self) -> u32 {
        lock(&self.state).code()
    }

    /// 取走结果（只在 Ready 时有），状态回到 Idle。
    pub fn take(&self) -> Option<String> {
        let mut state = lock(&self.state);
        match std::mem::replace(&mut *state, RewriteState::Idle) {
            RewriteState::Ready(text) => Some(text),
            other => {
                *state = other;
                None
            }
        }
    }

    pub fn cancel(&self) {
        self.generation.fetch_add(1, Ordering::SeqCst);
        *lock(&self.state) = RewriteState::Idle;
    }
}

fn rewrite(client: &Client, text: &str) -> Option<String> {
    let request = json!({
        "model": MODEL,
        "messages": [
            {"role": "system", "content": PROMPT},
            {"role": "user", "content": text},
        ],
        "temperature": 0.3,
        // 关掉思考：要的是快
        "reasoning_effort": "none",
        "stream": false,
    });
    let response = client
        .chat(&request)
        .inspect_err(|error| tracing::warn!(%error, "润色请求失败"))
        .ok()?;
    let content = response
        .pointer("/choices/0/message/content")
        .and_then(Value::as_str)?
        .trim()
        .trim_matches(|c| matches!(c, '"' | '“' | '”' | '「' | '」'))
        .trim();
    (!content.is_empty() && content != text).then(|| content.to_owned())
}

/// 后台线程 panic 过也接着用：状态只是一个枚举，不会半截。
fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}
