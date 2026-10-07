// App 里对象详情（02 的 1b）、改一条（1c）、对象设置（1d）的文案，提成纯函数方便单测。App 里照旧用真名，键盘的称呼走 MemoryContact.chipName。

import Foundation

enum MemoryDetailText {
    /// 键盘日子提醒从几天前开始出（今天到 3 天后），与桥的 `REMINDER_DAYS` 一致。
    static let reminderLeadDays = 3

    /// 代号最多几个字，与桥的 `MAX_DISPLAY_NAME_CHARS` 一致。
    static let maxDisplayNameChars = 12

    /// 详情页名字下面的一行：「认识 214 天 · 46 条记忆」。
    static func subtitle(knownDays: Int, cardCount: Int) -> String {
        "认识 \(knownDays) 天 · \(cardCount) 条记忆"
    }

    /// 有日子的卡左列 M.dd 下面的小字：今天、明天、一周内写周几、一个月内写「n 天后 / n 天前」；
    /// 再远就不写（左列只剩月日）。`days` 是离今天几天，过去是负数。
    static func relativeDay(days: Int, target: Date) -> String? {
        switch days {
        case 0: return "今天"
        case 1: return "明天"
        case 2...6: return weekday(target)
        case 7...30: return "\(days) 天后"
        case -30 ... -1: return "\(-days) 天前"
        default: return nil
        }
    }

    /// 改一条里「到哪天」那一行右边的说明，只写真有的行为：日子与约定都在键盘提示行里提前 3 天起提醒（选中这个人时），
    /// 设计稿写的「前一天提醒」与实情不符，照实写（UI 清单约束 7）；这个人关了「日子提醒」时不写。
    static func reminderNote(kind: MemoryCard.Kind, contact: MemoryContact?) -> String? {
        guard kind == .promise || kind == .date, let contact, contact.remindOn else { return nil }
        return "提前 \(reminderLeadDays) 天提醒"
    }

    /// 「到哪天」那一行的日期：「10 月 10 日 周六」。
    static func dayTitle(_ date: Date) -> String {
        let parts = MemoryDate.calendar.dateComponents([.month, .day], from: date)
        return "\(parts.month ?? 0) 月 \(parts.day ?? 0) 日 \(weekday(date))"
    }

    /// 忘掉弹层的标题。
    static func forgetTitle(name: String) -> String { "忘掉\(name)？" }

    /// 忘掉弹层的正文（本机版，开了云服务后改成两边一起删，见 T11）。
    static func forgetBody(knownDays: Int, cardCount: Int) -> String {
        "\(knownDays) 天里的 \(cardCount) 条记忆和学到的说话习惯，会从这台手机上删除，无法恢复。"
    }

    /// 改代号时的收拾：去首尾空白、截到 12 字；空的或和名字一样的当没有（键盘上就显示名字）。
    static func displayName(_ draft: String, name: String) -> String? {
        let trimmed = String(draft.trimmingCharacters(in: .whitespacesAndNewlines).prefix(maxDisplayNameChars))
        return trimmed.isEmpty || trimmed == name ? nil : trimmed
    }

    private static func weekday(_ date: Date) -> String {
        let names = ["周日", "周一", "周二", "周三", "周四", "周五", "周六"]
        return names[MemoryDate.calendar.component(.weekday, from: date) - 1]
    }
}
