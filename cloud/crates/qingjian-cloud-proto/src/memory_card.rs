//! 一张记忆卡片。字段缺省与服务端的形状对齐：旧版本或 PUT 以外的场合可能不带某些字段，也可能带客户端不认识的新字段（会被忽略）。

use serde::{Deserialize, Serialize};

use crate::{CardKind, CardSource};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryCard {
    /// 卡片 id；手动卡由客户端生成。
    pub card_id: String,

    /// 属于哪个对象；`null` 是「还没归到人」。
    pub contact_id: Option<String>,

    pub kind: CardKind,

    /// 卡片文字。
    pub text: String,

    /// 关键词，用来在输入时提示；缺省为空。
    #[serde(default)]
    pub keywords: Vec<String>,

    /// 相关日期，格式 `YYYY-MM-DD`；没有为 `null`。
    pub when: Option<String>,

    pub source: CardSource,

    /// 用户确认过；只有确认过的卡才用于输入提示。
    #[serde(default)]
    pub confirmed: bool,

    /// 已淡出（过期不再提示，「全部」里还看得到）。
    #[serde(default)]
    pub faded: bool,

    /// 墓碑：已删除，拉取时带上让各设备跟着删。
    #[serde(default)]
    pub deleted: bool,

    /// 服务端的全局递增序号，拉取靠它补；缺省 0。
    #[serde(default)]
    pub seq: i64,

    /// 最后修改时间，Unix 毫秒（与 `SessionInfo.created_at` 一致），冲突以新者为准；缺省 0。
    #[serde(default)]
    pub updated_at: i64,
}
