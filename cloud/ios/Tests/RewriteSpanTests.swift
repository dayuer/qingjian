// 改写选段与应用校验的规则（对着 RewriteSpan 写，不测文案）。
// 审计要求的五种场景：句中带句末标点、光标紧跟句号、换行分段、结尾空白、选中优先。

import XCTest

@testable import QingjianCloud

final class RewriteSpanTests: XCTestCase {
    private func span(_ before: String, _ selection: String? = nil) -> RewriteSpan? {
        RewriteSpan.select(before: before, selection: selection)
    }

    func testSentenceAfterTerminalPunctuation() {
        // 句中的句号是边界：只取它后面的这一句
        let s = span("我们明天去。你来吗")!
        XCTAssertEqual(s.original, "你来吗")
        XCTAssertEqual(s.tail, "")
        XCTAssertTrue(s.applies(before: "我们明天去。你来吗", selection: nil))
    }

    func testCursorRightAfterPeriodKeepsItAsTail() {
        // 光标紧跟「。」：句号进尾巴不发给模型；应用时校验带尾巴，改写结果能上屏、句号还在
        let s = span("这一句。")!
        XCTAssertEqual(s.original, "这一句")
        XCTAssertEqual(s.tail, "。")
        XCTAssertTrue(s.applies(before: "这一句。", selection: nil))
        XCTAssertEqual(s.deleteCount, 4)
        XCTAssertEqual(s.committed("改好的"), "改好的。")
    }

    func testNewlineIsAlsoABoundary() {
        let s = span("第一行\n第二句。")!
        XCTAssertEqual(s.original, "第二句")
        XCTAssertEqual(s.tail, "。")
    }

    func testTrailingSpacesGoToTheTail() {
        let s = span("这一句。 ")!
        XCTAssertEqual(s.original, "这一句")
        XCTAssertEqual(s.tail, "。 ")
        // 光标前的空格没被吃掉：校验按「原文 + 尾巴」对
        XCTAssertTrue(s.applies(before: "上一句。这一句。 ", selection: nil))
        XCTAssertEqual(s.committed("好的"), "好的。 ")
    }

    func testLongParagraphWithoutAnyBoundaryTakesTheWholeLine() {
        // 聊天框里的长段：一个边界都没有就取整段（KeyboardModel 再按 300 字截）
        let s = span("今天下午三点在老地方见")!
        XCTAssertEqual(s.original, "今天下午三点在老地方见")
    }

    func testSelectionWinsOverSentence() {
        let s = span("我们明天去。你来吗", "明天")!
        XCTAssertEqual(s.original, "明天")
        XCTAssertTrue(s.isSelection)
        XCTAssertEqual(s.deleteCount, 0)
        XCTAssertEqual(s.committed("后天"), "后天")
        // 选中变了就放弃
        XCTAssertFalse(s.applies(before: "我们明天去。你来吗", selection: "后天"))
        XCTAssertTrue(s.applies(before: "我们明天去。你来吗", selection: "明天"))
    }

    func testAllWhitespaceYieldsNothing() {
        XCTAssertNil(span("   "))
        XCTAssertNil(span("。！？ "))
    }

    func testCursorOnBlankLineRewritesThePreviousSentence() {
        // 光标停在换行后的空白行：尾巴把换行和空白都收走，改的是上一行的整句
        let s = span("上一句。\n  ")!
        XCTAssertEqual(s.original, "上一句")
        XCTAssertEqual(s.tail, "。\n  ")
        XCTAssertTrue(s.applies(before: "上一句。\n  ", selection: nil))
    }

    func testEditTextBetweenRequestAndApplyRejects() {
        // 等结果的间隙里用户改了字：校验不过，结果被丢弃
        let s = span("你来吗")!
        XCTAssertFalse(s.applies(before: "你来吗吗", selection: nil))
        // 前面多了字没关系，看的是结尾
        XCTAssertTrue(s.applies(before: "我问你来吗", selection: nil))
    }
}
