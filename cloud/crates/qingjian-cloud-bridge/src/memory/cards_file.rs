//! `<对象 id>/cards.json` 的外层：修订号加卡片。每写一次修订号加一，App 整份写回时靠它发现键盘这期间改过。
//! 早期写的是光秃秃的卡片数组，读时当修订号 0（见 `store.rs` 的 `parse_cards`）。

use serde::{Deserialize, Serialize};

use super::Card;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CardsFile {
    pub rev: u64,

    pub cards: Vec<Card>,
}
