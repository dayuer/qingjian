// 键盘上的称呼 chipName：代号优先、空白当没有，旧文件没有 display_name 也读得进来，写回时没有代号就不写这个字段。

import XCTest
@testable import QingjianCloud

final class ChipNameTests: XCTestCase {
    func testFallsBackToNameWhenDisplayNameMissingOrBlank() {
        XCTAssertEqual(MemoryContact.chipName(displayName: nil, name: "小美"), "小美")
        XCTAssertEqual(MemoryContact.chipName(displayName: "", name: "小美"), "小美")
        XCTAssertEqual(MemoryContact.chipName(displayName: " \u{3000}\n", name: "小美"), "小美")
        XCTAssertEqual(MemoryContact.chipName(displayName: "阿美", name: "小美"), "阿美")
        XCTAssertEqual(MemoryContact.chipName(displayName: " 阿美 ", name: "小美"), "阿美")
    }

    func testOldFileDecodesWithoutDisplayNameAndRoundTrips() throws {
        let old = #"{"id":"a","name":"小美","pronoun":"ta_f","created_at":0}"#
        var contact = try JSONDecoder().decode(MemoryContact.self, from: Data(old.utf8))
        XCTAssertNil(contact.displayName)
        XCTAssertEqual(contact.chipName, "小美")
        XCTAssertFalse(String(decoding: try JSONEncoder().encode(contact), as: UTF8.self).contains("display_name"))

        contact.displayName = "阿美"
        let again = try JSONDecoder().decode(MemoryContact.self, from: try JSONEncoder().encode(contact))
        XCTAssertEqual(again.displayName, "阿美")
        XCTAssertEqual(again.chipName, "阿美")
        XCTAssertEqual(again.name, "小美")
    }
}
