// 「记一笔」拆剪贴板：微信多选复制一条一张、去掉名字与时间；长文按段拼成每张不超过 200 字；最多 10 张。消息全是合成的。

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

    func testWechatMultiCopySplitsIntoOneCardPerMessage() {
        let cards = ClipMessages.split(wechat)
        XCTAssertEqual(cards.count, 3)
        XCTAssertEqual(cards[0], "这一两个月业务还可以，一点点来，但还是要把事情做扎实。")
        XCTAssertTrue(cards[1].hasPrefix("你周围如果有类似的客户"))
        XCTAssertTrue(cards[1].hasSuffix("2. 陪练"), "一条消息里的空行与列表留着")
        XCTAssertEqual(cards[2], "不是做不下来，是做不过来。")
        XCTAssertFalse(cards.joined().contains("2026年10月05日"), "时间行去掉")
        XCTAssertFalse(cards.joined().contains("阿杰\n"), "名字行去掉")
    }

    func testTimestampFormats() {
        XCTAssertTrue(ClipMessages.isTimestamp("2026年10月05日 09:34"))
        XCTAssertTrue(ClipMessages.isTimestamp("2026/10/5 9:34:12"))
        XCTAssertFalse(ClipMessages.isTimestamp("下午 3 点见"))
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
        let many = (1...15).map { "阿杰\n2026年10月05日 09:\(String(format: "%02d", $0))\n第 \($0) 条" }.joined(separator: "\n\n")
        XCTAssertEqual(ClipMessages.split(many).count, ClipMessages.maxCards)
    }

    func testBarTexts() {
        XCTAssertEqual(NoteBarText.clipLabel(count: 3), "刚复制的 · 3 条")
        XCTAssertEqual(NoteBarText.clipLabel(count: 1), "刚复制的")
        XCTAssertEqual(NoteBarText.doneText(count: 3), "记下了 3 条")
        XCTAssertEqual(NoteBarText.doneText(count: 1), "记下了")
    }
}
