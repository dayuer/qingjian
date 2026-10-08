// 首页「记得」的 7 天（今天起，设计稿 02 的 2a）：每天一行事件。
// 事件来自两处：某个人的日子与约定（`MemoryStore.upcoming(within:)` 已经挡掉关了提醒的人），
// 以及「还没归到人的」卡——它没有日子，一律算在今天（D1：「还没归到人的」作为当天事件出现）。
//
// 纯值，不碰 UI 与存储，好单测（见 Tests/DayEventsTests）。

import Foundation

/// 首页一天里的一条事件。
enum DayEvent: Identifiable, Equatable {
    /// 某个人的日子或约定。
    case person(MemoryUpcoming)

    /// 还没归到人的一条素材（首页「+ 记一条」先记下的原话）。
    case unassigned(MemoryMaterial)

    var id: String {
        switch self {
        case .person(let item): item.card.id
        case .unassigned(let material): material.clientId
        }
    }

    /// 事件行的主文案。
    var title: String {
        switch self {
        case .person(let item): item.title
        case .unassigned(let material): material.text
        }
    }

    /// 事件行的副文案。
    var subtitle: String {
        switch self {
        case .person(let item):
            // 设计稿写细节（「去年送的是香水」）；本地卡只有关键词，没有就空着，不重复行尾的种类
            item.card.subtitle
        case .unassigned:
            "还没归到人 · 和谁？"
        }
    }

    /// 行尾的标记。未归人的是「补上」，要能点（accent）；其余只写种类。
    var tag: String {
        switch self {
        case .person(let item): item.card.kind.title
        case .unassigned: "补上"
        }
    }

    /// 行尾标记是不是可点的动作（未归人的「补上」）。
    var isAction: Bool {
        if case .unassigned = self { return true }
        return false
    }

    /// 头像字：某个人的取末字（MemoryAvatar 同规则）；未归人的是问号。
    var avatarText: String {
        switch self {
        case .person(let item): String(item.contact.name.last ?? "?")
        case .unassigned: "?"
        }
    }

    /// 头像用不用灰绿（未归人的不用：它还不代表谁）。
    var hasPerson: Bool {
        if case .person = self { return true }
        return false
    }

    /// 这条路通往谁；未归人的没有。
    var contact: MemoryContact? {
        switch self {
        case .person(let item): item.contact
        case .unassigned: nil
        }
    }
}

/// 首页日历条里的一天。
struct DaySlot: Identifiable, Equatable {
    /// 那天的零点（北京时间）。同一天只出一个槽，所以拿它当 id。
    let date: Date

    /// 日历条上的星期小字：今天 / 一 / 二 …
    let weekday: String

    /// 日历条上的日期数字。
    let number: Int

    /// 事件区的标题：「今天 · 10 月 4 日」「明天 · 10 月 5 日」「10 月 6 日 周二」。
    let title: String

    let events: [DayEvent]

    var id: Date { date }

    var hasEvents: Bool { !events.isEmpty }
}

enum DayEvents {
    /// 日历条与事件区要的 7 天，今天排在第一个。
    ///
    /// - Parameters:
    ///   - upcoming: `MemoryStore.upcoming(within: 6)` 的结果（按 days 排序）。
    ///   - unassigned: 「还没归到人的」素材，全部落在今天。
    ///   - now: 注入「今天」，好让单测固定。
    static func week(
        upcoming: [MemoryUpcoming], unassigned: [MemoryMaterial] = [], now: Date = Date()
    ) -> [DaySlot] {
        let calendar = MemoryDate.calendar
        let today = calendar.startOfDay(for: now)
        let weekdays = ["周日", "周一", "周二", "周三", "周四", "周五", "周六"]

        return (0..<7).map { offset in
            let date = calendar.date(byAdding: .day, value: offset, to: today) ?? today
            let parts = calendar.dateComponents([.month, .day, .weekday], from: date)

            var events: [DayEvent] = []
            if offset == 0 {
                events.append(contentsOf: unassigned.map(DayEvent.unassigned))
            }
            events.append(contentsOf: upcoming.filter { $0.days == offset }.map(DayEvent.person))

            let weekday = offset == 0 ? "今天" : String(weekdays[(parts.weekday ?? 1) - 1].dropFirst())

            return DaySlot(
                date: date,
                weekday: weekday,
                number: parts.day ?? 0,
                title: title(offset: offset, parts: parts, weekdayName: weekdays[(parts.weekday ?? 1) - 1]),
                events: events)
        }
    }

    /// 「今天 · 10 月 4 日」/「明天 · 10 月 5 日」/「10 月 6 日 周二」。
    private static func title(offset: Int, parts: DateComponents, weekdayName: String) -> String {
        let month = parts.month ?? 0
        let day = parts.day ?? 0
        switch offset {
        case 0: return "今天 · \(month) 月 \(day) 日"
        case 1: return "明天 · \(month) 月 \(day) 日"
        default: return "\(month) 月 \(day) 日 \(weekdayName)"
        }
    }

    /// 顶部小字「10 月 · 第 40 周」。
    ///
    /// 周数按 **ISO 8601**（周一为一周之首、首周至少 4 天）算：设计稿画的是 2026-10-04 为「第 40 周」，
    /// 而系统 locale 缺省给的是 41。国内习惯也是周一起算，这里显式定死，不跟 locale 走。
    static func monthAndWeek(now: Date = Date()) -> String {
        var calendar = MemoryDate.calendar
        calendar.firstWeekday = 2
        calendar.minimumDaysInFirstWeek = 4
        let parts = calendar.dateComponents([.month, .weekOfYear], from: now)
        return "\(parts.month ?? 0) 月 · 第 \(parts.weekOfYear ?? 0) 周"
    }

    /// 「本周 · 9.28 – 10.4」：今天到本周日（今天就是周日时两头是同一天）。
    static func thisWeekRange(now: Date = Date()) -> String {
        let calendar = MemoryDate.calendar
        let today = calendar.startOfDay(for: now)
        // weekday：周日 1 … 周六 7；本周日 = 今天往后 (8 - weekday) 天，今天正好是周日时是 0 天
        let weekday = calendar.component(.weekday, from: today)
        let sunday = calendar.date(byAdding: .day, value: (8 - weekday) % 7, to: today) ?? today
        return "本周 · \(short(today)) – \(short(sunday))"
    }

    private static func short(_ date: Date) -> String {
        let parts = MemoryDate.calendar.dateComponents([.month, .day], from: date)
        return "\(parts.month ?? 0).\(parts.day ?? 0)"
    }
}
