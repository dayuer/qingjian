//! 测试共用：本地假服务，每个连接回一个固定状态行，并把收到的请求头转给测试。
//! 同时被 `src/test_support.rs` 以 `#[path]` 引入，crate 内测试也能用。

#![allow(dead_code)]

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::sync::mpsc;

/// 起一个假服务，返回地址与请求头的接收端。`status_line` 如 `"403 Forbidden"`。
pub fn fake_server(status_line: &'static str) -> (String, mpsc::Receiver<String>) {
    fake_server_with_body(status_line, "")
}

/// 同 [`fake_server`]，响应带一个 JSON 体。
pub fn fake_server_with_body(
    status_line: &'static str,
    body: &'static str,
) -> (String, mpsc::Receiver<String>) {
    fake_server_sequence(vec![(status_line, body)])
}

/// 每次连接按顺序取一条 `(状态行, 响应体)`；用完之后一直用最后一条。
/// 一个操作要发好几条请求、每条的回应不同时用它（例如建空间之后接着问一次开关）。
pub fn fake_server_sequence(
    responses: Vec<(&'static str, &'static str)>,
) -> (String, mpsc::Receiver<String>) {
    assert!(!responses.is_empty(), "至少给一条响应");
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let mut responses = responses.into_iter();
        let mut current = responses.next().unwrap();
        for stream in listener.incoming().flatten() {
            let mut head = String::new();
            let mut reader = BufReader::new(&stream);
            loop {
                let mut line = String::new();
                if reader.read_line(&mut line).unwrap_or(0) == 0 || line == "\r\n" {
                    break;
                }
                head.push_str(&line);
            }
            // 把请求体读完再回：带着没读的数据关连接会让客户端收到 RST（macOS 上是 EINVAL）
            let length = head
                .lines()
                .find_map(|line| {
                    line.to_ascii_lowercase()
                        .strip_prefix("content-length:")?
                        .trim()
                        .parse()
                        .ok()
                })
                .unwrap_or(0u64);
            let mut request_body = String::new();
            reader
                .by_ref()
                .take(length)
                .read_to_string(&mut request_body)
                .ok();
            // 请求头之后空一行接请求体，测试可以断言请求里发了什么
            head.push_str("\r\n");
            head.push_str(&request_body);
            tx.send(head).ok();
            let (status_line, body) = current;
            write!(
                &stream,
                "HTTP/1.1 {status_line}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )
            .ok();
            current = responses.next().unwrap_or(current);
        }
    });
    (url, rx)
}

/// 请求头的第一行，如 `DELETE /v1/account HTTP/1.1`。
pub fn request_line(head: &str) -> &str {
    head.lines().next().unwrap_or_default()
}

/// 请求头里有没有 `Authorization`（不区分大小写）。
pub fn has_authorization(head: &str) -> bool {
    head.lines()
        .any(|line| line.to_ascii_lowercase().starts_with("authorization:"))
}
