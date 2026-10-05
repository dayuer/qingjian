// 键盘上与记忆有关的显示判断，提成纯函数方便单测：提示行在不在、牌子写什么、快速切人列谁、
// 格子里「上次用的时间」、首选候选用不用强调色、「记一笔」出不出。

import Foundation

enum ScopeDisplay {
    /// 草稿卡、冲突屏、对象卡这类占键区的面板打开时，提示行那一行的高度：键盘总高不变（宿主界面不跳），
    /// 记一笔的两个面板沿用打开前那一刻的行高（`held`，确认条比提示行高），对象卡沿用当前行高（`live`）；
    /// 这一行不画，高度让给面板（`total`），候选栏与键区也不往下挪（`inset` 为 0）。别的面板照常：行高就是 `live`。
    static func rowHeights(live: CGFloat, held: CGFloat, noteCardOpen: Bool, contactCardOpen: Bool)
        -> (total: CGFloat, inset: CGFloat)
    {
        if noteCardOpen { return (held, 0) }
        if contactCardOpen { return (live, 0) }
        return (live, live)
    }

    /// 提示行这一行在不在：选了人且有提示，或有记一笔 / 起名字的输入条时才在，没东西就不占行（设计稿）；
    /// 代价是提示出现与消失时键盘高度变 34pt、宿主界面跟着动，用户把常驻的空行当成了 bug，2026-10-04 改定。
    /// 记一笔的草稿卡、冲突屏打开时（`noteCardOpen`）这一行收起：设计稿 1e-2、1e-3
    /// 那两屏顶上直接是牌子那一行，不画提示行，高度让给面板（`rowHeights`）。
    static func hasHintRow(hasContact: Bool, hasHint: Bool, hasNoteBar: Bool, noteCardOpen: Bool = false) -> Bool {
        !noteCardOpen && ((hasContact && hasHint) || hasNoteBar)
    }

    /// 没开完全访问时牌子只剩说明入口（点开是开启路径），不出人名与灰绿。
    static func chipShowsPerson(fullAccess: Bool) -> Bool { fullAccess }

    /// App「我」页的常驻说明：App 读不到键盘拿没拿到完全访问，所以用不分状态的中性写法，开了的人看着也不误会。
    static let fullAccessExplanation = "「完全访问」用于按键震动，以及让键盘读到你在「记得」里写下的人与事。开了也不联网，卡片只在这台手机上。"

    static func chipUsesAccent(fullAccess: Bool) -> Bool { fullAccess }

    /// 牌子：人名，没选人时「不指定」。
    static func chipPerson(_ contactName: String?) -> String { contactName ?? noScopeTitle }

    static let noScopeTitle = "不指定"

    /// 点牌子在工具栏里横着列的人：当前这个人之外的人，再加「不指定」（当前就是不指定时不列）；nil 表示不指定。
    /// 名单平铺后人数不限，这一行不能无限长——按 `ContactOrder` 取前 `quickPickCount` 个，其余回 App 里切。
    static func quickPicks(people: [MemoryContact], current: String?, used: [String: Int64]) -> [String?] {
        let others = ContactOrder.ordered(people, used: used).map(\.id).filter { $0 != current }
        let head = Array(others.prefix(ContactOrder.quickPickCount))
        return current == nil ? head : head + [nil]
    }

    static let needsFullAccessText = "开启完全访问后才能用记忆。开了也不联网，卡片只在这台手机上"

    /// 没开完全访问时牌子上的说明：**改写与记忆都要它**——iOS 键盘扩展没开就没有网络，
    /// 改写要请求服务器，按下去必然失败，所以那颗按钮也并进这个说明入口。
    static let needsFullAccessForRewrite = "改写和记忆都要开完全访问。开了也不会上传你没让它上传的内容。"

    /// 「去开启」在工具栏里展开的路径。键盘扩展打不开系统设置，也不许借响应链打开 App，只能给文字。
    static let fullAccessPath = "设置 → 通用 → 键盘 → 键盘 → 素笺 → 允许完全访问"

    /// 「记一笔」按钮出不出：开了完全访问、选了对象、不在私密输入框（密码、验证码这类）。
    /// 剪贴板没字也出，点了进手写（NoteEntry）；所以私密输入框这道门同时挡住了确认条与手写。
    static func canNote(fullAccess: Bool, privateField: Bool, hasContact: Bool) -> Bool {
        fullAccess && !privateField && hasContact
    }

    /// 改写出不出：技能包在、开了完全访问（没开就没有网络，改写按下去必然失败）、不在私密输入框、有改写器。
    static func canRewrite(
        fullAccess: Bool, privateField: Bool, hasSkills: Bool, hasRewriter: Bool
    ) -> Bool {
        fullAccess && hasSkills && hasRewriter && !privateField
    }

    static func contactSubtitle(knownDays: Int) -> String { "认识 \(knownDays) 天" }

    /// 上次在键盘里选中这个人的时间，按北京时间的日历日算。
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

    /// 首选候选用强调色：选了人时（代表在和这个人说话）；不指定只加粗。
    static func accentFirstCandidate(hasContact: Bool) -> Bool { hasContact }

    /// 对象卡页脚左边：面板只列与今天有关的卡。
    static func cardFooter(count: Int) -> String { "只显示与今天有关的 \(count) 条" }

    /// 键盘扩展没有官方办法打开容器 App，点了只给提示。
    static let allMemoryNotice = "打开素笺 App → 记住的"
}
