// 场景与分组：每个场景一组人、各场景上次选的人与上次用的时间、切场景的参数、牌子、快速切人、面板格子、首选候选，
// 以及键盘面板的取人顺序（置顶的先、其余按沟通情况）。

import XCTest
@testable import QingjianCloud

final class SceneGroupTests: XCTestCase {
    private func person(_ name: String, _ scene: String) -> MemoryContact {
        MemoryContact.new(name: name, pronoun: .ta, scene: scene)
    }

    private func scene(_ id: String, _ name: String) -> MemoryScene {
        MemoryScene(id: id, name: name, createdAt: 0)
    }

    func testGroupsFollowTheSceneList() {
        let contacts = [person("妈妈", "a"), person("小美", "b"), person("老周", "a")]
        let scenes = [scene("a", "家人"), scene("b", "朋友")]
        let groups = scenes.map { one in
            SceneGroup(
                id: one.id, name: one.name,
                people: SceneGroup.people(in: one.id, from: contacts))
        }
        XCTAssertEqual(groups.map(\.id), ["a", "b"], "顺序照场景列表")
        XCTAssertEqual(groups.map(\.name), ["家人", "朋友"])
        XCTAssertEqual(groups[0].people.map(\.name), ["妈妈", "老周"], "组内保持名单顺序")
        XCTAssertEqual(SceneGroup.people(in: "c", from: contacts), [], "没有这个场景就是空的")
    }

    func testNewContactKeepsTheGivenScene() {
        XCTAssertEqual(person("a", "work").scene, "work")
        XCTAssertEqual(MemoryScene.new(name: "家人").name, "家人")
        XCTAssertEqual(MemoryScene.new(name: "家人").id.count, 32, "id 用与对象同一套随机十六进制")
    }

    func testScopeDecodesLastAndOldFilesWithout() throws {
        let json = #"{"scene":"daily","contact_id":"b","last":{"dating":"a","daily":"b"}}"#
        let scope = try JSONDecoder().decode(MemoryScope.self, from: Data(json.utf8))
        XCTAssertEqual(scope.last, ["dating": "a", "daily": "b"])
        let old = try JSONDecoder().decode(MemoryScope.self, from: Data(#"{"scene":"dating","contact_id":"a"}"#.utf8))
        XCTAssertEqual(old.last, [:], "旧文件没有 last")
    }

    func testScopePickArgument() {
        XCTAssertNil(ScopePick.last.argument, "空指针：回到这个场景上次选的人")
        XCTAssertEqual(ScopePick.nobody.argument, "", "空字符串：明确不指定")
        XCTAssertEqual(ScopePick.contact("a").argument, "a")
    }

    func testQuickPicksListOthersThenNobody() {
        let people = [person("小美", "dating"), person("阿林", "dating")]
        XCTAssertEqual(ScopeDisplay.quickPicks(people: people, current: people[0].id), [people[1].id, nil])
        XCTAssertEqual(
            ScopeDisplay.quickPicks(people: people, current: nil), people.map(\.id), "当前不指定时不再列不指定")
        XCTAssertEqual(ScopeDisplay.quickPicks(people: [], current: nil), [])
    }

    func testCellStyleSwitchesWhenOneRowIsFull() {
        XCTAssertEqual(ScopeDisplay.cellStyle(people: 0), .tall)
        XCTAssertEqual(ScopeDisplay.cellStyle(people: 2), .tall, "2 人加两格正好一行（设计稿 1d）")
        XCTAssertEqual(ScopeDisplay.cellStyle(people: 3), .compact)
        XCTAssertEqual(ScopeDisplay.cellStyle(people: 8), .compact)
    }

    func testScopeDecodesUsed() throws {
        let json = #"{"scene":"daily","used":{"a":1791043200}}"#
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

final class ContactOrderTests: XCTestCase {
    private func person(
        _ name: String, scene: String = "daily", pinnedAt: Int64? = nil, createdAt: Int64 = 0
    ) -> MemoryContact {
        MemoryContact(
            id: name, name: name, pronoun: .ta, scene: scene, pinnedAt: pinnedAt, createdAt: createdAt)
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

    func testPanelKeepsRoomForNobodyAndNewContact() {
        XCTAssertEqual(ContactOrder.panelCount + 2, 10, "键区里 10 格三行")
        XCTAssertLessThan(ContactOrder.quickPickCount, ContactOrder.panelCount)
    }
}
