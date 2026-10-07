// 对象详情、改一条、对象设置的文案与胶囊：副标题、相对日子、meta、提醒说明（按真实行为）、日期行、忘掉弹层、代号收拾、种类胶囊。

import XCTest
@testable import QingjianCloud

final class MemoryDetailTextTests: XCTestCase {
    private func day(_ text: String) -> Date { MemoryDate.parse(text)! }

    private func contact(remindOn: Bool = true) -> MemoryContact {
        MemoryContact(id: "a", name: "小美", pronoun: .taF, createdAt: 0, remindOn: remindOn)
    }

    func testSubtitle() {
        XCTAssertEqual(MemoryDetailText.subtitle(knownDays: 214, cardCount: 46), "认识 214 天 · 46 条记忆")
        XCTAssertEqual(MemoryDetailText.subtitle(knownDays: 1, cardCount: 0), "认识 1 天 · 0 条记忆")
    }

    func testRelativeDay() {
        // 2026-10-04 是周日
        XCTAssertEqual(MemoryDetailText.relativeDay(days: 0, target: day("2026-10-04")), "今天")
        XCTAssertEqual(MemoryDetailText.relativeDay(days: 1, target: day("2026-10-05")), "明天")
        XCTAssertEqual(MemoryDetailText.relativeDay(days: 2, target: day("2026-10-06")), "周二")
        XCTAssertEqual(MemoryDetailText.relativeDay(days: 6, target: day("2026-10-10")), "周六")
        XCTAssertEqual(MemoryDetailText.relativeDay(days: 7, target: day("2026-10-11")), "7 天后")
        XCTAssertEqual(MemoryDetailText.relativeDay(days: 30, target: day("2026-11-03")), "30 天后")
        XCTAssertNil(MemoryDetailText.relativeDay(days: 31, target: day("2026-11-04")), "超过 30 天只写月日")
        XCTAssertEqual(MemoryDetailText.relativeDay(days: -1, target: day("2026-10-03")), "1 天前")
        XCTAssertEqual(MemoryDetailText.relativeDay(days: -30, target: day("2026-09-04")), "30 天前")
        XCTAssertNil(MemoryDetailText.relativeDay(days: -31, target: day("2026-09-03")))
    }

    /// 只写真有的行为：键盘提示行对约定提前 3 天起提醒；日子设计稿没画不写；关了提醒的不写。
    func testReminderNoteFollowsRealBehavior() {
        XCTAssertEqual(MemoryDetailText.reminderLeadDays, 3, "与桥的 REMINDER_DAYS 一致")
        XCTAssertEqual(MemoryDetailText.reminderNote(kind: .promise, contact: contact()), "提前 3 天提醒")
        XCTAssertEqual(MemoryDetailText.reminderNote(kind: .date, contact: contact()), "提前 3 天提醒", "日子也照实写")
        XCTAssertNil(MemoryDetailText.reminderNote(kind: .preference, contact: contact()))
        XCTAssertNil(MemoryDetailText.reminderNote(kind: .promise, contact: contact(remindOn: false)))
        XCTAssertNil(MemoryDetailText.reminderNote(kind: .promise, contact: nil))
    }

    func testDayTitle() {
        XCTAssertEqual(MemoryDetailText.dayTitle(day("2026-10-10")), "10 月 10 日 周六")
        XCTAssertEqual(MemoryDetailText.dayTitle(day("2027-01-01")), "1 月 1 日 周五")
    }

    func testForgetSheetTexts() {
        XCTAssertEqual(MemoryDetailText.forgetTitle(name: "小美"), "忘掉小美？")
        XCTAssertEqual(
            MemoryDetailText.forgetBody(knownDays: 214, cardCount: 46),
            "214 天里的 46 条记忆和学到的说话习惯，会从这台手机上删除，无法恢复。")
    }

    func testDisplayNameDraft() {
        XCTAssertNil(MemoryDetailText.displayName("", name: "小美"))
        XCTAssertNil(MemoryDetailText.displayName("  ", name: "小美"))
        XCTAssertNil(MemoryDetailText.displayName(" 小美 ", name: "小美"), "和名字一样当没有")
        XCTAssertEqual(MemoryDetailText.displayName(" 阿美 ", name: "小美"), "阿美")
        XCTAssertEqual(MemoryDetailText.displayName("一二三四五六七八九十一二三", name: "小美"), "一二三四五六七八九十一二")
    }

    func testKindPills() {
        let pills = KindPill.pills(selected: .promise)
        XCTAssertEqual(pills.map(\.title), ["日子", "约定", "喜好", "近况", "其他"])
        XCTAssertEqual(pills.filter(\.selected).map(\.kind), [.promise])
        let on = pills[1]
        XCTAssertTrue(on.filled)
        XCTAssertFalse(on.outlined)
        XCTAssertNil(on.textRole, "选中是黑底白字")
        for off in pills where !off.selected {
            XCTAssertFalse(off.filled)
            XCTAssertTrue(off.outlined)
            XCTAssertEqual(off.textRole, .ink2)
        }
        XCTAssertFalse(pills.contains { $0.textRole?.isAccent == true }, "胶囊全是中性色")
    }
}
