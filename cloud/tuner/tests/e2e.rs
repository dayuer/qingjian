//! 端到端：真服务器 + 假的大模型上游 + 上游真正的 qingjian-cli 与产品数据，跑一轮纠错闭环。
//! 需要 QINGJIAN_CLI（qingjian-cli 可执行文件）与 QINGJIAN_DATA（…/data/generated）；没有就跳过。

use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;

use axum::Json;
use axum::routing::post;
use qingjian_cloud_client::Client;
use qingjian_cloud_proto::{InputLogPush, LearningPush, SetPut};
use qingjian_cloud_server::llm::{LlmConfig, Upstream};
use qingjian_cloud_server::{AppState, Store, router};
use serde_json::{Value, json};

fn tools() -> Option<(PathBuf, PathBuf)> {
    let cli = PathBuf::from(std::env::var_os("QINGJIAN_CLI")?);
    let data = PathBuf::from(std::env::var_os("QINGJIAN_DATA")?);
    (cli.exists() && data.join("dict.qj").exists()).then_some((cli, data))
}

/// 假上游：按系统提示词认出是哪一步，给固定的回答。
fn answer(body: &Value) -> String {
    let system = body["messages"][0]["content"].as_str().unwrap_or_default();
    let reply = if system.contains("词库审校") {
        json!([{"word": "我的", "action": "fix", "pinyin": "wo de", "reason": "的 在这里读 de"},
               {"word": "不存在的词", "action": "delete", "reason": "不在列表里，应被忽略"}])
    } else if system.contains("词库编辑") {
        json!([{"text": "青简", "pinyin": "qing jian", "reason": "产品名"},
               {"text": "青简", "pinyin": "qing jia", "reason": "拼音对不上键，应被丢弃"}])
    } else {
        json!([{"text": "含章知微", "pinyin": "han zhang zhi wei", "reason": "用户在写模型的事"},
               {"text": "试过", "pinyin": "shi guo", "reason": "常用词，现在就打得出来，应被跳过"}])
    };
    reply.to_string()
}

fn commit(id: u32, scope: &str, keys: &str, text: &str, index: u32) -> String {
    json!({"t": "x", "event": "commit", "id": id, "scope": scope, "keys": keys, "pinyin": "", "corrected": false,
           "text": text, "source": "word", "index": index, "top": [], "scheme": "", "english": false})
    .to_string()
}

#[test]
fn one_round_fixes_learns_and_passes_the_gate() {
    let Some((cli, data)) = tools() else {
        eprintln!("跳过：没有 QINGJIAN_CLI / QINGJIAN_DATA");
        return;
    };
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    // 假上游
    let upstream_listener = runtime
        .block_on(tokio::net::TcpListener::bind("127.0.0.1:0"))
        .unwrap();
    let upstream_url = format!("http://{}", upstream_listener.local_addr().unwrap());
    let fake = axum::Router::new().route(
        "/chat/completions",
        post(|Json(body): Json<Value>| async move {
            Json(json!({"choices": [{"message": {"role": "assistant", "content": answer(&body)}}]}))
        }),
    );
    runtime.spawn(async move { axum::serve(upstream_listener, fake).await.unwrap() });

    // 服务器
    let store = Arc::new(Store::in_memory().unwrap());
    let upstream = Upstream::new(LlmConfig {
        base_url: upstream_url,
        api_key: "sk".to_owned(),
        model: None,
        context_chars: 0,
        timeout_secs: 10,
    })
    .unwrap();
    let listener = runtime
        .block_on(tokio::net::TcpListener::bind("127.0.0.1:0"))
        .unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let app = router(AppState::new(store.clone()).with_llm(upstream));
    runtime.spawn(async move { axum::serve(listener, app).await.unwrap() });

    // 一台 Mac：学过一个读音错的用户词；日志里两次把 qingjian 分两步选成「青简」，留出段里又打了一次
    let mac = Client::new(&url, &store.add_device("mac").unwrap());
    mac.push_learning(&LearningPush {
        puts: vec![SetPut {
            table: "words".to_owned(),
            key: "我的".to_owned(),
            value: "wo di".to_owned(),
        }],
        ..LearningPush::default()
    })
    .unwrap();
    let filler = "我们今天讨论含章知微模型的训练进度和评测结果，".repeat(20);
    let mut lines = vec![
        json!({"t": "x", "event": "session", "v": 1, "version": "0.1.5", "platform": "macos", "model": false, "scheme": ""}).to_string(),
        commit(1, "qingjian", "qing", "青", 0),
        commit(2, "jian", "jian", "简", 0),
        json!({"t": "x", "event": "passthrough", "text": filler}).to_string(),
        commit(3, "qingjian", "qing", "青", 0),
        commit(4, "jian", "jian", "简", 0),
        commit(5, "shiguo", "shiguo", "试过", 0),
    ];
    // 留出段：整段 qingjian 选了第 2 个「青简」
    lines.push(commit(6, "qingjian", "qingjian", "青简", 1));
    lines.push(commit(7, "shiguo", "shiguo", "试过", 0));
    mac.push_input_log(&InputLogPush {
        batch_id: "0:0".to_owned(),
        lines,
    })
    .unwrap();

    let tuner_token = store.add_device("tuner").unwrap();
    let state = tempfile::tempdir().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_qingjian-cloud-tuner"))
        .env("QINGJIAN_CLOUD_SERVER", &url)
        .env("QINGJIAN_TUNER_TOKEN", &tuner_token)
        .env("QINGJIAN_CLI", &cli)
        .env("QINGJIAN_DATA", &data)
        .env("QINGJIAN_TUNER_STATE", state.path())
        .args(["--train-ratio", "0.7", "--min-holdout", "2"])
        .output()
        .unwrap();
    let report = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "{report}\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    eprintln!("{report}");
    assert!(report.contains("回放门槛通过"), "{report}");
    // 留出段里的 qingjian → 青简 修正前不是首选，修正后是
    assert!(
        report.contains("50.0%（1 / 2） → 100.0%（2 / 2）"),
        "{report}"
    );
    assert!(report.contains("改读音 我的：wo di → wo de"));
    assert!(report.contains("加词 青简（qing jian，键 qingjian）"));
    assert!(report.contains("加词 含章知微"));
    assert!(!report.contains("加词 试过"));
    assert!(!report.contains("不存在的词"));

    // 推上去的修正，Mac 拉学习数据能看到
    let rows = mac.learning(0, 500).unwrap().rows;
    let value = |key: &str| {
        rows.iter()
            .find(|r| r.table == "words" && r.key == key)
            .and_then(|r| r.value.clone())
    };
    assert_eq!(value("我的").as_deref(), Some("wo de"));
    assert_eq!(value("青简").as_deref(), Some("qing jian"));
    assert!(
        rows.iter()
            .any(|r| r.table == "choices" && r.key == "qingjian\t青简" && r.count == 1)
    );
    assert!(
        rows.iter()
            .any(|r| r.table == "ngram" && r.key == "<s>\t青简" && r.count == 1)
    );
    assert_eq!(
        store
            .usage(1)
            .unwrap()
            .iter()
            .find(|u| u.device == "tuner")
            .unwrap()
            .requests,
        3
    );

    // 第二轮：审过的不再审、问过的不再问
    let again = Command::new(env!("CARGO_BIN_EXE_qingjian-cloud-tuner"))
        .env("QINGJIAN_CLOUD_SERVER", &url)
        .env("QINGJIAN_TUNER_TOKEN", &tuner_token)
        .env("QINGJIAN_CLI", &cli)
        .env("QINGJIAN_DATA", &data)
        .env("QINGJIAN_TUNER_STATE", state.path())
        .args(["--train-ratio", "0.7", "--min-holdout", "2"])
        .output()
        .unwrap();
    let report = String::from_utf8_lossy(&again.stdout);
    assert!(report.contains("没有要改的"), "{report}");
}
