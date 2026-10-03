//! 菜单内容的中间表示：既画成菜单栏图标的 NSMenu，也写成 `menu.txt` 给输入法画「青简 Cloud」子菜单。
//!
//! `menu.txt` 一行一项：`<tag>\t<标题>`；tag 为 `-` 的是不可点的说明行，整行 `---` 是分隔线。
//! 输入法点了某项就往 `commands/` 写一个只含 tag 的文件，本程序每拍取走执行（见 `app.rs`）。

/// 菜单里的一行。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Line {
    /// 不可点的说明（连接状态、学习数据状态、「还没有剪贴板记录」）。
    Text(String),

    /// 可点的项：标题与 tag（非负是剪贴板历史的下标，负数是固定动作）。
    Action(String, isize),

    Separator,
}

/// 写给输入法的文本；标题里的换行与制表符换成空格，免得破坏行格式。
pub fn to_text(lines: &[Line]) -> String {
    let clean = |title: &str| title.replace(['\n', '\r', '\t'], " ");
    let mut text = String::new();
    for line in lines {
        match line {
            Line::Text(title) => text.push_str(&format!("-\t{}\n", clean(title))),
            Line::Action(title, tag) => text.push_str(&format!("{tag}\t{}\n", clean(title))),
            Line::Separator => text.push_str("---\n"),
        }
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_one_item_per_line() {
        let lines = [
            Line::Text("已连接".to_owned()),
            Line::Separator,
            Line::Action("[本机] a\tb\nc".to_owned(), 0),
            Line::Action("暂停同步".to_owned(), -1),
        ];
        assert_eq!(
            to_text(&lines),
            "-\t已连接\n---\n0\t[本机] a b c\n-1\t暂停同步\n"
        );
    }
}
