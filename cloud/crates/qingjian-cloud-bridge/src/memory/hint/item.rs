//! 一条提示，C 接口原样转成 `{"card_id","text","reason","more"}`。

use serde::Serialize;

use super::HintReason;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Hint {
    pub card_id: String,

    /// 提示行上显示的字：匹配时是卡片文字，日子提醒时是套好模板的一句。
    pub text: String,

    pub reason: HintReason,

    /// 除了这张，当前对象还有别的卡（提示行右侧「展开」看得到）。
    pub more: bool,
}
