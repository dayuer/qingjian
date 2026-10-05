// 「记一笔」拆剪贴板：原话原样保留（名字、时间都留着），长文按段拼成每张不超过 200 字、一个字不丢；最多 10 张。消息全是合成的。

import XCTest
@testable import QingjianCloud

final class ClipMessagesTests: XCTestCase {
    private let wechat = """
    阿杰
    2026年10月05日 09:34
    这一两个月业务还可以，一点点来，但还是要把事情做扎实。


    阿杰
    2026年10月05日 09:34
    你周围如果有类似的客户，可以介绍给我。

    我们有一套完整的方案：

    1. 录音与分析
    2. 陪练


    阿杰
    2026年10月05日 09:35
    不是做不下来，是做不过来。
    """

    func testWechatMultiCopyKeepsNamesAndTimes() {
        let cards = ClipMessages.split(wechat)
        let joined = cards.joined(separator: "\n")
        XCTAssertTrue(cards.allSatisfy { MemoryLimits.count($0) <= MemoryLimits.maxTextChars })
        XCTAssertTrue(joined.contains("2026年10月05日 09:34"), "时间留着，交给大模型整理时要用")
        XCTAssertTrue(joined.contains("阿杰"), "名字留着")
        XCTAssertTrue(joined.contains("不是做不下来，是做不过来。"), "最后一条不丢")
    }

    func testPlainShortTextIsOneCard() {
        XCTAssertEqual(ClipMessages.split("她不吃香菜"), ["她不吃香菜"])
    }

    func testLongPlainTextPacksParagraphsUnderTheLimit() {
        let paragraph = String(repeating: "字", count: 120)
        let cards = ClipMessages.split([paragraph, paragraph, paragraph].joined(separator: "\n\n"))
        XCTAssertEqual(cards.count, 3)
        XCTAssertTrue(cards.allSatisfy { MemoryLimits.count($0) <= MemoryLimits.maxTextChars })
        XCTAssertEqual(cards.joined().count, 360, "一个字都不丢")
    }

    func testCapsAtTenCards() {
        let many = (1...15).map { _ in String(repeating: "字", count: 190) }.joined(separator: "\n\n")
        XCTAssertEqual(ClipMessages.split(many).count, ClipMessages.maxCards)
    }

    func testBarTexts() {
        XCTAssertEqual(NoteBarText.clipLabel(count: 3), "刚复制的 · 3 条")
        XCTAssertEqual(NoteBarText.clipLabel(count: 1), "刚复制的")
        XCTAssertEqual(NoteBarText.doneText(count: 3), "记下了 3 条")
        XCTAssertEqual(NoteBarText.doneText(count: 1), "记下了")
    }
}
