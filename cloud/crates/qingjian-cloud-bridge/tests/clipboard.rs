//! 剪贴板提示的取舍：跳过自己复制的、太旧的、处理过的，被删的撤回。用一个只会答 whoami / events 的假服务器。

use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use qingjian_cloud_bridge::Clipboard;
use qingjian_cloud_client::Client;

fn now_ms() -> i64 {
    i64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis(),
    )
    .unwrap()
}

/// 起一个假服务器：whoami 答 `iphone`，events 答给定的事件（不管 since，测试里只拉一页）。
fn serve(events: String) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = format!("http://{}", listener.local_addr().unwrap());
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut request_line = String::new();
            reader.read_line(&mut request_line).unwrap();
            // 读完请求头
            loop {
                let mut line = String::new();
                if reader.read_line(&mut line).unwrap() == 0 || line == "\r\n" {
                    break;
                }
            }
            let body = if request_line.contains("/v1/whoami") {
                r#"{"device":"iphone","latest":0}"#.to_owned()
            } else {
                events.clone()
            };
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = stream.write_all(response.as_bytes());
        }
    });
    address
}

fn clip(seq: u64, device: &str, text: &str, at: i64) -> String {
    format!(
        r#"{{"seq":{seq},"device":"{device}","at":{at},"type":"clip_added","client_id":"c{seq}","text":"{text}"}}"#
    )
}

fn wait_for_offer(clipboard: &Clipboard) -> Option<qingjian_cloud_bridge::ClipOffer> {
    for _ in 0..50 {
        std::thread::sleep(Duration::from_millis(20));
        if let Some(offer) = clipboard.offer() {
            return Some(offer);
        }
    }
    None
}

#[test]
fn offers_the_newest_fresh_clip_from_another_device() {
    let now = now_ms();
    let events = format!(
        r#"{{"events":[{},{},{}],"latest":3}}"#,
        clip(1, "macbook", "旧的", now - 60 * 60 * 1000),
        clip(2, "macbook", "123456", now - 5_000),
        clip(3, "iphone", "自己复制的", now - 1_000),
    );
    let server = serve(events);
    let dir = std::env::temp_dir().join(format!("qj-clip-{}", std::process::id()));
    let state = dir.join("cloud/clipboard.json");
    let clipboard = Clipboard::new(Client::new(&server, "t"), state.clone());
    clipboard.refresh();
    let offer = wait_for_offer(&clipboard).expect("应该有提示");
    assert_eq!(offer.text, "123456");
    assert_eq!(offer.device, "macbook");

    // 处理过（插入或关掉）就不再给，重开键盘（新进程读进度）也一样
    clipboard.handled();
    assert!(clipboard.offer().is_none());
    let reopened = Clipboard::new(Client::new(&server, "t"), state);
    reopened.refresh();
    assert!(wait_for_offer(&reopened).is_none());
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn deleted_clip_is_withdrawn() {
    // 先确认同样的数据不删时是有提示的，免得「没提示」只是因为解析失败
    let now = now_ms();
    let kept = serve(format!(
        r#"{{"events":[{}],"latest":1}}"#,
        clip(1, "macbook", "留着", now - 1_000)
    ));
    let dir = std::env::temp_dir().join(format!("qj-clip-kept-{}", std::process::id()));
    let clipboard = Clipboard::new(Client::new(&kept, "t"), dir.join("cloud/clipboard.json"));
    clipboard.refresh();
    assert_eq!(
        wait_for_offer(&clipboard).map(|o| o.text).as_deref(),
        Some("留着")
    );
    std::fs::remove_dir_all(&dir).ok();

    let now = now_ms();
    let events = format!(
        r#"{{"events":[{},{{"seq":2,"device":"macbook","at":{now},"type":"clip_deleted","target":1}}],"latest":2}}"#,
        clip(1, "macbook", "删掉了", now - 1_000),
    );
    let server = serve(events);
    let dir = std::env::temp_dir().join(format!("qj-clip-del-{}", std::process::id()));
    let clipboard = Clipboard::new(Client::new(&server, "t"), dir.join("cloud/clipboard.json"));
    clipboard.refresh();
    assert!(wait_for_offer(&clipboard).is_none());
    std::fs::remove_dir_all(&dir).ok();
}
