// 「记一笔」确认条与 toast 的文字：剪贴板拆成几张时写明条数，免得用户以为只记了第一条；
// 记下后按开没开素笺云说清什么时候整理；记不下（这个人的待整理满了）和补写时没记上的，直接写原因，不静默。

enum NoteBarText {
    static func clipLabel(count: Int) -> String { count > 1 ? "刚复制的 · \(count) 条" : "刚复制的" }

    /// 确认条里只放得下一行：首条原文，换行压成空格。
    static func preview(_ cards: [String]) -> String {
        (cards.first ?? "").replacingOccurrences(of: "\n", with: " ")
    }

    /// 记下后的 toast：开了素笺云「记下了，明早整理」，没开「记下了，开通素笺云后整理」；多条时写明条数。
    static func doneText(count: Int, cloud: Bool) -> String {
        let head = count > 1 ? "记下了 \(count) 条" : "记下了"
        return head + (cloud ? "，明早整理" : "，开通素笺云后整理")
    }

    /// 草稿卡、冲突屏上点「不记」后的 toast（设计稿 1e-2 / 1e-3）：草稿卡那里剪贴板这段已经标成处理过，写明不会再提示。
    static func skipped(fromDraft: Bool) -> String {
        fromDraft ? "没记，剪贴板里的这段不会再提示" : "没记"
    }

    /// 草稿卡里一项都删光了还点「记下」。
    static let nothingToSave = "没有可记的"

    /// 冲突屏选完存好后的 toast（设计稿 1e-3）。
    static func conflictResolved(_ decision: ConflictDecision, newText: String) -> String {
        switch decision {
        case .useNew: "已更新：\(newText)"
        case .keepBoth: "两条都留了，按时间排"
        }
    }

    /// 一个人最多留几条没整理的素材，与桥的 MAX_UNPROCESSED_MATERIALS 一致。
    static let materialCap = 200

    /// 桥报 material_limit（这次的条数加上没整理的超过上限，整次没记）时记一笔条上的话：
    /// 没空位了写「这个人还有 200 条没整理」，还有空位但装不下写清这次几条、剩几个空位。
    static func materialLimit(remaining: Int, needed: Int) -> String {
        guard remaining > 0 else { return "这个人还有 \(materialCap) 条没整理，先去 App 里看看" }
        return "这次有 \(needed) 条，这个人只剩 \(remaining) 个空位，先去 App 里整理"
    }

    /// 键盘出现时提示一次的「没记上」：拿不到锁排队的记一笔，补写时被拒绝了，按原因分开写。
    static func dropped(_ notes: DroppedNotes) -> [String] {
        var lines: [String] = []
        if notes.materialLimit > 0 { lines.append("有 \(notes.materialLimit) 条没记上：这个人的待整理满了") }
        if notes.contactGone > 0 { lines.append("有 \(notes.contactGone) 条没记上：这个人已经被忘掉了") }
        if notes.queueFull > 0 { lines.append("有 \(notes.queueFull) 条没记上：键盘一下子记得太多了") }
        return lines
    }
}
