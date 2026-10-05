// 「记一笔」从确认条往后的那一段（设计稿 01 的 1e-2 草稿卡、1e-3 冲突屏）：原话、草稿里的几项、冲突的那一对。
// 提成纯值类型方便单测（KeyboardModel 不在测试 target 里）；KeyboardModel 持有一份，面板切换仍由它管。
// 原话要留到冲突处理完（选了「不记」「两条都留」「更新为新的」）才清：冲突屏顶上显示的就是它。
//
// **草稿项与冲突现在都是样例**（照设计稿），真正的抽取与冲突比对要等 2C 的云端整理。

struct NoteDraftFlow: Equatable {
    /// 草稿卡与冲突屏顶部那条原话（剪贴板拆出来的几段，空行隔开）。
    private(set) var source = ""

    /// 原话展没展开。
    private(set) var sourceExpanded = false

    /// 草稿卡里的几项。
    private(set) var fields: [DraftField] = []

    /// 冲突屏的那一对；没进冲突屏时为 nil。
    private(set) var conflict: NoteConflict?

    /// 点「记到 X」：带着原话进草稿卡。
    mutating func open(cards: [String]) {
        source = cards.joined(separator: "\n\n")
        sourceExpanded = false
        fields = Self.sampleDraft
        conflict = nil
    }

    mutating func toggleSource() {
        sourceExpanded.toggle()
    }

    mutating func updateField(index: Int, value: String) {
        guard fields.indices.contains(index) else { return }
        fields[index].value = value
    }

    mutating func removeField(index: Int) {
        guard fields.indices.contains(index) else { return }
        fields.remove(at: index)
    }

    /// 「记下 n 条」：草稿项收起，进冲突屏。原话留着（冲突屏要显示）；冲突落在 `contactName` 身上，
    /// 调用方传键盘上的称呼（`MemoryContact.chipName`，代号优先）。
    mutating func save(contactName: String) {
        fields = []
        conflict = Self.sampleConflict(contactName: contactName)
    }

    /// 「不记」（草稿卡或冲突屏上）：全部清掉，什么都不写。
    mutating func discard() {
        self = NoteDraftFlow()
    }

    /// 冲突屏上选完：返回要存成素材的原话（两条怎么合留给 2C 的整理去判），然后全部清掉。
    mutating func resolve(_ decision: ConflictDecision) -> String {
        let text = source
        self = NoteDraftFlow()
        _ = decision
        return text
    }

    /// 样例草稿卡（照设计稿 1e-2 的 `e2Init`）；等 2C 换成真抽取。
    static let sampleDraft = [
        DraftField(label: "计划", value: "去厦门"),
        DraftField(
            label: "时间", value: "11 月", unsure: true, why: "「下个月」按今天（10 月）算的"),
        DraftField(label: "地点", value: "沙坡尾"),
    ]

    /// 样例冲突（照设计稿 1e-3）；等 2C 换成真的冲突比对。人名用传进来的称呼，不写死。
    static func sampleConflict(contactName: String) -> NoteConflict {
        NoteConflict(
            contactName: contactName, oldLabel: "已有 · 9 月 2 日", oldText: "不吃香菜",
            newLabel: "新的 · 今天", newText: "最近爱吃香菜")
    }
}
