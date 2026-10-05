//! 「记一笔」的本机素材库（`cloud/docs/plans/2026-10-05-note-materials.md`）：原话原样存成待整理素材，
//! 开了素笺云且同意「记忆」的交给 2B 上传、2C 每日整理成卡；没开的留在本机，开通并同意后补传。
//! 这里放切段、上传前替换对象名字、补传判断这几个纯函数；读写 `materials.jsonl` 在 `file.rs`（`MemoryStore` 的方法）。
//! 素材是原话，日志里只记条数与错误码。单测在 `memory/tests/materials.rs`。

mod cloud_state;
mod consent;
mod file;
mod material;
mod source;
mod upload_decision;

use qingjian_cloud_proto::MAX_MEMORY_TEXT_BYTES;

use super::Contact;

pub use self::cloud_state::CloudState;
pub use self::consent::Consent;
pub use self::material::Material;
pub use self::source::MaterialSource;
pub use self::upload_decision::UploadDecision;

/// 每个对象最多留几条没整理的素材；到了再记直接拒绝（不悄悄丢），App 那边 180 条时先提示。
pub const MAX_UNPROCESSED_MATERIALS: usize = 200;

/// 整理成卡的素材在本机再留多少天（审计定夺 2）。
pub const PROCESSED_KEEP_DAYS: i64 = 30;

/// 上传前替换对象名字用的占位。
pub const CONTACT_PLACEHOLDER: &str = "〔对象〕";

/// 没整理的条数（App 的「待整理 · n 条」与上限都按它）。
pub fn unprocessed(materials: &[Material]) -> usize {
    materials.iter().filter(|m| !m.processed).count()
}

/// 把一次「记一笔」切成每条不超过 [`MAX_MEMORY_TEXT_BYTES`] 字节的素材，一个字不丢：
/// 先按空行分段、把相邻的段用空行拼回去装满一条；单段还超的按字节切，切点落在字符边界上。
/// 换行统一成 `\n`，整体与每段去掉首尾空白；空的返回空表。
pub fn split_note(text: &str) -> Vec<String> {
    let text = text.replace("\r\n", "\n");
    let text = text.trim();
    if text.is_empty() {
        return Vec::new();
    }
    if text.len() <= MAX_MEMORY_TEXT_BYTES {
        return vec![text.to_owned()];
    }
    let mut pieces = Vec::new();
    let mut current = String::new();
    for paragraph in text.split("\n\n").map(str::trim).filter(|p| !p.is_empty()) {
        for piece in cut_at_char_boundaries(paragraph, MAX_MEMORY_TEXT_BYTES) {
            let joined_len = if current.is_empty() {
                piece.len()
            } else {
                current.len() + 2 + piece.len()
            };
            if joined_len <= MAX_MEMORY_TEXT_BYTES {
                if !current.is_empty() {
                    current.push_str("\n\n");
                }
                current.push_str(piece);
            } else {
                pieces.push(std::mem::take(&mut current));
                current.push_str(piece);
            }
        }
    }
    if !current.is_empty() {
        pieces.push(current);
    }
    pieces
}

/// 按字节切成每块不超过 `max` 的几块，切点往前退到字符边界（一个字符最多 4 字节，`max` 远大于它）。
fn cut_at_char_boundaries(text: &str, max: usize) -> Vec<&str> {
    let mut pieces = Vec::new();
    let mut rest = text;
    while rest.len() > max {
        let mut end = max;
        while !rest.is_char_boundary(end) {
            end -= 1;
        }
        pieces.push(&rest[..end]);
        rest = &rest[end..];
    }
    if !rest.is_empty() {
        pieces.push(rest);
    }
    pieces
}

/// 上传前把文本里这个对象的名字与代号（非空的话）都换成 [`CONTACT_PLACEHOLDER`]（审计定夺 1）。
/// 一遍扫过去、每个位置先试长的，免得短名字把长名字的一部分换掉，也不会在刚换上的占位里再换。
/// 其他人名与用户自己的名字不在本机处理，交给服务端 2B 的 deidentify。M3 上传时调用。
pub fn mask_contact_names(text: &str, contact: &Contact) -> String {
    let mut names: Vec<&str> = [Some(contact.name.as_str()), contact.display_name.as_deref()]
        .into_iter()
        .flatten()
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .collect();
    names.sort_by_key(|name| std::cmp::Reverse(name.len()));
    names.dedup();
    if names.is_empty() {
        return text.to_owned();
    }
    let mut masked = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(c) = rest.chars().next() {
        if let Some(name) = names.iter().find(|name| rest.starts_with(**name)) {
            masked.push_str(CONTACT_PLACEHOLDER);
            rest = &rest[name.len()..];
        } else {
            masked.push(c);
            rest = &rest[c.len_utf8()..];
        }
    }
    masked
}

/// 补传要过同意（审计定夺 5）：开通且同意「记忆」、又有没传的素材才传；同意被撤回就停传；其余都先留在本机。
/// M3 接 2B 上传队列时调用。
pub fn should_upload(cloud: CloudState, consent: Consent, has_unuploaded: bool) -> UploadDecision {
    match (cloud, consent) {
        (_, Consent::Withdrawn) => UploadDecision::Stop,
        (CloudState::Activated, Consent::Given) if has_unuploaded => UploadDecision::Upload,
        _ => UploadDecision::Hold,
    }
}
