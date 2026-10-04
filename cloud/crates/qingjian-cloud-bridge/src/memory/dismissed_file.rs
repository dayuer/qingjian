//! `memory/dismissed.json`：键盘「知道了」的记录，只有键盘写，App 不碰。卡片 id → 点的那天（`YYYY-MM-DD`，北京时间）；
//! 当天不再提示，第二天自然失效，读时顺带清掉 30 天前的与已经不在的卡。

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct DismissedFile {
    pub cards: BTreeMap<String, String>,
}
