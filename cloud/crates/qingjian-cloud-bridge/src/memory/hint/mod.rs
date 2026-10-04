//! 打字时的提示（spec「2A 本地记忆 · 提示」）：按当前对象的卡片建匹配词，拿最近上屏的字加当前首选去碰；
//! 日子与约定在 3 天内的给一条提醒（日子按年重复，约定只提醒一次）。另有对象卡面板挑卡、提醒文案两个自由函数。

mod entry;
mod index;
mod item;
mod reason;

use std::cmp::Reverse;
use std::collections::HashSet;
use std::sync::LazyLock;

use qingjian_cloud_proto::CardKind;

use self::entry::IndexedCard;
use super::{Card, LocalDate, Pronoun, has_date};

pub use self::index::HintIndex;
pub use self::item::Hint;
pub use self::reason::HintReason;

/// 匹配词至少几个字。
const MIN_TERM_CHARS: usize = 2;

/// 同一张卡给出过之后多久不再出（秒）。
const THROTTLE_SECS: i64 = 10 * 60;

/// 日子与约定提前几天提醒（含当天，0–3）。
const REMINDER_DAYS: i64 = 3;

/// 键盘内对象卡面板最多几张。
const PANEL_CARDS: usize = 3;

static STOPWORDS: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    include_str!("../stopwords.txt")
        .lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .flat_map(str::split_whitespace)
        .collect()
});

fn is_term(word: &str) -> bool {
    word.chars().count() >= MIN_TERM_CHARS && !STOPWORDS.contains(word)
}

/// 离 `today` 还有几天：日子按年重复（看今年或明年的那一天），约定按写的那天；别的种类、日期写错都是 `None`。
pub fn days_away(kind: CardKind, when: LocalDate, today: LocalDate) -> Option<i64> {
    match kind {
        CardKind::Date => Some(today.days_until(when.next_anniversary(today))),
        CardKind::Promise => Some(today.days_until(when)),
        _ => None,
    }
}

/// 日子：「今天是她的生日」「明天是…」「3 天后是…」；约定：「今天：看电影」「明天：…」「3 天后：…」。
pub fn reminder_text(
    kind: CardKind,
    days: i64,
    pronoun: Pronoun,
    name: &str,
    text: &str,
) -> String {
    let when = match days {
        0 => "今天".to_owned(),
        1 => "明天".to_owned(),
        n => format!("{n} 天后"),
    };
    if kind == CardKind::Promise {
        return format!("{when}：{text}");
    }
    format!("{when}是{}的{text}", pronoun.label(name))
}

/// 对象卡面板的卡片：3 天内的日子与约定（近的在前），然后是正在提示的那张，其余按最近改动；最多 3 张。
pub fn panel_cards(mut cards: Vec<Card>, today: LocalDate, focus: Option<&str>) -> Vec<Card> {
    cards.sort_by_key(|card| {
        let days = card
            .when
            .as_deref()
            .and_then(LocalDate::parse)
            .filter(|_| has_date(card.kind))
            .and_then(|when| days_away(card.kind, when, today))
            .filter(|days| (0..=REMINDER_DAYS).contains(days));
        let rank = match (days, focus == Some(card.id.as_str())) {
            (Some(days), _) => (0, days),
            (None, true) => (1, 0),
            (None, false) => (2, 0),
        };
        (rank, Reverse(card.touched_at))
    });
    cards.truncate(PANEL_CARDS);
    cards
}
