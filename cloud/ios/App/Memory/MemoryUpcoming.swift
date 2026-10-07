// 「今天」「本周」里的一条：哪个人的哪张卡、还有几天。

struct MemoryUpcoming: Identifiable, Equatable {
    let contact: MemoryContact

    let card: MemoryCard

    let days: Int

    var id: String { card.id }

    /// 「明天是她的生日」「明天：看电影」。
    var text: String { card.reminderText(days: days, contact: contact) }

    /// 首页事件与本周的标题：日子「小美生日」，约定等只写内容「看电影」（设计稿 2a「答应陪她去看海」：是谁看头像与副标题）。
    var title: String { card.kind == .date ? "\(contact.name)\(card.text)" : card.text }

    /// 首页提醒卡左列：「今天」「明天」「3 天后」。
    var shortDay: String {
        switch days {
        case 0: "今天"
        case 1: "明天"
        default: "\(days) 天后"
        }
    }

    /// 通讯录行尾的短提示（设计稿 02 的 2b）：日子「明天生日」，约定与别的「明天 · 看电影」。
    var rowNote: String {
        card.kind == .date ? "\(shortDay)\(card.text)" : "\(shortDay) · \(card.text)"
    }

    /// 「今天」「明天」「3 天后 · 10.07」（设计稿日期写 M.dd）。
    var dayLabel: String {
        switch days {
        case 0: "今天"
        case 1: "明天"
        default: "\(days) 天后 · \((card.when?.suffix(5) ?? "").replacingOccurrences(of: "-", with: "."))"
        }
    }
}
