//! 规则层的各类规则，每类一个文件。顺序在 [`crate::redact_rules`] 里定。

pub mod account;
pub mod address;
pub mod bank_card;
pub mod credential;
pub mod email;
mod hit;
pub mod id_card;
mod keyword;
pub mod keyword_bank;
pub mod keyword_id;
pub mod landline;
pub mod phone;
mod replace;
pub mod url;
