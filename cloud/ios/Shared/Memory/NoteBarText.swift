// 「记一笔」确认条与 toast 的文字：剪贴板拆成几张时写明条数，免得用户以为只记了第一条。

enum NoteBarText {
    static func clipLabel(count: Int) -> String { count > 1 ? "刚复制的 · \(count) 条" : "刚复制的" }

    /// 确认条里只放得下一行：首条原文，换行压成空格。
    static func preview(_ cards: [String]) -> String {
        (cards.first ?? "").replacingOccurrences(of: "\n", with: " ")
    }

    static func doneText(count: Int) -> String { count > 1 ? "记下了 \(count) 条" : "记下了" }
}
