//! 当前对象的提示索引：卡片与匹配词、每张卡上次给出的时间、「知道了」的日子，以及正显示着的那张。
//! 卡片变了用 [`HintIndex::rebuild`] 重建，节流与「知道了」带过去；「知道了」由会话落盘到 `dismissed.json`。

use std::collections::HashMap;

use super::{
    Hint, HintReason, IndexedCard, REMINDER_DAYS, THROTTLE_SECS, days_away, is_term, reminder_text,
};
use crate::memory::{Card, LocalDate, Pronoun};

#[derive(Debug, Default)]
pub struct HintIndex {
    cards: Vec<IndexedCard>,

    /// 卡片 id → 上一次给出的时间（Unix 秒）。
    shown: HashMap<String, i64>,

    /// 卡片 id → 点了「知道了」的那天（北京时间）。
    dismissed: HashMap<String, LocalDate>,

    /// 上一次给出、还在显示的那张：接着命中时不受节流挡。
    current: Option<String>,
}

impl HintIndex {
    /// `segment` 把卡片文字切成词（会话里用引擎的语言模型切）。只收确认过的卡（手写的都确认过）。
    pub fn build(cards: &[Card], segment: impl Fn(&str) -> Vec<String>) -> Self {
        let cards = cards
            .iter()
            .filter(|card| card.confirmed)
            .map(|card| {
                let mut terms: Vec<String> = Vec::new();
                let keywords = card.keywords.iter().map(|k| k.trim().to_owned());
                for term in keywords.chain(segment(&card.text)) {
                    if is_term(&term) && !terms.contains(&term) {
                        terms.push(term);
                    }
                }
                IndexedCard {
                    id: card.id.clone(),
                    text: card.text.clone(),
                    kind: card.kind,
                    when: card.when.as_deref().and_then(LocalDate::parse),
                    touched_at: card.touched_at,
                    terms,
                }
            })
            .collect();
        Self {
            cards,
            ..Self::default()
        }
    }

    /// 卡片变了时重建：还在的卡带上节流时间与正显示的状态；「知道了」整份带过去
    /// （里面也有别的对象的卡，过期与已删的卡由 `MemoryStore::dismissed` 加载时清）。
    pub fn rebuild(&self, cards: &[Card], segment: impl Fn(&str) -> Vec<String>) -> Self {
        let mut next = Self::build(cards, segment);
        let exists = |id: &str| next.cards.iter().any(|card| card.id == id);
        let shown = self
            .shown
            .iter()
            .filter(|(id, _)| exists(id))
            .map(|(id, at)| (id.clone(), *at))
            .collect();
        let current = self.current.clone().filter(|id| exists(id));
        next.shown = shown;
        next.current = current;
        next.dismissed = self.dismissed.clone();
        next
    }

    /// 「知道了」的记录：卡片 id → 点的那天。
    pub fn dismissed(&self) -> &HashMap<String, LocalDate> {
        &self.dismissed
    }

    /// 换上从 `dismissed.json` 读出来的记录。
    pub fn set_dismissed(&mut self, dismissed: HashMap<String, LocalDate>) {
        self.dismissed = dismissed;
    }

    /// `recent` 是最近上屏的字加当前首选。命中词多的、新改过的优先，最多给一条。
    pub fn match_text(&mut self, recent: &str, now: i64) -> Option<Hint> {
        let today = LocalDate::from_unix(now);
        let mut ranked: Vec<(usize, usize)> = self
            .cards
            .iter()
            .enumerate()
            .map(|(i, card)| {
                (
                    i,
                    card.terms
                        .iter()
                        .filter(|t| recent.contains(t.as_str()))
                        .count(),
                )
            })
            .filter(|(_, hits)| *hits > 0)
            .collect();
        ranked.sort_by(|a, b| {
            b.1.cmp(&a.1)
                .then(self.cards[b.0].touched_at.cmp(&self.cards[a.0].touched_at))
        });
        for (i, _) in ranked {
            let card = &self.cards[i];
            if self.dismissed.get(&card.id) == Some(&today) {
                continue;
            }
            let showing = self.current.as_deref() == Some(card.id.as_str());
            let recently = self
                .shown
                .get(&card.id)
                .is_some_and(|&at| now - at < THROTTLE_SECS);
            if !showing && recently {
                continue;
            }
            let hint = Hint {
                card_id: card.id.clone(),
                text: card.text.clone(),
                reason: HintReason::Match,
                more: self.has_other(&card.id),
            };
            self.shown.insert(card.id.clone(), now);
            self.current = Some(card.id.clone());
            return Some(hint);
        }
        self.current = None;
        None
    }

    /// 日子（按年重复）与约定在今天到 3 天后的，挑最近的一条；「知道了」过的当天不出。
    pub fn today(&self, today: LocalDate, pronoun: Pronoun, name: &str) -> Option<Hint> {
        let (days, card) = self
            .cards
            .iter()
            .filter(|card| self.dismissed.get(&card.id) != Some(&today))
            .filter_map(|card| {
                let days = days_away(card.kind, card.when?, today)?;
                (0..=REMINDER_DAYS).contains(&days).then_some((days, card))
            })
            .min_by_key(|(days, card)| (*days, std::cmp::Reverse(card.touched_at)))?;
        Some(Hint {
            card_id: card.id.clone(),
            text: reminder_text(card.kind, days, pronoun, name, &card.text),
            reason: HintReason::Today,
            more: self.has_other(&card.id),
        })
    }

    /// 除了 `card_id` 这张还有别的卡（提示行的「展开」看得到）。
    fn has_other(&self, card_id: &str) -> bool {
        self.cards.iter().any(|card| card.id != card_id)
    }

    /// `today` 为真：当天不再出；为假：从 `now` 起按刚给出过算，10 分钟内不再出。
    pub fn dismiss(&mut self, card_id: &str, today: bool, now: i64) {
        if today {
            self.dismissed
                .insert(card_id.to_owned(), LocalDate::from_unix(now));
        } else {
            self.shown.insert(card_id.to_owned(), now);
        }
        if self.current.as_deref() == Some(card_id) {
            self.current = None;
        }
    }
}
