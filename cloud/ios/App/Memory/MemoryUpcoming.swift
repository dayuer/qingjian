// 「今天」「本周」里的一条：哪个人的哪张卡、还有几天。

struct MemoryUpcoming: Identifiable, Equatable {
    let contact: MemoryContact

    let card: MemoryCard

    let days: Int

    var id: String { card.id }

    /// 「明天是她的生日」「明天：看电影」。
    var text: String { card.reminderText(days: days, contact: contact) }

    /// 首页提醒卡的标题：日子「小美生日」，约定「小美 · 看电影」。
    var title: String { card.kind == .date ? "\(contact.name)\(card.text)" : "\(contact.name) · \(card.text)" }

    /// 首页提醒卡左列：「今天」「明天」「3 天后」。
    var shortDay: String {
        switch days {
        case 0: "今天"
        case 1: "明天"
        default: "\(days) 天后"
        }
    }

    /// 「今天」「明天」「3 天后 · 10-07」。
    var dayLabel: String {
        switch days {
        case 0: "今天"
        case 1: "明天"
        default: "\(days) 天后 · \(card.when?.suffix(5) ?? "")"
        }
    }
}
