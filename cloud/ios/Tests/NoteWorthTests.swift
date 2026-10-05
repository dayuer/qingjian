// 「记一笔」要不要为剪贴板出确认条（设计稿 1e）：有时间 / 地点 / 喜好 / 计划这类可记的东西才出。
// 这是本地启发式、不是抽取，所以宁宽勿严——误判成「可记」只是多出一次确认条。

import XCTest
@testable import QingjianCloud

final class NoteWorthTests: XCTestCase {
    func testDesignExampleIsWorthNoting() {
        // 设计稿 1e 的原话：下个月（时间）+ 想去（计划）+ 厦门 / 沙坡尾（地点）
        XCTAssertTrue(NoteWorth.isMemorable("下个月想去厦门，听说沙坡尾很好逛"))
    }

    func testEachKindOfSignalCounts() {
        XCTAssertTrue(NoteWorth.isMemorable("她不吃香菜"), "喜好")
        XCTAssertTrue(NoteWorth.isMemorable("明天是她生日"), "时间")
        XCTAssertTrue(NoteWorth.isMemorable("周六约了看电影"), "时间 + 计划")
        XCTAssertTrue(NoteWorth.isMemorable("喜欢冰美式"), "喜好")
        XCTAssertTrue(NoteWorth.isMemorable("3 月 5 号体检"), "带数字的时间")
        XCTAssertTrue(NoteWorth.isMemorable("周末去她家"), "时间 + 地点")
    }

    func testThingsNobodyWouldNoteAreNotWorthNoting() {
        XCTAssertFalse(NoteWorth.isMemorable("123456"), "验证码")
        XCTAssertFalse(NoteWorth.isMemorable("https://github.com/dayuer/qingjian"), "链接")
        XCTAssertFalse(NoteWorth.isMemorable("abcdefg"), "一串字母")
        XCTAssertFalse(NoteWorth.isMemorable("   "), "空白")
        XCTAssertFalse(NoteWorth.isMemorable(""), "空串")
    }

    /// 判定只看有没有信号，不看到底是什么——所以宁可宽一点，别把要记的挡在外面。
    func testItErrsOnTheGenerousSide() {
        XCTAssertTrue(NoteWorth.isMemorable("我想到一个办法"), "「想」也算计划信号，宽一点没关系")
    }
}
