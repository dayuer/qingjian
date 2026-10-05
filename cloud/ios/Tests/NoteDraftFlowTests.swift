// 记一笔的草稿卡（1e-2）→ 冲突屏（1e-3）：原话留到冲突处理完才清，冲突屏的人名用当前对象的 chipName，
// 草稿卡与冲突屏打开时提示行收起。

import XCTest
@testable import QingjianCloud

final class NoteDraftFlowTests: XCTestCase {
    private let clipboard = ["下周三要交季度报告", "周五晚上一起吃饭"]

    private var opened: NoteDraftFlow {
        var flow = NoteDraftFlow()
        flow.open(cards: clipboard)
        return flow
    }

    func testSourceSurvivesIntoTheConflictScreen() {
        var flow = opened
        XCTAssertEqual(flow.source, "下周三要交季度报告\n\n周五晚上一起吃饭")
        XCTAssertFalse(flow.fields.isEmpty)
        flow.save(contactName: "阿青")
        XCTAssertEqual(flow.source, "下周三要交季度报告\n\n周五晚上一起吃饭", "进冲突屏时原话还在（冲突屏顶上要显示）")
        XCTAssertTrue(flow.fields.isEmpty)
        XCTAssertNotNil(flow.conflict)
    }

    func testEveryConflictChoiceClearsTheSourceAfterwards() {
        for decision in [ConflictDecision.keepBoth, .useNew] {
            var flow = opened
            flow.save(contactName: "阿青")
            XCTAssertEqual(flow.resolve(decision), "下周三要交季度报告\n\n周五晚上一起吃饭", "存的是原话，不是样例")
            XCTAssertEqual(flow, NoteDraftFlow(), "选完才清")
        }
        var flow = opened
        flow.save(contactName: "阿青")
        flow.discard()
        XCTAssertEqual(flow, NoteDraftFlow(), "「不记」也清")
    }

    func testConflictNamesTheCurrentContactByChipName() {
        let contact = MemoryContact(
            id: "0123456789abcdef0123456789abcdef", name: "王大明", displayName: "阿明", pronoun: .ta,
            createdAt: 0)
        var flow = opened
        flow.save(contactName: contact.chipName)
        XCTAssertEqual(flow.conflict?.contactName, "阿明", "代号优先")

        var plain = contact
        plain.displayName = nil
        var other = opened
        other.save(contactName: plain.chipName)
        XCTAssertEqual(other.conflict?.contactName, "王大明", "没有代号时用名字")
    }

    func testDraftFieldsEditAndRemove() {
        var flow = opened
        let count = flow.fields.count
        flow.updateField(index: 0, value: "改过的")
        XCTAssertEqual(flow.fields[0].value, "改过的")
        flow.removeField(index: 0)
        flow.removeField(index: 99)
        XCTAssertEqual(flow.fields.count, count - 1)
        flow.toggleSource()
        XCTAssertTrue(flow.sourceExpanded)
    }

    func testHintRowFoldsWhileTheDraftOrConflictCardIsOpen() {
        XCTAssertTrue(ScopeDisplay.hasHintRow(hasContact: true, hasHint: true, hasNoteBar: false, noteCardOpen: false))
        XCTAssertFalse(
            ScopeDisplay.hasHintRow(hasContact: true, hasHint: true, hasNoteBar: false, noteCardOpen: true),
            "设计稿 1e-2 / 1e-3 顶上没有提示行")
        XCTAssertFalse(ScopeDisplay.hasHintRow(hasContact: true, hasHint: false, hasNoteBar: true, noteCardOpen: true))
    }

    func testDraftAndConflictCardsKeepTheKeyboardHeight() {
        // 确认条 49pt 时点「记到」：行不画了，49pt 让给草稿卡，候选栏与键区不下移，键盘总高不变。
        let card = ScopeDisplay.rowHeights(
            live: 0, held: 49, noteCardOpen: true, contactCardOpen: false)
        XCTAssertEqual(card.total, 49)
        XCTAssertEqual(card.inset, 0)
        let contact = ScopeDisplay.rowHeights(
            live: 34, held: 0, noteCardOpen: false, contactCardOpen: true)
        XCTAssertEqual(contact.total, 34)
        XCTAssertEqual(contact.inset, 0)
        let keys = ScopeDisplay.rowHeights(
            live: 34, held: 49, noteCardOpen: false, contactCardOpen: false)
        XCTAssertEqual(keys.total, 34)
        XCTAssertEqual(keys.inset, 34)
    }
}
