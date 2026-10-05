// 用户输入的匹配码怎么算：去掉空白与连字符、转大写；8 位才算输完。

import XCTest
@testable import QingjianCloud

final class MatchCodeTests: XCTestCase {
    func testWhitespaceAndHyphensAreDroppedAndLettersGoUpper() {
        XCTAssertEqual(MatchCode.entered(" k7p2-9qxm "), "K7P29QXM")
        XCTAssertEqual(MatchCode.entered("K7P2 9QXM"), "K7P29QXM")
    }

    func testOnlyEightCharactersCountAsComplete() {
        XCTAssertFalse(MatchCode.isComplete("K7P29QX"))
        XCTAssertTrue(MatchCode.isComplete("K7P29QXM"))
        XCTAssertFalse(MatchCode.isComplete(""))
        // 连字符与空白不算位数
        XCTAssertTrue(MatchCode.isComplete("K7P2-9QXM"))
    }

    func testTheRemainingCountTellsHowManyKeysAreMissing() {
        XCTAssertEqual(MatchCode.remaining("K7P2-9QXM"), 0)
        XCTAssertEqual(MatchCode.remaining("K7P2-9"), 3)
        XCTAssertEqual(MatchCode.remaining(""), 8)
    }

    func testAnythingLongerIsNotSilentlyTrimmed() {
        XCTAssertEqual(MatchCode.entered("K7P2-9QXM123"), "K7P29QXM123")
        XCTAssertFalse(MatchCode.isComplete("K7P2-9QXM123"))
    }
}
