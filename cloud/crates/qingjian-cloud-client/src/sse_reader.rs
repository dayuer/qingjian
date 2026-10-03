//! 最小的 SSE 解析：只认 `data:` 行（多行拼接）与空行分隔，忽略 `id:` / `event:` 与 `:` 开头的保活注释。
//! 每个 `data` 是一条 JSON 的 [`Event`]，`seq` 已经在里面，用不着 `id:`。

use std::io::BufRead;

use qingjian_cloud_proto::Event;

use crate::ClientError;

pub struct SseReader<R> {
    reader: R,

    line: String,
}

impl<R: BufRead> SseReader<R> {
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            line: String::new(),
        }
    }
}

impl<R: BufRead> Iterator for SseReader<R> {
    type Item = Result<Event, ClientError>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut data = String::new();
        loop {
            self.line.clear();
            match self.reader.read_line(&mut self.line) {
                // 连接正常结束；半截事件丢掉，重连后按 seq 补
                Ok(0) => return None,
                Ok(_) => {}
                Err(error) => return Some(Err(ClientError::Unreachable(error.to_string()))),
            }
            let line = self.line.trim_end_matches(['\r', '\n']);
            if line.is_empty() {
                if data.is_empty() {
                    continue;
                }
                return Some(
                    serde_json::from_str(&data)
                        .map_err(|e| ClientError::BadResponse(e.to_string())),
                );
            }
            if let Some(value) = line.strip_prefix("data:") {
                if !data.is_empty() {
                    data.push('\n');
                }
                data.push_str(value.strip_prefix(' ').unwrap_or(value));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_events_and_skips_comments() {
        let body = ": keep-alive\n\nid: 1\ndata: {\"seq\":1,\"device\":\"mac\",\"at\":5,\"type\":\"clip_deleted\",\"target\":9}\n\n:\n\ndata: {\"seq\":2,\"device\":\"mac\",\"at\":6,\ndata: \"type\":\"clip_deleted\",\"target\":3}\n\n";
        let events: Vec<_> = SseReader::new(body.as_bytes())
            .map(|r| r.unwrap().seq)
            .collect();
        assert_eq!(events, [1, 2]);
    }
}
