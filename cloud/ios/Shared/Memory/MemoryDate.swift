// 记忆里的日期：yyyy-MM-dd，按北京时间（与桥的 LocalDate 一致）。

import Foundation

enum MemoryDate {
    static let timeZone = TimeZone(identifier: "Asia/Shanghai") ?? TimeZone(secondsFromGMT: 8 * 3600)!

    static var calendar: Calendar {
        var calendar = Calendar(identifier: .gregorian)
        calendar.timeZone = timeZone
        return calendar
    }

    static func parse(_ text: String) -> Date? { formatter().date(from: text) }

    static func format(_ date: Date) -> String { formatter().string(from: date) }

    /// 两个时刻之间隔了几个日历日（北京时间）。
    static func daysBetween(_ from: Date, _ to: Date) -> Int {
        calendar.dateComponents([.day], from: calendar.startOfDay(for: from), to: calendar.startOfDay(for: to))
            .day ?? 0
    }

    /// 按年重复的日子：`date` 的月日落在 `now` 当天或之后最近的一次；2 月 29 日在平年算 2 月 28 日（与桥的 `next_anniversary` 一致）。
    static func nextAnniversary(of date: Date, from now: Date) -> Date {
        let cal = calendar
        let parts = cal.dateComponents([.month, .day], from: date)
        let today = cal.startOfDay(for: now)
        let year = cal.component(.year, from: today)
        func occurrence(_ year: Int) -> Date {
            var target = DateComponents(year: year, month: parts.month, day: 1)
            let first = cal.date(from: target) ?? today
            let days = cal.range(of: .day, in: .month, for: first)?.count ?? 28
            target.day = min(parts.day ?? 1, days)
            return cal.date(from: target) ?? today
        }
        let thisYear = occurrence(year)
        return thisYear >= today ? thisYear : occurrence(year + 1)
    }

    /// 从今天到 `text` 那天还有几天，过去的是负数；日期写错时为 nil。
    static func daysUntil(_ text: String, now: Date = Date()) -> Int? {
        parse(text).map { daysBetween(now, $0) }
    }

    /// DateFormatter 不是 Sendable，每次现建。
    private static func formatter() -> DateFormatter {
        let formatter = DateFormatter()
        formatter.locale = Locale(identifier: "en_US_POSIX")
        formatter.timeZone = timeZone
        formatter.dateFormat = "yyyy-MM-dd"
        return formatter
    }
}
