// 键盘上与记忆有关的显示判断，提成纯函数方便单测：提示行在不在、牌子写什么、选择面板能不能用、「记一笔」出不出。

enum ScopeDisplay {
    /// 恋爱场景选了对象时提示行一直在（没有提示时是空行）；日常、工作与「恋爱 · 不指定」没有这一行。
    static func hasHintRow(scene: String, hasContact: Bool) -> Bool {
        scene == MemoryScope.dating && hasContact
    }

    /// 牌子上的字：恋爱选了对象是「小美 · 恋爱」，其余只是场景名。
    static func chipTitle(scene: String, contactName: String?) -> String {
        guard scene == MemoryScope.dating, let contactName else { return MemoryScope.title(of: scene) }
        return "\(contactName) · 恋爱"
    }

    enum PickerMode: Equatable {
        case picker
        case needsFullAccess
    }

    /// 没开完全访问读不到 App Group 里的记忆，也不让切场景（桥不知道有没有完全访问，这道门在 Swift 侧）。
    static func pickerMode(fullAccess: Bool) -> PickerMode { fullAccess ? .picker : .needsFullAccess }

    static let needsFullAccessText = "开启完全访问后才能使用记忆"

    /// 「记一笔」：开了完全访问、剪贴板有字、选了对象、不在私密输入框。
    static func canNote(fullAccess: Bool, clipboardHasText: Bool, privateField: Bool, hasContact: Bool) -> Bool {
        fullAccess && clipboardHasText && !privateField && hasContact
    }
}
