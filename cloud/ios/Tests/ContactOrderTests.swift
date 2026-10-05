// 名单上列人与切人的纯值：牌子与快速切人写什么、列谁（ScopeDisplay），列人的顺序（ContactOrder），
// 以及各人上次用的时间怎么说。ContactOrder 在 App 与键盘上共用。

import XCTest
@testable import QingjianCloud

final class ContactOrderTests: XCTestCase {
    private func person(
        _ name: String, pinnedAt: Int64? = nil, createdAt: Int64 = 0
    ) -> MemoryContact {
        MemoryContact(
            id: name, name: name, pronoun: .ta, pinnedAt: pinnedAt, createdAt: createdAt)
    }

    func testPinnedComeFirstByWhenTheyWerePinned() {
        let people = [
            person("甲", pinnedAt: 30), person("乙"), person("丙", pinnedAt: 10),
        ]
        let ordered = ContactOrder.ordered(people, used: [:])
        XCTAssertEqual(ordered.map(\.name), ["丙", "甲", "乙"], "早置顶的在前，没置顶的在后")
    }

    func testTheRestFollowHowOftenYouTalk() {
        let people = [person("甲"), person("乙"), person("丙")]
        let used: [String: Int64] = ["甲": 100, "丙": 300, "乙": 200]
        XCTAssertEqual(ContactOrder.ordered(people, used: used).map(\.name), ["丙", "乙", "甲"])
    }

    func testNeverUsedSortsByHowLongYouHaveKnownThem() {
        let people = [person("甲", createdAt: 100), person("乙", createdAt: 300), person("丙")]
        XCTAssertEqual(ContactOrder.ordered(people, used: [:]).map(\.name), ["乙", "甲", "丙"])
    }

    func testLimitKeepsTheHead() {
        let people = [person("甲"), person("乙"), person("丙")]
        let used: [String: Int64] = ["甲": 1, "乙": 3, "丙": 2]
        XCTAssertEqual(ContactOrder.ordered(people, used: used, limit: 2).map(\.name), ["乙", "丙"])
        XCTAssertEqual(ContactOrder.ordered(people, used: used, limit: 0).count, 0)
        XCTAssertEqual(ContactOrder.ordered([], used: [:]).count, 0)
    }

    func testScoreFallsBackToWhenYouMet() {
        let one = person("甲", createdAt: 42)
        XCTAssertEqual(ContactOrder.score(one, used: ["甲": 7]), 7)
        XCTAssertEqual(ContactOrder.score(one, used: [:]), 42)
    }

    func testScopePickArgument() {
        XCTAssertNil(ScopePick.keep.argument, "空指针：保持现在选的人不变")
        XCTAssertEqual(ScopePick.nobody.argument, "", "空字符串：明确不指定")
        XCTAssertEqual(ScopePick.contact("a").argument, "a")
    }

    func testQuickPicksListOthersThenNobody() {
        let people = [person("Amy"), person("Bob")]
        XCTAssertEqual(
            ScopeDisplay.quickPicks(people: people, current: people[0].id, used: [:]),
            [people[1].id, nil])
        XCTAssertEqual(
            ScopeDisplay.quickPicks(people: people, current: nil, used: [:]),
            people.map(\.id), "当前不指定时不再列不指定")
        XCTAssertEqual(ScopeDisplay.quickPicks(people: [], current: nil, used: [:]), [])
    }

    /// 名单平铺后人数不限，这一行只取排在前面的几个，别把工具栏撑破。
    func testQuickPicksStopAtTheHeadOfTheList() {
        let people = (0..<ContactOrder.quickPickCount + 3).map { person("第 \($0) 个") }
        XCTAssertEqual(
            ScopeDisplay.quickPicks(people: people, current: nil, used: [:]).count,
            ContactOrder.quickPickCount, "不指定时只列排在前面的几个")
        XCTAssertEqual(
            ScopeDisplay.quickPicks(people: people, current: people[0].id, used: [:]).count,
            ContactOrder.quickPickCount + 1, "选了人时，人之外再多一个「不指定」")
    }

    func testScopeDecodesUsed() throws {
        let json = #"{"used":{"a":1791043200}}"#
        XCTAssertEqual(try JSONDecoder().decode(MemoryScope.self, from: Data(json.utf8)).used, ["a": 1_791_043_200])
    }

    func testLastUsedLabel() throws {
        let now = try XCTUnwrap(MemoryDate.parse("2026-10-05"))
        let day: Int64 = 86400
        let today = Int64(now.timeIntervalSince1970) + 3600
        XCTAssertEqual(ScopeDisplay.lastUsed(at: today, now: now), "今天")
        XCTAssertEqual(ScopeDisplay.lastUsed(at: today - day, now: now), "昨天")
        XCTAssertEqual(ScopeDisplay.lastUsed(at: today - 3 * day, now: now), "3 天前")
        XCTAssertEqual(ScopeDisplay.lastUsed(at: today - 8 * day, now: now), "上周")
        XCTAssertEqual(ScopeDisplay.lastUsed(at: today - 15 * day, now: now), "2 周前")
        XCTAssertEqual(ScopeDisplay.lastUsed(at: today - 65 * day, now: now), "2 个月前")
        XCTAssertEqual(ScopeDisplay.lastUsed(at: nil, now: now), "还没用过")
    }

    func testFirstCandidateAccentOnlyWithAPerson() {
        XCTAssertTrue(ScopeDisplay.accentFirstCandidate(hasContact: true))
        XCTAssertFalse(ScopeDisplay.accentFirstCandidate(hasContact: false), "不指定只加粗")
    }
}
