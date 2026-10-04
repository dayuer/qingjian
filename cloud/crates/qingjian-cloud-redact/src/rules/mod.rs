//! 规则层的各类规则，每类一个文件。顺序在 [`crate::redact_rules`] 里定。

pub mod address;
pub mod bank_card;
pub mod email;
mod hit;
pub mod id_card;
pub mod landline;
pub mod phone;
mod replace;
pub mod url;
