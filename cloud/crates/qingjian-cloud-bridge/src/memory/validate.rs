//! 写盘前的校验：名单、置顶上限、卡片内容。卡片不合格时报错要带上是谁的哪张卡（[`MemoryError::InvalidCard`]），
//! 不然用户在别处（比如改场景名）碰上了，只看到一句「每个关键词要 2 到 8 个字」，不知道去改哪张。

use std::collections::HashSet;

use qingjian_cloud_proto::{MAX_CARD_KEYWORDS, MAX_CARD_TEXT_CHARS};

use super::{Card, Contact, LocalDate, MAX_DISPLAY_NAME_CHARS, MAX_PINNED, MemoryError, Scene};
use crate::scope::is_contact_id;

/// 一个关键词至少、至多几个字（spec 对云端卡关键词的校验，手写卡一并按它）。
const MIN_KEYWORD_CHARS: usize = 2;

const MAX_KEYWORD_CHARS: usize = 8;

/// 报错里引卡片开头几个字。
const CARD_PREVIEW_CHARS: usize = 8;

/// 一个场景里最多 [`MAX_PINNED`] 个置顶。
pub(super) fn check_pinned(contacts: &[Contact], scenes: &[Scene]) -> Result<(), MemoryError> {
    for scene in scenes {
        let pinned = contacts
            .iter()
            .filter(|c| c.scene == scene.id && c.pinned_at.is_some())
            .count();
        if pinned > MAX_PINNED {
            return Err(MemoryError::PinLimit);
        }
    }
    Ok(())
}

/// 人所在的场景不在名册上（旧版本的键盘还写着 `dating` 之类）就归到默认场景（列表里第一个），不报错：
/// 新旧版本并存时旧键盘写回的老场景 id 不该让整份写失败。换了分组，置顶不带过去。
pub(super) fn adopt_unknown_scenes(contacts: &mut [Contact], scenes: &[Scene]) {
    let Some(fallback) = scenes.first().map(|scene| scene.id.clone()) else {
        return;
    };
    for contact in contacts {
        if !scenes.iter().any(|scene| scene.id == contact.scene) {
            contact.scene.clone_from(&fallback);
            contact.pinned_at = None;
        }
    }
}

pub(super) fn validate_contacts(contacts: &[Contact]) -> Result<(), MemoryError> {
    let mut seen = HashSet::new();
    for contact in contacts {
        if !is_contact_id(&contact.id) {
            return Err(MemoryError::Invalid("对象编号不对"));
        }
        if contact.name.trim().is_empty() {
            return Err(MemoryError::Invalid("名字不能是空的"));
        }
        if contact
            .display_name
            .as_deref()
            .is_some_and(|name| name.trim().chars().count() > MAX_DISPLAY_NAME_CHARS)
        {
            return Err(MemoryError::Invalid("键盘上的代号最多 12 个字"));
        }
        if !seen.insert(contact.id.as_str()) {
            return Err(MemoryError::Invalid("同一个人出现了两次"));
        }
    }
    Ok(())
}

/// `owner` 是这些卡的主人：报错里写他的名字（App 里显示的那个）与卡片开头几个字。
pub(super) fn validate_cards(owner: &Contact, cards: &[Card]) -> Result<(), MemoryError> {
    let mut seen = HashSet::new();
    for card in cards {
        let duplicate = !seen.insert(card.id.as_str());
        if let Some(reason) = card_problem(card, duplicate) {
            return Err(MemoryError::InvalidCard {
                contact: owner.name.trim().to_owned(),
                card: card_preview(&card.text),
                reason,
            });
        }
    }
    Ok(())
}

/// 一张卡哪里不合格；合格时为 `None`。
fn card_problem(card: &Card, duplicate: bool) -> Option<&'static str> {
    if !is_contact_id(&card.id) {
        return Some("卡片编号不对");
    }
    if card.text.trim().is_empty() {
        return Some("卡片内容不能是空的");
    }
    if card.text.chars().count() > MAX_CARD_TEXT_CHARS {
        return Some("一张卡最多 200 个字");
    }
    if card.keywords.len() > MAX_CARD_KEYWORDS {
        return Some("关键词最多 8 个");
    }
    if card.keywords.iter().any(|keyword| {
        !(MIN_KEYWORD_CHARS..=MAX_KEYWORD_CHARS).contains(&keyword.trim().chars().count())
    }) {
        return Some("每个关键词要 2 到 8 个字");
    }
    if card
        .when
        .as_deref()
        .is_some_and(|when| LocalDate::parse(when).is_none())
    {
        return Some("日期要写成 2026-10-04 这样");
    }
    if duplicate {
        return Some("同一张卡片出现了两次");
    }
    None
}

/// 卡片开头几个字，超出的用「…」收尾；空白卡给「（空白）」。
fn card_preview(text: &str) -> String {
    let text = text.trim();
    if text.is_empty() {
        return "（空白）".to_owned();
    }
    let mut preview: String = text.chars().take(CARD_PREVIEW_CHARS).collect();
    if text.chars().count() > CARD_PREVIEW_CHARS {
        preview.push('…');
    }
    preview
}
