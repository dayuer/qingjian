// 首页「记得」的 7 天分桶：日期标题、星期小字、事件落哪天、未归人的卡算今天。
// 用固定日期（2026-10-04 是周日）避免依赖当天。

import XCTest
@testable import QingjianCloud

final class DayEventsTests: XCTestCase {
    private func contact(_ name: String = "小美", scene: String = MemoryScope.dating) -> MemoryContact {
        MemoryContact(id: name, name: name, pronoun: .taF, scene: scene, createdAt: 0)
    }

    private func card(_ id: String, _ kind: MemoryCard.Kind = .other, _ text: String = "x")
        -> MemoryCard
    {
        MemoryCard(
            id: id, kind: kind, text: text, keywords: [], when: nil, source: "manual",
            confirmed: true, createdAt: 0, touchedAt: 0)
    }

    func testWeekIsSevenDaysStartingToday() {
        let now = MemoryDate.parse("2026-10-04")!  // 周日
        let days = DayEvents.week(upcoming: [], now: now)

        XCTAssertEqual(days.count, 7)
        XCTAssertEqual(days[0].weekday, "今天")
        XCTAssertEqual(days[0].number, 4)
        XCTAssertEqual(days[1].weekday, "一")
        XCTAssertEqual(days[1].number, 5)
        XCTAssertEqual(days[6].weekday, "六")
        XCTAssertEqual(days[6].number, 10)
    }

    func testDayTitles() {
        let now = MemoryDate.parse("2026-10-04")!
        let days = DayEvents.week(upcoming: [], now: now)

        XCTAssertEqual(days[0].title, "今天 · 10 月 4 日")
        XCTAssertEqual(days[1].title, "明天 · 10 月 5 日")
        XCTAssertEqual(days[2].title, "10 月 6 日 周二")
        XCTAssertEqual(days[3].title, "10 月 7 日 周三")
    }

    func testUpcomingLandsOnItsDay() {
        let now = MemoryDate.parse("2026-10-04")!
        let item = MemoryUpcoming(
            contact: contact(), card: card("c1", .promise, "看电影"), days: 2)
        let days = DayEvents.week(upcoming: [item], now: now)

        XCTAssertFalse(days[0].hasEvents)
        XCTAssertTrue(days[2].hasEvents)
        XCTAssertEqual(days[2].events.first?.title, "小美 · 看电影", "约定写「人 · 内容」")
        XCTAssertEqual(days[2].events.first?.tag, "约定")
        XCTAssertFalse(days[2].events.first!.isAction, "有主的行不是动作")
    }

    func testUnassignedLandsOnToday() {
        let now = MemoryDate.parse("2026-10-04")!
        let note = card("u1", .other, "她不吃香菜")
        let days = DayEvents.week(upcoming: [], unassigned: [note], now: now)

        XCTAssertTrue(days[0].hasEvents, "未归人的卡算在今天")
        let event = days[0].events[0]
        XCTAssertEqual(event.title, "她不吃香菜")
        XCTAssertEqual(event.subtitle, "还没归到人 · 和谁？")
        XCTAssertEqual(event.tag, "补上")
        XCTAssertTrue(event.isAction, "「补上」要能点")
        XCTAssertFalse(event.hasPerson, "未归人不该用灰绿")
        XCTAssertEqual(event.avatarText, "?")
        XCTAssertNil(event.contact)
    }

    func testDefaultIsNotDotForEmptyDay() {
        let now = MemoryDate.parse("2026-10-04")!
        let days = DayEvents.week(upcoming: [], now: now)
        XCTAssertFalse(days.allSatisfy(\.hasEvents))
        XCTAssertTrue(days.allSatisfy { !$0.hasEvents })
    }

    func testMonthAndWeekHeader() {
        XCTAssertEqual(DayEvents.monthAndWeek(now: MemoryDate.parse("2026-10-04")!), "10 月 · 第 40 周")
    }

    func testThisWeekRangeEndsOnSunday() {
        // 2026-10-04 是周日，本周日就是当天
        XCTAssertEqual(
            DayEvents.thisWeekRange(now: MemoryDate.parse("2026-10-04")!), "本周 · 10.4 – 10.4")
        // 周三（10-07）往后到周日 10-11
        XCTAssertEqual(
            DayEvents.thisWeekRange(now: MemoryDate.parse("2026-10-07")!), "本周 · 10.7 – 10.11")
    }
}
