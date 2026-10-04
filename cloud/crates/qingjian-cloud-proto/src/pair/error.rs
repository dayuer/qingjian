//! 匹配码规范化失败的原因。

use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum PairCodeError {
    /// 去掉连字符与空白后的字符数不对。
    #[error("pair code must have 8 characters, got {0}")]
    Length(usize),

    /// Crockford Base32 之外的字符（含不用的 `U`），带用户原样输入的那个字符。
    #[error("invalid character {0:?} in pair code")]
    Char(char),
}
