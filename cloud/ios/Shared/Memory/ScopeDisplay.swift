// 键盘上与记忆有关的显示判断，提成纯函数方便单测：提示行在不在、牌子写什么、选择面板能不能用、「记一笔」出不出。

enum ScopeDisplay {
    /// 提示行只在「恋爱 · 某人」且这一行有东西（提示、记一笔条、手写条）时出现，没东西就不占行（设计稿）；
    /// 代价是提示出现与消失时键盘高度变 34pt、宿主界面跟着动，用户把常驻的空行当成了 bug，2026-10-04 改定。
    static func hasHintRow(scene: String, hasContact: Bool, hasContent: Bool) -> Bool {
        scene == MemoryScope.dating && hasContact && hasContent
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

    /// 「记一笔」按钮出不出：开了完全访问、选了对象、不在私密输入框（密码、验证码这类）。
    /// 剪贴板没字也出，点了进手写（NoteEntry）；所以私密输入框这道门同时挡住了确认条与手写。
    static func canNote(fullAccess: Bool, privateField: Bool, hasContact: Bool) -> Bool {
        fullAccess && !privateField && hasContact
    }

    /// 恋爱场景最多几个对象（与桥的上限一致）。
    static let maxContacts = 8

    static func contactSubtitle(knownDays: Int) -> String { "认识 \(knownDays) 天" }

    static func newContactSubtitle(count: Int) -> String { "\(count) / \(maxContacts)" }

    static let noScopeSubtitle = "只用场景"

    /// 对象卡页脚左边：面板只列与今天有关的卡。
    static func cardFooter(count: Int) -> String { "只显示与今天有关的 \(count) 条" }

    /// 键盘扩展没有官方办法打开容器 App，点了只给提示。
    static let allMemoryNotice = "打开素笺 App → 记住的"

    static let enableFullAccessNotice = "打开素笺 App，按引导开启完全访问"
}
