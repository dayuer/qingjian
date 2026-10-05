// 「记一笔」拆剪贴板：原话原样保留（名字、时间都留着），每条素材不超过 2000 字节，一次复制通常就是一条；
// 超过的按段拼、单段按字节硬切，一个字不丢；最多 10 条。消息全是合成的。

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

    /// 去掉空白后的全部字，核对切前切后一个字不丢。
    private func letters(_ text: String) -> String {
        String(String.UnicodeScalarView(text.unicodeScalars.filter { !CharacterSet.whitespacesAndNewlines.contains($0) }))
    }

    func testWechatMultiCopyIsOnePieceWithNamesAndTimes() {
        let pieces = ClipMessages.split(wechat)
        XCTAssertEqual(pieces.count, 1, "一次复制通常就是一条素材")
        XCTAssertTrue(pieces[0].contains("2026年10月05日 09:34"), "时间留着，交给大模型整理时要用")
        XCTAssertTrue(pieces[0].hasPrefix("阿杰"), "名字留着")
        XCTAssertTrue(pieces[0].hasSuffix("不是做不下来，是做不过来。"), "最后一条不丢")
    }

    func testPlainShortTextIsOnePiece() {
        XCTAssertEqual(ClipMessages.split("  她不吃香菜\r\n"), ["她不吃香菜"])
        XCTAssertEqual(ClipMessages.split(" \n\n "), [])
    }

    func testExactlyTheLimitStaysWhole() {
        let text = String(repeating: "a", count: ClipMessages.maxBytes)
        XCTAssertEqual(ClipMessages.split(text), [text])
        XCTAssertEqual(ClipMessages.split(text + "b").count, 2)
    }

    func testLongTextPacksParagraphsUnderTheByteLimit() {
        // 每段 300 个汉字 = 900 字节：两段加空行 1802 字节装得下一条，三段装不下
        let paragraph = String(repeating: "字", count: 300)
        let text = Array(repeating: paragraph, count: 5).joined(separator: "\n\n")
        let pieces = ClipMessages.split(text)
        XCTAssertEqual(pieces.count, 3)
        XCTAssertTrue(pieces.allSatisfy { $0.utf8.count <= ClipMessages.maxBytes })
        XCTAssertEqual(pieces[0], paragraph + "\n\n" + paragraph, "段之间的空行留着")
        XCTAssertEqual(letters(pieces.joined()), letters(text), "一个字都不丢")
    }

    func testHardCutKeepsCharactersWhole() {
        // 3 字节的汉字与 4 字节的 emoji、带肤色的组合 emoji 混着
        let text = String(repeating: "中😀👍🏽", count: 200)
        let pieces = ClipMessages.split(text)
        XCTAssertGreaterThan(pieces.count, 1)
        XCTAssertTrue(pieces.allSatisfy { $0.utf8.count <= ClipMessages.maxBytes })
        XCTAssertEqual(pieces.joined(), text, "不丢字、不切断字符")
    }

    func testCapsAtTenPieces() {
        let many = (1...15).map { _ in String(repeating: "字", count: 600) }.joined(separator: "\n\n")
        XCTAssertEqual(ClipMessages.split(many).count, ClipMessages.maxPieces)
    }

    func testBarTexts() {
        XCTAssertEqual(NoteBarText.clipLabel(count: 3), "刚复制的 · 3 条")
        XCTAssertEqual(NoteBarText.clipLabel(count: 1), "刚复制的")
        XCTAssertEqual(NoteBarText.doneText(count: 3), "记下了 3 条")
        XCTAssertEqual(NoteBarText.doneText(count: 1), "记下了")
    }
}
