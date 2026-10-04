// 关于页文案：版本号取自 Info.plist，GPL 署名与链接不能丢。

import XCTest
@testable import QingjianCloud

final class AboutInfoTests: XCTestCase {
    func testVersionTextFromInfo() {
        XCTAssertEqual(AboutInfo.versionText(info: ["CFBundleShortVersionString": "0.1.0"]), "版本 0.1.0")
    }

    func testVersionTextMissing() {
        XCTAssertEqual(AboutInfo.versionText(info: nil), "")
        XCTAssertEqual(AboutInfo.versionText(info: [:]), "")
        XCTAssertEqual(AboutInfo.versionText(info: ["CFBundleShortVersionString": ""]), "")
    }

    func testAttributionAndLinks() {
        XCTAssertEqual(AboutInfo.attribution, "基于开源的青简输入法（GPL-3.0）")
        XCTAssertEqual(AboutInfo.sourceURL.absoluteString, "https://github.com/dayuer/qingjian")
        XCTAssertEqual(AboutInfo.licenseURL.absoluteString, "https://www.gnu.org/licenses/gpl-3.0.html")
    }
}
