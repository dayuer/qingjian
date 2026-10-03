//! 大模型代理：假的上游服务验证换密钥、插上文、缓存、用量与流式透传。

mod common;

use std::sync::{Arc, Mutex};

use axum::Json;
use axum::body::Body;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use qingjian_cloud_server::Store;
use qingjian_cloud_server::llm::{LlmConfig, Upstream};
use serde_json::{Value, json};

use common::TestServer;

/// 假上游：记下收到的请求；`stream` 的回两段 SSE，否则回带 usage 的 JSON；密钥不对回 401。
fn fake_upstream(received: Arc<Mutex<Vec<Value>>>) -> (tokio::runtime::Runtime, String) {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let listener = runtime
        .block_on(tokio::net::TcpListener::bind("127.0.0.1:0"))
        .unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let app = axum::Router::new().route(
        "/chat/completions",
        post(move |headers: HeaderMap, Json(body): Json<Value>| {
            let received = received.clone();
            async move {
                if headers.get("authorization").and_then(|v| v.to_str().ok())
                    != Some("Bearer sk-upstream")
                {
                    return (StatusCode::UNAUTHORIZED, "bad key").into_response();
                }
                let stream = body["stream"].as_bool().unwrap_or(false);
                received.lock().unwrap().push(body);
                if stream {
                    return Response::builder()
                        .header("content-type", "text/event-stream")
                        .body(Body::from("data: {\"choices\":[]}\n\ndata: [DONE]\n\n"))
                        .unwrap();
                }
                Json(json!({
                    "choices": [{"message": {"role": "assistant", "content": "青简"}}],
                    "usage": {"prompt_tokens": 12, "completion_tokens": 3}
                }))
                .into_response()
            }
        }),
    );
    runtime.spawn(async move { axum::serve(listener, app).await.unwrap() });
    (runtime, url)
}

fn server_with_llm(upstream_url: &str, context_chars: usize) -> TestServer {
    let store = Arc::new(Store::in_memory().unwrap());
    let upstream = Upstream::new(LlmConfig {
        base_url: upstream_url.to_owned(),
        api_key: "sk-upstream".to_owned(),
        model: Some("deepseek-v4-flash".to_owned()),
        context_chars,
        timeout_secs: 5,
    })
    .unwrap();
    TestServer::start_with(common::free_addr(), store, |state| state.with_llm(upstream))
}

fn chat(url: &str, token: &str, body: Value) -> (u16, String) {
    let agent = ureq::Agent::config_builder()
        .http_status_as_error(false)
        .build()
        .new_agent();
    let mut response = agent
        .post(format!("{url}/v1/chat/completions"))
        .header("Authorization", format!("Bearer {token}"))
        .send_json(body)
        .unwrap();
    (
        response.status().as_u16(),
        response.body_mut().read_to_string().unwrap(),
    )
}

fn request(stream: bool) -> Value {
    json!({
        "model": "whatever",
        "stream": stream,
        "messages": [{"role": "system", "content": "你是输入法"}, {"role": "user", "content": "nihao"}]
    })
}

#[test]
fn proxies_with_server_key_caches_and_counts_usage() {
    let received = Arc::new(Mutex::new(Vec::new()));
    let (_upstream, upstream_url) = fake_upstream(received.clone());
    let server = server_with_llm(&upstream_url, 0);
    let token = server.store.add_device("mac").unwrap();

    let (status, body) = chat(&server.url, &token, request(false));
    assert_eq!(status, 200, "{body}");
    assert!(body.contains("青简"));
    // 模型名被服务器配置覆盖，没插上文
    let first = received.lock().unwrap()[0].clone();
    assert_eq!(first["model"], "deepseek-v4-flash");
    assert_eq!(first["messages"].as_array().unwrap().len(), 2);

    // 同样的请求第二次走缓存，上游只收到一次
    let (status, again) = chat(&server.url, &token, request(false));
    assert_eq!((status, again), (200, body));
    assert_eq!(received.lock().unwrap().len(), 1);

    let usage = server.store.usage(1).unwrap();
    assert_eq!(usage.len(), 1);
    assert_eq!((usage[0].requests, usage[0].cached), (2, 1));
    assert_eq!(
        (usage[0].prompt_tokens, usage[0].completion_tokens),
        (12, 3)
    );

    // 设备令牌不对：401，不碰上游
    let (status, _) = chat(&server.url, "qjc_wrong", request(false));
    assert_eq!(status, 401);
    assert_eq!(received.lock().unwrap().len(), 1);
}

#[test]
fn streams_through_unchanged() {
    let received = Arc::new(Mutex::new(Vec::new()));
    let (_upstream, upstream_url) = fake_upstream(received);
    let server = server_with_llm(&upstream_url, 0);
    let token = server.store.add_device("mac").unwrap();
    let (status, body) = chat(&server.url, &token, request(true));
    assert_eq!(status, 200);
    assert_eq!(body, "data: {\"choices\":[]}\n\ndata: [DONE]\n\n");
}

#[test]
fn injects_recent_text_from_all_devices_when_enabled() {
    let received = Arc::new(Mutex::new(Vec::new()));
    let (_upstream, upstream_url) = fake_upstream(received.clone());
    let server = server_with_llm(&upstream_url, 20);
    let mac = server.store.add_device("mac").unwrap();
    let phone = server.store.add_device("iphone").unwrap();
    let log = qingjian_cloud_client::Client::new(&server.url, &phone);
    log.push_input_log(&qingjian_cloud_proto::InputLogPush {
        batch_id: "0:0".to_owned(),
        lines: vec![
            r#"{"t":"x","event":"session","v":1,"version":"0.1","platform":"ios","model":false,"scheme":""}"#.to_owned(),
            r#"{"t":"x","event":"commit","id":1,"keys":"mingtian","pinyin":"ming'tian","corrected":false,"text":"明天","source":"sentence","index":0,"top":[],"scheme":"","english":false}"#.to_owned(),
            r#"{"t":"x","event":"passthrough","text":"，"}"#.to_owned(),
            r#"{"t":"x","event":"commit","id":2,"keys":"kaihui","pinyin":"kai'hui","corrected":false,"text":"开会","source":"sentence","index":0,"top":[],"scheme":"","english":false}"#.to_owned(),
        ],
    })
    .unwrap();
    let (status, _) = chat(&server.url, &mac, request(false));
    assert_eq!(status, 200);
    let sent = received.lock().unwrap()[0].clone();
    let messages = sent["messages"].as_array().unwrap();
    assert_eq!(messages.len(), 3);
    assert_eq!(messages[1]["role"], "system");
    assert!(
        messages[1]["content"]
            .as_str()
            .unwrap()
            .ends_with("明天，开会")
    );
}

#[test]
fn without_key_the_proxy_is_unavailable() {
    let server = TestServer::start();
    let token = server.store.add_device("mac").unwrap();
    let (status, _) = chat(&server.url, &token, request(false));
    assert_eq!(status, 503);
}
