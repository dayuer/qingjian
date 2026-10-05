// 功勋路的节点规则（设计稿 02 的 2a 的 dv-note，2026-10-05 答的 D3）：
// 认识天数满 100/200/周年、确认卡数满 10/50/100/200 各亮一次；不做连续打卡；
// 已亮的按时间排，浅绿点是「最近的下一站」。

import XCTest
@testable import QingjianCloud

final class MilestonesTests: XCTestCase {
    private let day: Int64 = 86_400

    /// 建在 `daysAgo` 天前的对象；`now` 固定成 2026-10-04。
    private func contact(daysAgo: Int, name: String = "小美") -> MemoryContact {
        MemoryContact(
            id: name, name: name, pronoun: .taF,
            createdAt: now - Int64(daysAgo) * day)
    }

    private var now: Int64 { Int64(MemoryDate.parse("2026-10-04")!.timeIntervalSince1970) }

    /// `count` 张确认过的卡，从建的那天起一天一张，好让「记满 n 张」的日期可预期
    /// （第 10 张落在认识第 10 天，早于「认识 100 天」，与设计稿那张图的先后一致）。
    private func cards(_ count: Int, from created: Int64, confirmed: Bool = true) -> [MemoryCard] {
        (0..<count).map { index in
            MemoryCard(
                id: "c\(index)", kind: .other, text: "第 \(index) 条", keywords: [], when: nil,
                source: "manual", confirmed: confirmed,
                createdAt: created + Int64(index) * day, touchedAt: now)
        }
    }

    private func road(daysAgo: Int, cardCount: Int, confirmed: Bool = true) -> [Milestone] {
        let created = now - Int64(daysAgo) * day
        return Milestones.road(
            contact: contact(daysAgo: daysAgo),
            cards: cards(cardCount, from: created, confirmed: confirmed),
            now: Date(timeIntervalSince1970: TimeInterval(now)))
    }

    func testOriginIsAlwaysFirst() {
        let list = road(daysAgo: 0, cardCount: 0)
        XCTAssertEqual(list.first?.title, "认识")
        XCTAssertTrue(list.first!.earned)
        XCTAssertNil(list.first!.remaining, "原点下面写日期")
    }

    /// 设计稿那张图：认识 214 天、46 张卡，亮四个、下一站是「记满 50 张」。
    func testDesignSample() {
        let list = road(daysAgo: 213, cardCount: 46)  // 建在 213 天前 → 认识第 214 天
        let titles = list.map(\.title)
        XCTAssertEqual(titles, ["认识", "记满 10 张", "认识 100 天", "认识 200 天", "记满 50 张"])
        XCTAssertEqual(list.map(\.earned), [true, true, true, true, false])
        XCTAssertEqual(list.last?.remaining, "还差 4 张")
    }

    func testNoContinuousStreakIsEverCounted() {
        // 规则里只有天数与卡数两种节点，没有「连续打卡」这类节点
        let list = road(daysAgo: 400, cardCount: 200)
        XCTAssertFalse(list.contains { $0.title.contains("连续") })
    }

    func testDayMilestonesAreNotEarnedEarly() {
        let list = road(daysAgo: 99, cardCount: 0)  // 认识第 100 天，刚够
        XCTAssertEqual(list.first { $0.title == "认识 100 天" }?.earned, true)
        let before = road(daysAgo: 98, cardCount: 0)  // 认识第 99 天
        XCTAssertEqual(before.first { $0.title == "认识 100 天" }?.earned, false)
        XCTAssertEqual(before.first { $0.title == "认识 100 天" }?.remaining, "还差 1 天")
    }

    func testUnconfirmedCardsDoNotCount() {
        let list = road(daysAgo: 30, cardCount: 46, confirmed: false)
        XCTAssertFalse(list.contains { $0.title == "记满 10 张" && $0.earned })
    }

    func testNextStopIsTheClosestByProgress() {
        // 46/50 = 0.92 比 214/365 = 0.59 近，所以下一站是「记满 50 张」而不是「认识 365 天」
        let list = road(daysAgo: 213, cardCount: 46)
        XCTAssertEqual(list.last?.title, "记满 50 张")
    }

    func testEverythingEarnedHasNoNextStop() {
        let list = road(daysAgo: 400, cardCount: 200)
        XCTAssertTrue(list.allSatisfy(\.earned), "都达成了就不再有浅绿点")
        XCTAssertTrue(list.allSatisfy { $0.remaining == nil })
    }
}
