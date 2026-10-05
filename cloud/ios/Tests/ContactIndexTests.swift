// 通讯录（设计稿 02 的 2b）的分组、排序、搜索、行尾提示与展开区取哪几张。
// 首字母是桥（键盘那侧用词库）算好写进 contacts.json 的，这里只按它分组。

import XCTest
@testable import QingjianCloud

final class ContactIndexTests: XCTestCase {
    private func contact(
        _ name: String, initial: String? = nil, scene: String = MemoryScope.dating, displayName: String? = nil
    ) -> MemoryContact {
        MemoryContact(
            id: name, name: name, displayName: displayName, initial: initial, pronoun: .taF,
            scene: scene, createdAt: 0)
    }

    private func card(_ id: String, _ text: String, kind: MemoryCard.Kind = .other, touchedAt: Int64 = 0)
        -> MemoryCard
    {
        MemoryCard(
            id: id, kind: kind, text: text, keywords: [], when: nil, source: "manual",
            confirmed: true, createdAt: 0, touchedAt: touchedAt)
    }

    // MARK: - 分组

    func testGroupsByInitialWithOtherLast() {
        let contacts = [
            contact("小明", initial: "X"), contact("阿林", initial: "A"),
            contact("陈", initial: "C"), contact("没首字母"),
        ]
        let sections = ContactIndex.sections(contacts)

        XCTAssertEqual(sections.map(\.letter), ["A", "C", "X", "#"], "「#」排最后")
        XCTAssertEqual(sections[0].people.map(\.name), ["阿林"])
        XCTAssertEqual(sections[3].people.map(\.name), ["没首字母"])
    }

    func testGroupIsSortedByNameAndStableOnTies() {
        let contacts = [contact("么", initial: "M"), contact("明", initial: "M"), contact("马", initial: "M")]
        let section = ContactIndex.sections(contacts)[0]

        XCTAssertEqual(section.people.map(\.name).sorted(), section.people.map(\.name).sorted())
        XCTAssertEqual(ContactIndex.sections(contacts), ContactIndex.sections(contacts.reversed()), "同分按 id，顺序稳定")
    }

    func testInitialIsNormalizedToASingleUpperLetter() {
        XCTAssertEqual(ContactIndex.letter(of: contact("甲", initial: "j")), "J", "小写也认")
        XCTAssertEqual(ContactIndex.letter(of: contact("乙", initial: "zhang")), "Z", "多字母取头一个")
        XCTAssertEqual(ContactIndex.letter(of: contact("丙", initial: "1")), "#", "不是字母归「#」")
        XCTAssertEqual(ContactIndex.letter(of: contact("丁", initial: "🐱")), "#")
        XCTAssertEqual(ContactIndex.letter(of: contact("戊", initial: "")), "#")
        XCTAssertEqual(ContactIndex.letter(of: contact("己")), "#", "旧数据没有这个字段")
    }

    func testLettersFollowTheSections() {
        let sections = ContactIndex.sections([contact("小明", initial: "X"), contact("阿林", initial: "A")])
        XCTAssertEqual(ContactIndex.letters(sections), ["A", "X"])
    }

    // MARK: - 搜索

    func testSearchMatchesNameDisplayNameAndScene() {
        let contacts = [
            contact("小美", initial: "X"), contact("阿林", initial: "A", displayName: "林林"),
            contact("老板", initial: "L", scene: MemoryScope.work),
        ]
        XCTAssertEqual(ContactIndex.search(contacts, text: "美").map(\.name), ["小美"])
        XCTAssertEqual(ContactIndex.search(contacts, text: "林林").map(\.name), ["阿林"], "代号也认")
        XCTAssertEqual(ContactIndex.search(contacts, text: "工作").map(\.name), ["老板"], "场景名也认")
        XCTAssertEqual(ContactIndex.search(contacts, text: "小美").map(\.name), ["小美"])
    }

    func testSearchIgnoresCaseAndSurroundingSpace() {
        let contacts = [contact("Alice", initial: "A")]
        XCTAssertEqual(ContactIndex.search(contacts, text: "alice").count, 1)
        XCTAssertEqual(ContactIndex.search(contacts, text: "  ALI  ").count, 1)
    }

    func testEmptySearchKeepsEveryone() {
        let contacts = [contact("小美", initial: "X")]
        XCTAssertEqual(ContactIndex.search(contacts, text: "   ").count, 1)
    }

    func testSearchPromptCountsPeople() {
        XCTAssertEqual(ContactIndex.searchPrompt(count: 7), "搜索 7 个人")
        XCTAssertEqual(ContactIndex.searchPrompt(count: 0), "搜索 0 个人")
    }

    // MARK: - 行尾提示与展开区

    func testNoteIsTheNearestEventPerPerson() {
        let 小美 = contact("小美", initial: "X")
        let 阿林 = contact("阿林", initial: "A")
        let upcoming = [
            MemoryUpcoming(contact: 小美, card: card("a", "看电影", kind: .promise), days: 1),
            MemoryUpcoming(contact: 小美, card: card("b", "生日", kind: .date), days: 4),
            MemoryUpcoming(contact: 阿林, card: card("c", "生日", kind: .date), days: 0),
        ]
        let notes = ContactIndex.notes(upcoming)

        XCTAssertEqual(notes["小美"], "明天 · 看电影", "取最近的那件")
        XCTAssertEqual(notes["阿林"], "今天生日", "日子不加分隔号")
        XCTAssertNil(notes["陈"])
    }

    func testPreviewKeepsTheThreeMostRecentlyTouched() {
        let cards = [
            card("a", "旧的", touchedAt: 1), card("b", "新的", touchedAt: 9),
            card("c", "中间的", touchedAt: 5), card("d", "最新", touchedAt: 20),
        ]
        XCTAssertEqual(ContactIndex.preview(cards).map(\.text), ["最新", "新的", "中间的"])
        XCTAssertEqual(ContactIndex.preview(cards, limit: 1).map(\.text), ["最新"])
        XCTAssertTrue(ContactIndex.preview([]).isEmpty)
    }
}
