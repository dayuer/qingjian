//! 经服务器的大模型代理问问题（用量记在 tuner 这台「设备」名下），要求回一个 JSON 数组。

use std::time::Duration;

use serde_json::{Value, json};

use crate::error::TunerError;

pub struct Llm {
    agent: ureq::Agent,

    url: String,

    token: String,

    model: String,
}

impl Llm {
    pub fn new(server: &str, token: &str, model: &str) -> Self {
        let agent = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(180)))
            .http_status_as_error(false)
            .build()
            .new_agent();
        Self {
            agent,
            url: format!("{}/v1/chat/completions", server.trim_end_matches('/')),
            token: token.to_owned(),
            model: model.to_owned(),
        }
    }

    /// 发一轮对话，从回答里取出第一个 JSON 数组；模型偶尔包一层 ```json 代码块，一并剥掉。
    pub fn ask_array(&self, system: &str, user: &str) -> Result<Vec<Value>, TunerError> {
        let request = json!({
            "model": self.model,
            "temperature": 0.2,
            "messages": [
                {"role": "system", "content": system},
                {"role": "user", "content": user},
            ],
        });
        let mut response = self
            .agent
            .post(&self.url)
            .header("Authorization", format!("Bearer {}", self.token))
            .send_json(&request)
            .map_err(|e| TunerError::Llm(e.to_string()))?;
        let status = response.status().as_u16();
        let body: Value = response
            .body_mut()
            .read_json()
            .map_err(|e| TunerError::Llm(format!("HTTP {status}: {e}")))?;
        if status != 200 {
            return Err(TunerError::Llm(format!("HTTP {status}: {body}")));
        }
        let content = body["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or_default();
        parse_array(content)
            .ok_or_else(|| TunerError::Llm(format!("回答里没有 JSON 数组：{content}")))
    }
}

fn parse_array(content: &str) -> Option<Vec<Value>> {
    let start = content.find('[')?;
    let end = content.rfind(']')?;
    serde_json::from_str::<Vec<Value>>(content.get(start..=end)?).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_array_from_fenced_answer() {
        let answer = "好的：\n```json\n[{\"text\": \"青简\"}]\n```";
        assert_eq!(parse_array(answer).unwrap()[0]["text"], "青简");
        assert!(parse_array("没有").is_none());
    }
}
