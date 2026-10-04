//! 建好索引的一张卡：只留匹配与提醒用得上的字段。

use qingjian_cloud_proto::CardKind;

use crate::memory::LocalDate;

#[derive(Debug)]
pub struct IndexedCard {
    pub id: String,

    pub text: String,

    pub kind: CardKind,

    /// 解析得了的 `when`。
    pub when: Option<LocalDate>,

    pub touched_at: i64,

    /// 匹配词：两字以上、不在停用词表里、去重。
    pub terms: Vec<String>,
}
