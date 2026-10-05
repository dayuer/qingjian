// 键盘里新建对象：名字的收拾与桥回复的解析。

import XCTest
@testable import QingjianCloud

final class ContactAddTests: XCTestCase {
    func testNameIsTrimmedAndCapped() {
        XCTAssertEqual(ContactAdd.name("  阿杰 "), "阿杰")
        XCTAssertNil(ContactAdd.name("   "), "空名字不能建")
        XCTAssertEqual(ContactAdd.name(String(repeating: "长", count: 20))?.count, ContactAdd.maxNameChars)
    }

    func testParseSuccessAndFailure() {
        XCTAssertEqual(try ContactAdd.parse(#"{"id":"0123456789abcdef0123456789abcdef"}"#).get(), "0123456789abcdef0123456789abcdef")
        guard case .failure(let limit) = ContactAdd.parse(#"{"code":"pin_limit","message":"最多置顶 4 个人"}"#) else {
            return XCTFail("置顶超了应当是失败")
        }
        XCTAssertEqual(limit.code, .pinLimit)
        XCTAssertEqual(limit.userMessage, "最多置顶 4 个人", "照桥给的")
        XCTAssertEqual(MemoryFailure(code: .pinLimit, message: "").userMessage, "最多置顶 4 个人")
        guard case .failure(let busy) = ContactAdd.parse(#"{"code":"lock_timeout","message":"x"}"#) else {
            return XCTFail("锁被占应当是失败")
        }
        XCTAssertEqual(busy.userMessage, "键盘正在写记忆，请稍后再试")
        guard case .failure(let empty) = ContactAdd.parse(nil) else { return XCTFail("没有回复当失败") }
        XCTAssertEqual(empty.code, .other)
    }
}
