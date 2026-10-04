// 每个场景各一组人：分组与人数、各场景上次选的人与上次用的时间、切场景的参数、哪些场景出提醒，以及牌子、快速切人、面板格子、首选候选与工作场景的中性色。

import XCTest
@testable import QingjianCloud

final class SceneGroupTests: XCTestCase {
    private func person(_ name: String, _ scene: String) -> MemoryContact {
        MemoryContact.new(name: name, pronoun: .ta, scene: scene)
    }

    func testGroupsAreDatingDailyWorkWithCounts() {
        let contacts = [person("妈妈", "daily"), person("小美", "dating"), person("老板", "work"), person("老周", "daily")]
        let groups = SceneGroup.all(contacts)
        XCTAssertEqual(groups.map(\.scene), ["dating", "daily", "work"])
        XCTAssertEqual(groups.map(\.title), ["恋爱", "日常", "工作"])
        XCTAssertEqual(groups[1].people.map(\.name), ["妈妈", "老周"], "组内保持名单顺序")
        XCTAssertEqual(groups[1].countLabel, "2 / 8")
        XCTAssertEqual(SceneGroup.all([]).count, 3, "没人的组也在，好从那里加人")
        let full = SceneGroup(scene: "work", people: (0..<8).map { person("人\($0)", "work") })
        XCTAssertTrue(full.isFull)
        XCTAssertEqual(full.fullNote, "工作最多 8 个人")
        XCTAssertEqual(groups[0].header, "恋爱 · 1 / 8")
        XCTAssertFalse(groups[0].isFull)
    }

    func testNewContactKeepsTheGivenScene() {
        XCTAssertEqual(MemoryContact.new(name: "a", pronoun: .ta).scene, "dating", "缺省仍是恋爱")
        XCTAssertEqual(person("a", "work").scene, "work")
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

    func testRemindersAndAccentSkipWork() {
        XCTAssertTrue(MemoryScope.reminds("dating"))
        XCTAssertTrue(MemoryScope.reminds("daily"))
        XCTAssertFalse(MemoryScope.reminds("work"))
        XCTAssertFalse(MemoryScope.usesAccent("work"))
        XCTAssertEqual(MemoryScope.pickerOrder, ["daily", "dating", "work"])
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

    func testWorkSceneUsesNeutralColors() {
        XCTAssertEqual(ColorUsage.chipPerson.role(in: "dating"), .accent)
        XCTAssertEqual(ColorUsage.chipPerson.role(in: "daily"), .accent, "日常的人也代表一个人")
        XCTAssertEqual(ColorUsage.chipPerson.role(in: "work"), .ink2)
        XCTAssertEqual(ColorUsage.chipBackground.role(in: "work"), .neutralSoft)
        XCTAssertEqual(ColorUsage.chipScene.role(in: "dating"), .ink2, "场景那半边任何场景都是中性色")
        XCTAssertEqual(ColorUsage.selectedContactCell.role(in: "work"), .ink2)
        for usage in ColorUsage.allCases {
            XCTAssertFalse(usage.role(in: "work").isAccent, "\(usage) 在工作场景不该用灰绿")
        }
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

    func testFirstCandidateAccentOnlyWithAPersonOutsideWork() {
        XCTAssertTrue(ScopeDisplay.accentFirstCandidate(scene: "dating", hasContact: true))
        XCTAssertTrue(ScopeDisplay.accentFirstCandidate(scene: "daily", hasContact: true))
        XCTAssertFalse(ScopeDisplay.accentFirstCandidate(scene: "daily", hasContact: false), "不指定只加粗")
        XCTAssertFalse(ScopeDisplay.accentFirstCandidate(scene: "work", hasContact: true))
    }
}
