//! 菜单内容的中间表示：输入法照它画「素笺云 ›」子菜单，点了某项就把 tag 传回 [`crate::perform`]。

/// 菜单里的一行。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Line {
    /// 不可点的说明（连接状态、学习数据状态、「还没有剪贴板记录」）。
    Text(String),

    /// 可点的项：标题与 tag（非负是剪贴板历史的下标，负数是固定动作）。
    Action(String, isize),

    Separator,
}
