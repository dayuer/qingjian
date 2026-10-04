// 键盘上与记忆有关的显示判断，提成纯函数方便单测：提示行在不在、牌子两半写什么、快速切人列谁、选择面板怎么排、
// 格子里「上次用的时间」、首选候选用不用强调色、「记一笔」出不出。

import Foundation

enum ScopeDisplay {
    /// 提示行这一行在不在：有提示（恋爱、日常选了人才有，工作没有）或有记一笔 / 起名字的输入条时才在，没东西就不占行（设计稿）；
    /// 代价是提示出现与消失时键盘高度变 34pt、宿主界面跟着动，用户把常驻的空行当成了 bug，2026-10-04 改定。
    /// 记一笔与起名字三个场景都能用，所以输入条不看场景。
    static func hasHintRow(scene: String, hasContact: Bool, hasHint: Bool, hasNoteBar: Bool) -> Bool {
        (MemoryScope.reminds(scene) && hasContact && hasHint) || hasNoteBar
    }

    /// 牌子左半：场景名。
    static func chipScene(_ scene: String) -> String { MemoryScope.title(of: scene) }

    /// 没开完全访问时素笺就是普通输入法：牌子只剩场景名、整块中性色（点开是说明），不出人名与灰绿。
    static func chipShowsPerson(fullAccess: Bool) -> Bool { fullAccess }

    /// App「我」页的常驻说明：App 读不到键盘拿没拿到完全访问，所以用不分状态的中性写法，开了的人看着也不误会。
    static let fullAccessExplanation = "「完全访问」用于按键震动，以及让键盘读到你在「记得」里写下的人与事。开了也不联网，卡片只在这台手机上。"

    static func chipUsesAccent(scene: String, fullAccess: Bool) -> Bool {
        fullAccess && MemoryScope.usesAccent(scene)
    }

    /// 牌子右半：人名，没选人时「不指定」。
    static func chipPerson(_ contactName: String?) -> String { contactName ?? noScopeTitle }

    static let noScopeTitle = "不指定"

    /// 点牌子右半在工具栏里横着列的：本场景除当前这个人之外的人，再加「不指定」（当前就是不指定时不列）；nil 表示不指定。
    static func quickPicks(people: [MemoryContact], current: String?) -> [String?] {
        let others: [String?] = people.map(\.id).filter { $0 != current }
        return current == nil ? others : others + [nil]
    }

    enum PickerMode: Equatable {
        case picker
        case needsFullAccess
    }

    /// 没开完全访问读不到 App Group 里的记忆，也不让切场景（桥不知道有没有完全访问，这道门在 Swift 侧）。
    static func pickerMode(fullAccess: Bool) -> PickerMode { fullAccess ? .picker : .needsFullAccess }

    static let needsFullAccessText = "开启完全访问后才能用记忆。开了也不联网，卡片只在这台手机上"

    /// 「去开启」在面板里展开的路径。键盘扩展打不开系统设置，也不许借响应链打开 App，只能给文字。
    static let fullAccessPath = "设置 → 通用 → 键盘 → 键盘 → 素笺 → 允许完全访问"

    /// 选择面板的格子：一行放得下（连「不指定」「新对象」不超过 4 格）时照设计稿竖排（头像在上）；
    /// 再多竖排的格子两行就超出键区高度，换成横排的矮格子（最多 10 格三行），免得键区里要滚动（键盘扩展里 SwiftUI 的滚动不可靠）。
    enum CellStyle: Equatable {
        case tall
        case compact
    }

    /// `people` 是这个场景的人数；另有「不指定」「新对象」两格。
    static func cellStyle(people: Int) -> CellStyle { people + 2 > 4 ? .compact : .tall }

    /// 「记一笔」按钮出不出：开了完全访问、选了对象、不在私密输入框（密码、验证码这类）；三个场景都能用。
    /// 剪贴板没字也出，点了进手写（NoteEntry）；所以私密输入框这道门同时挡住了确认条与手写。
    static func canNote(fullAccess: Bool, privateField: Bool, hasContact: Bool) -> Bool {
        fullAccess && !privateField && hasContact
    }

    /// 每个场景最多几个对象（与桥的上限一致）。
    static let maxContacts = SceneGroup.limit

    static func contactSubtitle(knownDays: Int) -> String { "认识 \(knownDays) 天" }

    /// 选择面板格子的副文字：上次在键盘里选中这个人的时间，按北京时间的日历日算。
    static func lastUsed(at seconds: Int64?, now: Date = Date()) -> String {
        guard let seconds else { return "还没用过" }
        let days = MemoryDate.daysBetween(Date(timeIntervalSince1970: TimeInterval(seconds)), now)
        switch days {
        case ..<1: return "今天"
        case 1: return "昨天"
        case 2...6: return "\(days) 天前"
        case 7...13: return "上周"
        case 14...29: return "\(days / 7) 周前"
        case 30...364: return "\(days / 30) 个月前"
        default: return "一年前"
        }
    }

    /// 首选候选用强调色：恋爱、日常选了人时（代表在和这个人说话）；工作与不指定只加粗。
    static func accentFirstCandidate(scene: String, hasContact: Bool) -> Bool {
        MemoryScope.usesAccent(scene) && hasContact
    }

    static func newContactSubtitle(count: Int) -> String { "\(count) / \(maxContacts)" }

    static let noScopeSubtitle = "只用场景"

    /// 对象卡页脚左边：面板只列与今天有关的卡。
    static func cardFooter(count: Int) -> String { "只显示与今天有关的 \(count) 条" }

    /// 键盘扩展没有官方办法打开容器 App，点了只给提示。
    static let allMemoryNotice = "打开素笺 App → 记住的"
}
