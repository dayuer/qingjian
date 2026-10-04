//! `PUT /v1/memory/cards/{card_id}` 的请求体：新建手动卡，或修改卡片；返回新卡。
//! 只序列化设了的字段；PUT 一个不存在的 `card_id` 就是新建手动卡，`source` 由服务端定，
//! 新建时缺 `kind` 或 `text` 由服务端校验后返回 400。
//! 409 的响应体带 `code`：`card_deleted`（这张卡已被删，本地跟着删）与 `card_limit`（提示「卡片太多了，先清理一些」）；
//! 每小时写入超限仍是 429。用户改了云端卡的文字，服务端会隐含把它设为 confirmed，客户端本地也要置 confirmed。

use serde::{Deserialize, Serialize};

use crate::CardKind;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PutCard {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<CardKind>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keywords: Option<Vec<String>>,

    /// 设置相关日期，`YYYY-MM-DD`；要清空日期用 [`PutCard::clear_when`]，二者互斥。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub when: Option<String>,

    /// 清空相关日期；与 `when` 互斥（同时给时以服务端为准）。用一个布尔而不是 `Option<Option<String>>`，
    /// 因为后者需要自定义反序列化才能区分「不给」与「给 null」。
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub clear_when: bool,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confirmed: Option<bool>,

    /// 「补上是谁」：把还没归到人的卡归到这个对象。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contact_id: Option<String>,
}
