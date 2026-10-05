// 关于页文案：版本号取自 Info.plist，GPL 署名与链接、MiSans 署名与随包协议不能丢。

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

    func testFontAttributionAndBundledLicense() throws {
        XCTAssertEqual(AboutInfo.fontAttribution, "本应用使用了 MiSans 字体（© 北京小米移动软件有限公司）")
        let license = try XCTUnwrap(AboutInfo.fontLicenseText(), "包里没有 MiSans-LICENSE.txt")
        XCTAssertTrue(license.contains("MiSans字体知识产权许可协议"))
    }
}
