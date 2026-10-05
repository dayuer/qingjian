//! `<对象 id>/cards.json` 的一项：一张记忆卡。种类与来源用 proto 的 `CardKind`、`CardSource`，与 2C 云端下发的卡（proto 的 `MemoryCard`）
//! 同一套 JSON 名。手写卡的上限（文字 200 字、关键词 8 个且每个 2–8 字）与 proto 的常量一致，写入前由 `MemoryStore` 校验。

use qingjian_cloud_proto::{CardKind, CardSource};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Card {
    pub id: String,

    pub kind: CardKind,

    pub text: String,

    /// 用户写的匹配词，最多 8 个、每个 2–8 字；提示还会从 `text` 里切词。
    #[serde(default)]
    pub keywords: Vec<String>,

    /// `YYYY-MM-DD`（北京时间，与 proto 的 `MemoryCard.when` 同格式），只对日子与约定有意义。
    #[serde(default)]
    pub when: Option<String>,

    /// 手写（2A）或云端整理（2C）。
    pub source: CardSource,

    /// 手写的恒为真；云端整理的（2C）等用户确认。2C 起用户改了云端卡的文字时服务端会隐含置真，本地跟着置真（2C 再做）。
    #[serde(default)]
    pub confirmed: bool,

    /// 已淡出：过期不再提示，「全部」里还看得到（与 proto 一致；2A 手写卡恒为假）。
    #[serde(default)]
    pub faded: bool,

    /// 服务端给的序号，每个用户单独递增，拉取靠它补（与 proto 一致）；2A 手写卡没上过云，为 0。
    #[serde(default)]
    pub seq: i64,

    /// 服务端记的最后修改时间，Unix **毫秒**（与 proto 一致）；2A 手写卡没上过云，为 0。本地改动时间看 `touched_at`。
    #[serde(default)]
    pub updated_at: i64,

    /// 建卡时间，Unix 秒。
    pub created_at: i64,

    /// 本地最后一次改这张卡的时间，Unix 秒；App 写回冲突时两边都改了以它新者为准。
    pub touched_at: i64,
}
