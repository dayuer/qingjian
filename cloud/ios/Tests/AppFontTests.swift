// App 字体：包里有 MiSans 且能按 PostScript 名取到、键盘扩展里没有字体；取不到时回退系统字体；UIKit 用的字重轴取值。

import SwiftUI
import UIKit
import XCTest
@testable import QingjianCloud

final class AppFontTests: XCTestCase {
    func testBundleRegistersMiSans() throws {
        XCTAssertNotNil(UIFont(name: "MiSansVF", size: 17), "Bundle.main 里取不到 MiSansVF：构建前没跑 scripts/fetch-fonts.sh？")
        XCTAssertTrue(AppFont.isAvailable)
        let fonts = try XCTUnwrap(Bundle.main.object(forInfoDictionaryKey: "UIAppFonts") as? [String])
        XCTAssertEqual(fonts, ["MiSansVF.ttf"])
        XCTAssertNotNil(Bundle.main.url(forResource: "MiSansVF", withExtension: "ttf"))
    }

    func testKeyboardExtensionHasNoFont() throws {
        let plugins = try XCTUnwrap(Bundle.main.builtInPlugInsURL)
        let keyboard = plugins.appendingPathComponent("Keyboard.appex")
        XCTAssertTrue(FileManager.default.fileExists(atPath: keyboard.path), "包里没有键盘扩展：\(keyboard.path)")
        let files = FileManager.default.enumerator(at: keyboard, includingPropertiesForKeys: nil)?
            .compactMap { ($0 as? URL)?.lastPathComponent } ?? []
        XCTAssertEqual(files.filter { $0.hasSuffix(".ttf") || $0.hasSuffix(".otf") }, [])
    }

    func testUsesMiSansWhenAvailable() {
        XCTAssertEqual(AppFont.font(size: 15, weight: .medium, available: true), .custom("MiSans VF", fixedSize: 15).weight(.medium))
        XCTAssertEqual(
            AppFont.font(size: 13, relativeTo: .footnote, weight: .regular, available: true),
            .custom("MiSans VF", size: 13, relativeTo: .footnote).weight(.regular)
        )
    }

    func testFallsBackToSystemWhenMissing() {
        XCTAssertEqual(AppFont.font(size: 15, weight: .medium, available: false), .system(size: 15, weight: .medium))
        XCTAssertEqual(AppFont.font(size: 13, relativeTo: .footnote, weight: .semibold, available: false), .system(.footnote).weight(.semibold))
    }

    func testUIFontPicksWeightInstance() {
        let regular = AppFont.uiFont(size: 17)
        let bold = AppFont.uiFont(size: 17, weight: .bold)
        XCTAssertTrue(regular.fontName.hasPrefix("MiSansVF"), regular.fontName)
        let variation = UIFontDescriptor.AttributeName(rawValue: kCTFontVariationAttribute as String)
        let boldAxes = bold.fontDescriptor.object(forKey: variation) as? [NSNumber: NSNumber]
        XCTAssertEqual(boldAxes?[NSNumber(value: AppFont.weightAxis)]?.doubleValue, 700)
    }

    func testAxisValues() {
        XCTAssertEqual(AppFont.axisValue(.regular), 400)
        XCTAssertEqual(AppFont.axisValue(.medium), 500)
        XCTAssertEqual(AppFont.axisValue(.semibold), 600)
        XCTAssertEqual(AppFont.axisValue(.bold), 700)
        XCTAssertEqual(AppFont.axisValue(.black), 700)
        XCTAssertEqual(AppFont.axisValue(.ultraLight), 150)
    }
}
