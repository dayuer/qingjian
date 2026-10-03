//! 跨设备上文：在请求的消息前面插一条 system 消息，内容是各设备最近上屏的文字。
//! iOS 键盘拿不到多少应用里的上文，这一条对它最有用；Mac 上效果要回放评测过才知道，所以缺省不插。

use serde_json::{Value, json};

/// 在第一条 system 消息之后（没有就在最前面）插入；请求不是预期的形状就原样返回 `false`。
pub fn inject_history(request: &mut Value, history: &str) -> bool {
    if history.trim().is_empty() {
        return false;
    }
    let Some(messages) = request.get_mut("messages").and_then(Value::as_array_mut) else {
        return false;
    };
    let position = messages
        .iter()
        .position(|m| m.get("role").and_then(Value::as_str) == Some("system"))
        .map_or(0, |index| index + 1);
    messages.insert(
        position,
        json!({
            "role": "system",
            "content": format!(
                "用户最近在各台设备上输入过的文字（按时间先后，仅供理解用户的用词与话题，不要复述）：\n{history}"
            ),
        }),
    );
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inserts_after_the_first_system_message() {
        let mut request = json!({
            "model": "m",
            "messages": [{"role": "system", "content": "s"}, {"role": "user", "content": "u"}]
        });
        assert!(inject_history(&mut request, "今天去开会"));
        let roles: Vec<&str> = request["messages"]
            .as_array()
            .unwrap()
            .iter()
            .map(|m| m["role"].as_str().unwrap())
            .collect();
        assert_eq!(roles, ["system", "system", "user"]);
        assert!(
            request["messages"][1]["content"]
                .as_str()
                .unwrap()
                .ends_with("今天去开会")
        );
        assert!(!inject_history(&mut json!({"prompt": "x"}), "y"));
    }
}
