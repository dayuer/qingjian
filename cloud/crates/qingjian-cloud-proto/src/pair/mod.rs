//! 匹配码加设备：旧设备出码、新设备输码申请、旧设备允许、新设备取令牌。匹配码的规范化放这里，服务端与各客户端共用。
//!
//! 匹配码是 8 位 Crockford Base32，展示成 `K7P2-9QXM`；输入时大小写不敏感，`I` / `L` 当 `1`、`O` 当 `0`，连字符与空白忽略。

mod code;
mod decision;
mod error;
mod grant;
mod join;
mod poll;
mod request_info;

pub use code::PairCode;
pub use decision::PairDecision;
pub use error::PairCodeError;
pub use grant::PairJoinGrant;
pub use join::PairJoin;
pub use poll::PairPoll;
pub use request_info::PairRequestInfo;

/// 匹配码的位数（不含展示用的连字符）。
pub const PAIR_CODE_LEN: usize = 8;

/// 把用户输入的匹配码规范成 8 位大写 Crockford Base32（不带连字符）。
pub fn normalize_pair_code(input: &str) -> Result<String, PairCodeError> {
    let mut code = String::with_capacity(PAIR_CODE_LEN);
    let mut count = 0;
    for raw in input.chars() {
        if raw == '-' || raw.is_whitespace() {
            continue;
        }
        let c = match raw.to_ascii_uppercase() {
            'I' | 'L' => '1',
            'O' => '0',
            // U 不在 Crockford 字母表里（避免拼出脏话），连同其他字符一律拒绝。
            c @ ('0'..='9' | 'A'..='H' | 'J' | 'K' | 'M' | 'N' | 'P'..='T' | 'V'..='Z') => c,
            _ => return Err(PairCodeError::Char(raw)),
        };
        count += 1;
        if count <= PAIR_CODE_LEN {
            code.push(c);
        }
    }
    if count != PAIR_CODE_LEN {
        return Err(PairCodeError::Length(count));
    }
    Ok(code)
}

/// 规范后的匹配码加上连字符给人看：`K7P29QXM` → `K7P2-9QXM`。长度不是 8 的原样返回。
pub fn format_pair_code(code: &str) -> String {
    if code.len() != PAIR_CODE_LEN || !code.is_ascii() {
        return code.to_owned();
    }
    let (head, tail) = code.split_at(PAIR_CODE_LEN / 2);
    format!("{head}-{tail}")
}
