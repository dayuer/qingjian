// 每个场景各一组人：分组与人数、各场景上次选的人、切场景的参数、哪些场景出提醒。

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
        XCTAssertEqual(MemoryScope.pickerOrder, ["daily", "dating", "work"])
    }
}
