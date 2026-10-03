//! 设备令牌：`qjc_` + 32 字节随机数的十六进制。库里只存 SHA-256，泄露数据库也拿不到令牌。

use qingjian_cloud_proto::TOKEN_PREFIX;
use sha2::{Digest, Sha256};

use crate::ServerError;

pub fn generate_token() -> Result<String, ServerError> {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).map_err(|e| ServerError::Random(e.to_string()))?;
    Ok(format!("{TOKEN_PREFIX}{}", hex(&bytes)))
}

pub fn hash_token(token: &str) -> String {
    hex(&Sha256::digest(token.as_bytes()))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
