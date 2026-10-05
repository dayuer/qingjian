// 待整理快满时（180 条，上限 200）在对象详情顶上提示一次；同一个人提示过就不再提示。
// 提示过谁记在 App 自己的 UserDefaults（只有对象 id，不进 App Group，键盘不需要知道）。

import Foundation

struct MaterialNudge {
    /// 到多少条开始提示（审计定夺 3）。
    static let threshold = 180

    static let key = "materialNudgeShown"

    let defaults: UserDefaults

    init(defaults: UserDefaults = .standard) {
        self.defaults = defaults
    }

    static func text(count: Int) -> String {
        "待整理快满了（\(count) / \(NoteBarText.materialCap)），开通素笺云整理，或者删掉一些"
    }

    /// 到了 180 条、这个人还没提示过才提示。
    static func shouldShow(count: Int, alreadyShown: Bool) -> Bool {
        count >= threshold && !alreadyShown
    }

    /// 该提示就记下「提示过了」并返回文案，否则 nil。对象详情每次读到条数时调，同一个人只会拿到一次。
    func take(contactId: String, count: Int) -> String? {
        var shown = Set(defaults.stringArray(forKey: Self.key) ?? [])
        guard Self.shouldShow(count: count, alreadyShown: shown.contains(contactId)) else { return nil }
        shown.insert(contactId)
        defaults.set(shown.sorted(), forKey: Self.key)
        return Self.text(count: count)
    }
}
